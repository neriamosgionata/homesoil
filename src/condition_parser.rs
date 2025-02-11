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

const OPERATORS: [Operator; 8] = [
    EQUAL,
    NOT_EQUAL,
    LESS,
    LESS_OR_EQUAL,
    GREATER,
    GREATER_OR_EQUAL,
    IN,
    NOT_IN,
];

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

#[derive(Clone)]
pub struct Expression {
    left: Value,
    operator: Operator,
    right: Value,
}

impl Expression {
    fn set_left(&mut self, left: Value) {
        self.left = left;
    }

    fn set_operator(&mut self, operator: Operator) {
        self.operator = operator;
    }

    fn set_right(&mut self, right: Value) {
        self.right = right;
    }

    fn evaluate(&self) -> bool {
        match self.operator {
            EQUAL => self.left == self.right,
            NOT_EQUAL => self.left != self.right,
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
                    if self.left == *right_value {
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
                    if self.left == *right_value {
                        return false;
                    }
                }

                true
            }
            _ => false,
        }
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

pub fn parse_condition(mut condition_component_vec: Args, variables: &Variables) -> Condition {
    let mut before_operator = true;
    let mut sub_condition = None;

    let mut expression = Expression {
        left: Value::String("".to_string()),
        operator: EQUAL,
        right: Value::String("".to_string()),
    };

    let mut final_condition = Condition::Expression(Expression {
        left: Value::String("".to_string()),
        operator: EQUAL,
        right: Value::String("".to_string()),
    });

    while !condition_component_vec.is_empty() {
        let component = condition_component_vec.remove(0);

        if component.to_string(variables) == PARENTHESIS_OPEN {
            let last_close_parentesis_index = condition_component_vec
                .iter()
                .rposition(|x| x.to_string(variables) == PARENTHESIS_CLOSE)
                .unwrap();
            let sub_condition_component_vec = condition_component_vec
                .drain(0..last_close_parentesis_index)
                .collect::<Args>();
            sub_condition = Some(parse_condition(sub_condition_component_vec, variables));
            continue;
        }

        if component.to_string(variables) == AND {
            if before_operator && sub_condition.is_some() {
                final_condition =
                    Condition::And(vec![sub_condition.clone().unwrap(), final_condition]);
            } else if !before_operator && sub_condition.is_some() {
                final_condition =
                    Condition::And(vec![final_condition, sub_condition.clone().unwrap()]);
            } else if sub_condition.is_none() {
                final_condition = Condition::And(vec![final_condition]);
            } else {
                final_condition = sub_condition.clone().unwrap();
            }

            before_operator = false;
            continue;
        }

        if component.to_string(variables) == OR {
            if before_operator && sub_condition.is_some() {
                final_condition =
                    Condition::Or(vec![sub_condition.clone().unwrap(), final_condition]);
            } else if !before_operator && sub_condition.is_some() {
                final_condition =
                    Condition::Or(vec![final_condition, sub_condition.clone().unwrap()]);
            } else if sub_condition.is_none() {
                final_condition = Condition::Or(vec![final_condition]);
            } else {
                final_condition = sub_condition.clone().unwrap();
            }
            before_operator = false;
            continue;
        }

        if OPERATORS.contains(&component.to_string(variables).as_str()) {
            expression.set_operator(match_operator(&component).unwrap());
            before_operator = false;
            continue;
        }

        if before_operator {
            expression.set_left(component);
            continue;
        }

        expression.set_right(component);
    }

    final_condition
}

