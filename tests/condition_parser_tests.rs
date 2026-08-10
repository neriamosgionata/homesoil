use homesoil::condition_parser::parse_condition;
use homesoil::script_parser::{Value, Variables};
use std::collections::HashMap;

fn empty_vars() -> Variables {
    HashMap::new()
}

fn make_args(tokens: Vec<&str>) -> Vec<Value> {
    tokens
        .into_iter()
        .map(|t| {
            if t.starts_with('$') {
                Value::Variable(t.to_string())
            } else if t == "true" {
                Value::Boolean(true)
            } else if t == "false" {
                Value::Boolean(false)
            } else if t.contains('.') {
                if let Ok(f) = t.parse::<f64>() {
                    Value::Float64(f)
                } else {
                    Value::String(t.to_string())
                }
            } else if let Ok(i) = t.parse::<i32>() {
                Value::Int32(i)
            } else {
                Value::String(t.to_string())
            }
        })
        .collect()
}

#[test]
fn test_equal_ints() {
    let args = make_args(vec!["5", "==", "5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_not_equal_ints() {
    let args = make_args(vec!["5", "!=", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_equal_ints_false() {
    let args = make_args(vec!["5", "==", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_less_than() {
    let args = make_args(vec!["3", "<", "5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_less_than_false() {
    let args = make_args(vec!["5", "<", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_greater_than() {
    let args = make_args(vec!["5", ">", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_less_or_equal() {
    let args = make_args(vec!["5", "<=", "5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());

    let args2 = make_args(vec!["4", "<=", "5"]);
    let cond2 = parse_condition(args2, &empty_vars());
    assert!(cond2.evaluate());
}

#[test]
fn test_greater_or_equal() {
    let args = make_args(vec!["5", ">=", "5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());

    let args2 = make_args(vec!["6", ">=", "5"]);
    let cond2 = parse_condition(args2, &empty_vars());
    assert!(cond2.evaluate());
}

#[test]
fn test_equal_strings() {
    let args = vec![
        Value::String("hello".to_string()),
        Value::String("==".to_string()),
        Value::String("hello".to_string()),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_not_equal_strings() {
    let args = vec![
        Value::String("hello".to_string()),
        Value::String("!=".to_string()),
        Value::String("world".to_string()),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_equal_floats() {
    let args = make_args(vec!["3.14", "==", "3.14"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_less_than_floats() {
    let args = make_args(vec!["2.5", "<", "3.5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_variable_comparison() {
    let mut vars: Variables = HashMap::new();
    vars.insert("$x".to_string(), Value::Int32(10));
    vars.insert("$y".to_string(), Value::Int32(20));

    let args = vec![
        Value::Variable("$x".to_string()),
        Value::String(">".to_string()),
        Value::Variable("$y".to_string()),
    ];
    let cond = parse_condition(args, &vars);
    assert!(!cond.evaluate());
}

#[test]
fn test_in_operator() {
    let args = vec![
        Value::Int32(3),
        Value::String("in".to_string()),
        Value::Array(vec![Value::Int32(1), Value::Int32(2), Value::Int32(3)]),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_in_operator_not_found() {
    let args = vec![
        Value::Int32(5),
        Value::String("in".to_string()),
        Value::Array(vec![Value::Int32(1), Value::Int32(2), Value::Int32(3)]),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_not_in_operator() {
    let args = vec![
        Value::Int32(5),
        Value::String("not in".to_string()),
        Value::Array(vec![Value::Int32(1), Value::Int32(2), Value::Int32(3)]),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_not_in_operator_found() {
    let args = vec![
        Value::Int32(2),
        Value::String("not in".to_string()),
        Value::Array(vec![Value::Int32(1), Value::Int32(2), Value::Int32(3)]),
    ];
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_boolean_comparison() {
    let args = make_args(vec!["true", "==", "true"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_boolean_not_equal() {
    let args = make_args(vec!["true", "!=", "false"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_and_conditions_both_true() {
    let args = make_args(vec!["1", "==", "1", "&&", "2", "==", "2"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_and_conditions_left_false() {
    // Regression: the old parser dropped everything before the last clause
    let args = make_args(vec!["1", "==", "2", "&&", "1", "==", "1"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_and_conditions_right_false() {
    let args = make_args(vec!["1", "==", "1", "&&", "2", "==", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_or_conditions() {
    let args = make_args(vec!["1", "==", "2", "||", "2", "==", "2"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_or_conditions_all_false() {
    let args = make_args(vec!["1", "==", "2", "||", "2", "==", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_and_or_mixed() {
    let args = make_args(vec!["1", "==", "2", "||", "1", "==", "1", "&&", "2", "==", "2"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_parenthesized_condition() {
    let args = make_args(vec!["(", "1", "==", "1", "||", "1", "==", "2", ")", "&&", "3", "==", "3"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_parenthesized_false_branch() {
    // Regression: only the first parenthesized clause used to survive
    let args = make_args(vec!["(", "1", "==", "2", ")", "&&", "(", "3", "==", "3", ")"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(!cond.evaluate());
}

#[test]
fn test_compound_with_variables() {
    let mut vars: Variables = HashMap::new();
    vars.insert("$a".to_string(), Value::Int32(10));
    vars.insert("$b".to_string(), Value::Int32(20));

    let args = make_args(vec!["$a", "==", "10", "&&", "$b", "==", "20"]);
    let cond = parse_condition(args, &vars);
    assert!(cond.evaluate());

    let args = make_args(vec!["$a", "==", "10", "&&", "$b", "==", "30"]);
    let cond = parse_condition(args, &vars);
    assert!(!cond.evaluate());
}

#[test]
fn test_numeric_cross_type_equality() {
    // 1 (Int32) must equal 1.0 (Float64)
    let args = make_args(vec!["1", "==", "1.0"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());

    let args = make_args(vec!["2", "<", "10.5"]);
    let cond = parse_condition(args, &empty_vars());
    assert!(cond.evaluate());
}

#[test]
fn test_unbalanced_parenthesis_does_not_panic() {
    let args = make_args(vec!["(", "1", "==", "1"]);
    let cond = parse_condition(args, &empty_vars());
    let _ = cond.evaluate();
}

#[test]
fn test_empty_condition_does_not_panic() {
    let cond = parse_condition(vec![], &empty_vars());
    assert!(cond.evaluate());
}
