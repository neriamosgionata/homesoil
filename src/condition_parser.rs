type Operator = &'static str;

use crate::script_parser::{Args, Value, Variables};
use anyhow::{anyhow, Result};

const PARENTHESIS_OPEN: Operator = "(";
const PARENTHESIS_CLOSE: Operator = ")";

const AND: Operator = "&&";
const OR: Operator = "||";

const EQUAL: Operator = "==";
const NOT_EQUAL: Operator = "!=";
const LESS: Operator = "<";
const LESS_OR_EQUAL: Operator = "<=";
const GREATER: Operator = ">";
const GREATER_OR_EQUAL: Operator = ">=";
const IN: Operator = "in";
const NOT_IN: Operator = "not in";

fn match_operator(operator: &Value) -> Result<Operator> {
    match operator {
        Value::String(other_equal) => match other_equal.as_str() {
            EQUAL => Ok(EQUAL),
            NOT_EQUAL => Ok(NOT_EQUAL),
            LESS => Ok(LESS),
            LESS_OR_EQUAL => Ok(LESS_OR_EQUAL),
            GREATER => Ok(GREATER),
            GREATER_OR_EQUAL => Ok(GREATER_OR_EQUAL),
            IN => Ok(IN),
            NOT_IN => Ok(NOT_IN),
            _ => Err(anyhow!("Invalid operator")),
        },
        _ => Err(anyhow!("Invalid operator")),
    }
}

/// Resolve a variable reference to its stored value (bounded depth, no panics).
fn resolve_value(value: &Value, variables: &Variables, depth: usize) -> Value {
    if depth > 16 {
        return Value::None;
    }

    match value {
        Value::Variable(name) => match variables.get(name) {
            Some(v) => resolve_value(v, variables, depth + 1),
            None => Value::None,
        },
        other => other.clone(),
    }
}

/// Numeric-aware equality: `1` equals `1.0` regardless of the parsed variant.
fn values_equal(a: &Value, b: &Value) -> bool {
    if let (Some(na), Some(nb)) = (a.as_number(), b.as_number()) {
        return (na - nb).abs() < f64::EPSILON;
    }
    a == b
}

impl Value {
    fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int32(n) => Some(*n as f64),
            Value::Int64(n) => Some(*n as f64),
            Value::Float32(n) => Some(*n as f64),
            Value::Float64(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Expression {
    left: Value,
    operator: Operator,
    right: Value,
}

impl Expression {
    fn evaluate(&self) -> bool {
        // Numeric-aware comparison when both sides are numbers, regardless of parsed variant
        if let (Some(na), Some(nb)) = (self.left.as_number(), self.right.as_number()) {
            return match self.operator {
                EQUAL => (na - nb).abs() < f64::EPSILON,
                NOT_EQUAL => (na - nb).abs() >= f64::EPSILON,
                LESS => na < nb,
                LESS_OR_EQUAL => na <= nb,
                GREATER => na > nb,
                GREATER_OR_EQUAL => na >= nb,
                _ => false,
            };
        }

        match self.operator {
            EQUAL => values_equal(&self.left, &self.right),
            NOT_EQUAL => !values_equal(&self.left, &self.right),
            LESS => self.left < self.right,
            LESS_OR_EQUAL => self.left <= self.right,
            GREATER => self.left > self.right,
            GREATER_OR_EQUAL => self.left >= self.right,
            IN => {
                let right_vec = match &self.right {
                    Value::Array(right_vec) => right_vec,
                    _ => return false,
                };

                for right_value in right_vec {
                    if values_equal(&self.left, right_value) {
                        return true;
                    }
                }

                false
            }
            NOT_IN => {
                let right_vec = match &self.right {
                    Value::Array(right_vec) => right_vec,
                    _ => return false,
                };

                for right_value in right_vec {
                    if values_equal(&self.left, right_value) {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
    }
}

fn empty_expression() -> Expression {
    Expression {
        left: Value::String("".to_string()),
        operator: EQUAL,
        right: Value::String("".to_string()),
    }
}

#[derive(Clone)]
pub enum Condition {
    Expression(Expression),
    And(Vec<Condition>),
    Or(Vec<Condition>),
}

impl Condition {
    pub fn evaluate(&self) -> bool {
        match self {
            Condition::Expression(expression) => expression.evaluate(),
            Condition::And(conditions) => {
                for condition in conditions {
                    if !condition.evaluate() {
                        return false;
                    }
                }

                true
            }
            Condition::Or(conditions) => {
                for condition in conditions {
                    if condition.evaluate() {
                        return true;
                    }
                }

                false
            }
        }
    }
}

/// Recursive-descent parser: `IF` condition components with `&&`, `||`, and parentheses.
/// Variables are resolved eagerly so comparisons work on their values.
pub fn parse_condition(components: Args, variables: &Variables) -> Condition {
    let mut remaining = components;
    parse_or(&mut remaining, variables)
}

fn parse_or(components: &mut Args, variables: &Variables) -> Condition {
    let mut left = parse_and(components, variables);

    loop {
        let is_or = matches!(
            components.first(),
            Some(Value::String(s)) if s == OR
        );
        if !is_or {
            break;
        }

        components.remove(0);
        let right = parse_and(components, variables);
        left = Condition::Or(vec![left, right]);
    }

    left
}

fn parse_and(components: &mut Args, variables: &Variables) -> Condition {
    let mut left = parse_atom(components, variables);

    loop {
        let is_and = matches!(
            components.first(),
            Some(Value::String(s)) if s == AND
        );
        if !is_and {
            break;
        }

        components.remove(0);
        let right = parse_atom(components, variables);
        left = Condition::And(vec![left, right]);
    }

    left
}

fn parse_atom(components: &mut Args, variables: &Variables) -> Condition {
    if components.is_empty() {
        return Condition::Expression(empty_expression());
    }

    // Parenthesized sub-condition
    if matches!(components.first(), Some(Value::String(s)) if s == PARENTHESIS_OPEN) {
        components.remove(0);
        let inner = parse_or(components, variables);

        if matches!(components.first(), Some(Value::String(s)) if s == PARENTHESIS_CLOSE) {
            components.remove(0);
        }

        return inner;
    }

    // Simple expression: left [operator] right
    let left = resolve_value(&components.remove(0), variables, 0);

    let is_not_in = matches!(
        components.first(),
        Some(Value::String(s)) if s == "not"
    ) && matches!(
        components.get(1),
        Some(Value::String(s)) if s == IN
    );

    let operator = if is_not_in {
        components.remove(0);
        components.remove(0);
        NOT_IN
    } else {
        match components.first() {
            Some(op_value) => match match_operator(op_value) {
                Ok(op) => {
                    components.remove(0);
                    op
                }
                Err(_) => EQUAL,
            },
            None => EQUAL,
        }
    };

    let right = if components.is_empty() {
        Value::None
    } else {
        resolve_value(&components.remove(0), variables, 0)
    };

    Condition::Expression(Expression {
        left,
        operator,
        right,
    })
}
