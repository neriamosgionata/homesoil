use homesoil::flow_engine::*;
use serde_json::json;
use std::collections::HashMap;

// ───────────────────────────────────────────────
// Helper builders
// ───────────────────────────────────────────────

fn make_node(id: &str, node_type: &str, data: serde_json::Value) -> FlowNode {
    FlowNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        data,
        position: None,
    }
}

fn make_edge(
    id: &str,
    source: &str,
    source_handle: Option<&str>,
    target: &str,
    target_handle: Option<&str>,
) -> FlowEdge {
    FlowEdge {
        id: id.to_string(),
        source: source.to_string(),
        source_handle: source_handle.map(|s| s.to_string()),
        target: target.to_string(),
        target_handle: target_handle.map(|s| s.to_string()),
    }
}

// ───────────────────────────────────────────────
// FlowGraph serialization / deserialization
// ───────────────────────────────────────────────

#[test]
fn test_flow_graph_roundtrip_serde() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("c1", "constant", json!({"value": 30.0})),
            make_node("cmp1", "comparison", json!({"operator": ">"})),
            make_node("a1", "actuator_output", json!({"actuator_id": 5, "pulse": false})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp1", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp1", Some("b")),
            make_edge("e3", "cmp1", Some("output"), "a1", Some("trigger")),
        ],
    };

    let serialized = serde_json::to_string(&graph).unwrap();
    let deserialized: FlowGraph = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.nodes.len(), 4);
    assert_eq!(deserialized.edges.len(), 3);
    assert_eq!(deserialized.nodes[0].node_type, "sensor_input");
    assert_eq!(deserialized.edges[0].source, "s1");
}

#[test]
fn test_flow_graph_from_xyflow_json() {
    let raw = json!({
        "nodes": [
            {"id": "n1", "type": "sensor_input", "data": {"sensor_id": 42}, "position": {"x": 100, "y": 200}},
            {"id": "n2", "type": "constant", "data": {"value": 25.5}}
        ],
        "edges": [
            {"id": "e1", "source": "n1", "sourceHandle": "value", "target": "n2", "targetHandle": "input"}
        ]
    });

    let graph: FlowGraph = serde_json::from_value(raw).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.nodes[0].id, "n1");
    assert_eq!(
        graph.nodes[0].data.get("sensor_id").unwrap().as_i64(),
        Some(42)
    );
    assert!(graph.nodes[0].position.is_some());
    assert!(graph.nodes[1].position.is_none());
}

#[test]
fn test_flow_graph_empty() {
    let graph: FlowGraph = serde_json::from_str(r#"{"nodes":[],"edges":[]}"#).unwrap();
    assert!(graph.nodes.is_empty());
    assert!(graph.edges.is_empty());
}

// ───────────────────────────────────────────────
// Topological sort
// ───────────────────────────────────────────────

#[test]
fn test_topological_sort_linear_chain() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("a", "sensor_input", json!({})),
            make_node("b", "comparison", json!({})),
            make_node("c", "actuator_output", json!({})),
        ],
        edges: vec![
            make_edge("e1", "a", None, "b", None),
            make_edge("e2", "b", None, "c", None),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 3);

    let pos_a = sorted.iter().position(|x| x == "a").unwrap();
    let pos_b = sorted.iter().position(|x| x == "b").unwrap();
    let pos_c = sorted.iter().position(|x| x == "c").unwrap();

    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn test_topological_sort_diamond() {
    // a -> b, a -> c, b -> d, c -> d
    let graph = FlowGraph {
        nodes: vec![
            make_node("a", "sensor_input", json!({})),
            make_node("b", "comparison", json!({})),
            make_node("c", "constant", json!({})),
            make_node("d", "actuator_output", json!({})),
        ],
        edges: vec![
            make_edge("e1", "a", None, "b", None),
            make_edge("e2", "a", None, "c", None),
            make_edge("e3", "b", None, "d", None),
            make_edge("e4", "c", None, "d", None),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 4);

    let pos_a = sorted.iter().position(|x| x == "a").unwrap();
    let pos_b = sorted.iter().position(|x| x == "b").unwrap();
    let pos_c = sorted.iter().position(|x| x == "c").unwrap();
    let pos_d = sorted.iter().position(|x| x == "d").unwrap();

    assert!(pos_a < pos_b);
    assert!(pos_a < pos_c);
    assert!(pos_b < pos_d);
    assert!(pos_c < pos_d);
}

#[test]
fn test_topological_sort_cycle_returns_none() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("a", "sensor_input", json!({})),
            make_node("b", "comparison", json!({})),
        ],
        edges: vec![
            make_edge("e1", "a", None, "b", None),
            make_edge("e2", "b", None, "a", None),
        ],
    };

    assert!(topological_sort(&graph).is_none());
}

#[test]
fn test_topological_sort_disconnected_nodes() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("a", "sensor_input", json!({})),
            make_node("b", "constant", json!({})),
            make_node("c", "comparison", json!({})),
        ],
        edges: vec![],
    };

    let sorted = topological_sort(&graph).unwrap();
    assert_eq!(sorted.len(), 3);
}

#[test]
fn test_topological_sort_single_node() {
    let graph = FlowGraph {
        nodes: vec![make_node("only", "constant", json!({}))],
        edges: vec![],
    };

    let sorted = topological_sort(&graph).unwrap();
    assert_eq!(sorted, vec!["only"]);
}

#[test]
fn test_topological_sort_empty_graph() {
    let graph = FlowGraph {
        nodes: vec![],
        edges: vec![],
    };

    let sorted = topological_sort(&graph).unwrap();
    assert!(sorted.is_empty());
}

// ───────────────────────────────────────────────
// evaluate_node — sensor_input
// ───────────────────────────────────────────────

#[test]
fn test_evaluate_sensor_input_trigger_match() {
    let node = make_node("s1", "sensor_input", json!({"sensor_id": 7}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 7, 42.5);

    let val = outputs.get("value").unwrap().as_number();
    assert!((val - 42.5).abs() < f64::EPSILON);
}

#[test]
#[ignore = "requires DATABASE_URL to be set"]
fn test_evaluate_sensor_input_no_match_returns_zero() {
    // When sensor_id doesn't match trigger, it would call get_latest_sensor_value
    // which needs DB. Without DB it panics.
    let node = make_node("s1", "sensor_input", json!({"sensor_id": 99}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 7, 42.5);

    let val = outputs.get("value").unwrap().as_number();
    assert!((val - 0.0).abs() < f64::EPSILON);
}

// ───────────────────────────────────────────────
// evaluate_node — constant
// ───────────────────────────────────────────────

#[test]
fn test_evaluate_constant() {
    let node = make_node("c1", "constant", json!({"value": 99.9}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - 99.9).abs() < f64::EPSILON);
}

#[test]
fn test_evaluate_constant_default_zero() {
    let node = make_node("c1", "constant", json!({}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_evaluate_constant_negative() {
    let node = make_node("c1", "constant", json!({"value": -15.3}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - (-15.3)).abs() < f64::EPSILON);
}

// ───────────────────────────────────────────────
// evaluate_node — comparison
// ───────────────────────────────────────────────

fn comparison_inputs(a: f64, b: f64) -> HashMap<String, NodeValue> {
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Number(a));
    inputs.insert("b".to_string(), NodeValue::Number(b));
    inputs
}

#[test]
fn test_comparison_greater_than_true() {
    let node = make_node("cmp", "comparison", json!({"operator": ">"}));
    let outputs = evaluate_node(&node, &comparison_inputs(31.0, 30.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_greater_than_false() {
    let node = make_node("cmp", "comparison", json!({"operator": ">"}));
    let outputs = evaluate_node(&node, &comparison_inputs(29.0, 30.0), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_greater_than_equal_false() {
    let node = make_node("cmp", "comparison", json!({"operator": ">"}));
    let outputs = evaluate_node(&node, &comparison_inputs(30.0, 30.0), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_less_than() {
    let node = make_node("cmp", "comparison", json!({"operator": "<"}));
    let outputs = evaluate_node(&node, &comparison_inputs(10.0, 20.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_greater_equal() {
    let node = make_node("cmp", "comparison", json!({"operator": ">="}));
    let outputs = evaluate_node(&node, &comparison_inputs(30.0, 30.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_less_equal() {
    let node = make_node("cmp", "comparison", json!({"operator": "<="}));
    let outputs = evaluate_node(&node, &comparison_inputs(30.0, 30.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_equal() {
    let node = make_node("cmp", "comparison", json!({"operator": "=="}));
    let outputs = evaluate_node(&node, &comparison_inputs(42.0, 42.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_not_equal() {
    let node = make_node("cmp", "comparison", json!({"operator": "!="}));
    let outputs = evaluate_node(&node, &comparison_inputs(42.0, 43.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_not_equal_same_value() {
    let node = make_node("cmp", "comparison", json!({"operator": "!="}));
    let outputs = evaluate_node(&node, &comparison_inputs(42.0, 42.0), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_default_operator_is_greater_than() {
    let node = make_node("cmp", "comparison", json!({}));
    let outputs = evaluate_node(&node, &comparison_inputs(31.0, 30.0), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_unknown_operator_returns_false() {
    let node = make_node("cmp", "comparison", json!({"operator": "???"}));
    let outputs = evaluate_node(&node, &comparison_inputs(31.0, 30.0), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_comparison_missing_inputs_default_zero() {
    let node = make_node("cmp", "comparison", json!({"operator": "=="}));
    let outputs = evaluate_node(&node, &HashMap::new(), 0, 0.0);
    // Both default to 0.0, so 0 == 0 is true
    assert!(outputs.get("output").unwrap().as_bool());
}

// ───────────────────────────────────────────────
// evaluate_node — logic_gate
// ───────────────────────────────────────────────

fn logic_inputs(a: bool, b: bool) -> HashMap<String, NodeValue> {
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(a));
    inputs.insert("b".to_string(), NodeValue::Bool(b));
    inputs
}

#[test]
fn test_logic_and_true_true() {
    let node = make_node("lg", "logic_gate", json!({"operator": "AND"}));
    let outputs = evaluate_node(&node, &logic_inputs(true, true), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_and_true_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "AND"}));
    let outputs = evaluate_node(&node, &logic_inputs(true, false), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_and_false_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "AND"}));
    let outputs = evaluate_node(&node, &logic_inputs(false, false), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_or_true_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "OR"}));
    let outputs = evaluate_node(&node, &logic_inputs(true, false), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_or_false_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "OR"}));
    let outputs = evaluate_node(&node, &logic_inputs(false, false), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_not_true() {
    let node = make_node("lg", "logic_gate", json!({"operator": "NOT"}));
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(true));
    let outputs = evaluate_node(&node, &inputs, 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_not_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "NOT"}));
    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(false));
    let outputs = evaluate_node(&node, &inputs, 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_default_operator_is_and() {
    let node = make_node("lg", "logic_gate", json!({}));
    let outputs = evaluate_node(&node, &logic_inputs(true, true), 0, 0.0);
    assert!(outputs.get("output").unwrap().as_bool());
}

#[test]
fn test_logic_unknown_operator_returns_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "XOR"}));
    let outputs = evaluate_node(&node, &logic_inputs(true, true), 0, 0.0);
    assert!(!outputs.get("output").unwrap().as_bool());
}

// ───────────────────────────────────────────────
// evaluate_node — actuator_output
// ───────────────────────────────────────────────

#[test]
fn test_actuator_output_trigger_true() {
    let node = make_node("a1", "actuator_output", json!({"actuator_id": 1}));
    let mut inputs = HashMap::new();
    inputs.insert("trigger".to_string(), NodeValue::Bool(true));

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);
    assert!(outputs.get("state").unwrap().as_bool());
}

#[test]
fn test_actuator_output_trigger_false() {
    let node = make_node("a1", "actuator_output", json!({"actuator_id": 1}));
    let mut inputs = HashMap::new();
    inputs.insert("trigger".to_string(), NodeValue::Bool(false));

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);
    assert!(!outputs.get("state").unwrap().as_bool());
}

#[test]
fn test_actuator_output_no_trigger_defaults_false() {
    let node = make_node("a1", "actuator_output", json!({"actuator_id": 1}));
    let outputs = evaluate_node(&node, &HashMap::new(), 0, 0.0);
    assert!(!outputs.get("state").unwrap().as_bool());
}

#[test]
fn test_actuator_output_numeric_trigger() {
    let node = make_node("a1", "actuator_output", json!({"actuator_id": 1}));
    let mut inputs = HashMap::new();
    inputs.insert("trigger".to_string(), NodeValue::Number(1.0));

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);
    // Number 1.0 -> as_bool() -> true
    assert!(outputs.get("state").unwrap().as_bool());
}

// ───────────────────────────────────────────────
// evaluate_node — unknown type
// ───────────────────────────────────────────────

#[test]
fn test_evaluate_unknown_node_type() {
    let node = make_node("x", "nonexistent_type", json!({}));
    let outputs = evaluate_node(&node, &HashMap::new(), 0, 0.0);
    assert!(outputs.is_empty());
}

// ───────────────────────────────────────────────
// collect_inputs
// ───────────────────────────────────────────────

#[test]
fn test_collect_inputs_basic() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({})),
            make_node("cmp1", "comparison", json!({})),
        ],
        edges: vec![make_edge("e1", "s1", Some("value"), "cmp1", Some("a"))],
    };

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    let mut s1_out = HashMap::new();
    s1_out.insert("value".to_string(), NodeValue::Number(42.0));
    node_outputs.insert("s1".to_string(), s1_out);

    let inputs = collect_inputs("cmp1", &graph, &node_outputs);
    assert!((inputs.get("a").unwrap().as_number() - 42.0).abs() < f64::EPSILON);
}

#[test]
fn test_collect_inputs_multiple_sources() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({})),
            make_node("c1", "constant", json!({})),
            make_node("cmp1", "comparison", json!({})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp1", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp1", Some("b")),
        ],
    };

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    let mut s1_out = HashMap::new();
    s1_out.insert("value".to_string(), NodeValue::Number(35.0));
    node_outputs.insert("s1".to_string(), s1_out);

    let mut c1_out = HashMap::new();
    c1_out.insert("output".to_string(), NodeValue::Number(30.0));
    node_outputs.insert("c1".to_string(), c1_out);

    let inputs = collect_inputs("cmp1", &graph, &node_outputs);
    assert!((inputs.get("a").unwrap().as_number() - 35.0).abs() < f64::EPSILON);
    assert!((inputs.get("b").unwrap().as_number() - 30.0).abs() < f64::EPSILON);
}

#[test]
fn test_collect_inputs_no_edges() {
    let graph = FlowGraph {
        nodes: vec![make_node("n1", "constant", json!({}))],
        edges: vec![],
    };

    let node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    let inputs = collect_inputs("n1", &graph, &node_outputs);
    assert!(inputs.is_empty());
}

#[test]
fn test_collect_inputs_default_handle_names() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({})),
            make_node("n2", "actuator_output", json!({})),
        ],
        edges: vec![make_edge("e1", "s1", None, "n2", None)],
    };

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    let mut s1_out = HashMap::new();
    s1_out.insert("output".to_string(), NodeValue::Bool(true));
    node_outputs.insert("s1".to_string(), s1_out);

    let inputs = collect_inputs("n2", &graph, &node_outputs);
    // Default handle: source="output", target="input"
    assert!(inputs.get("input").unwrap().as_bool());
}

#[test]
fn test_collect_inputs_missing_source_skipped() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({})),
            make_node("cmp1", "comparison", json!({})),
        ],
        edges: vec![make_edge("e1", "s1", Some("value"), "cmp1", Some("a"))],
    };

    // s1 has no outputs computed yet
    let node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    let inputs = collect_inputs("cmp1", &graph, &node_outputs);
    assert!(inputs.is_empty());
}

// ───────────────────────────────────────────────
// NodeValue conversions
// ───────────────────────────────────────────────

#[test]
fn test_node_value_number_as_number() {
    let v = NodeValue::Number(3.14);
    assert!((v.as_number() - 3.14).abs() < f64::EPSILON);
}

#[test]
fn test_node_value_bool_as_number() {
    assert!((NodeValue::Bool(true).as_number() - 1.0).abs() < f64::EPSILON);
    assert!((NodeValue::Bool(false).as_number() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_node_value_none_as_number() {
    assert!((NodeValue::None.as_number() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_node_value_number_as_bool() {
    assert!(NodeValue::Number(1.0).as_bool());
    assert!(NodeValue::Number(-5.0).as_bool());
    assert!(!NodeValue::Number(0.0).as_bool());
}

#[test]
fn test_node_value_bool_as_bool() {
    assert!(NodeValue::Bool(true).as_bool());
    assert!(!NodeValue::Bool(false).as_bool());
}

#[test]
fn test_node_value_none_as_bool() {
    assert!(!NodeValue::None.as_bool());
}

// ───────────────────────────────────────────────
// Full pipeline tests (no DB/socket needed)
// ───────────────────────────────────────────────

/// Simulates a full graph evaluation: sensor → comparison → actuator
#[test]
fn test_full_pipeline_sensor_gt_threshold_triggers_actuator() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("c1", "constant", json!({"value": 30.0})),
            make_node("cmp", "comparison", json!({"operator": ">"})),
            make_node("act", "actuator_output", json!({"actuator_id": 5})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp", Some("b")),
            make_edge("e3", "cmp", Some("output"), "act", Some("trigger")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();
    let trigger_sensor_id = 1;
    let trigger_value = 35.0; // above threshold

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();

    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs);
        let outputs = evaluate_node(node, &inputs, trigger_sensor_id, trigger_value);
        node_outputs.insert(node_id.clone(), outputs);
    }

    // Sensor should output 35.0
    assert!(
        (node_outputs["s1"]["value"].as_number() - 35.0).abs() < f64::EPSILON
    );
    // Constant should output 30.0
    assert!(
        (node_outputs["c1"]["output"].as_number() - 30.0).abs() < f64::EPSILON
    );
    // Comparison: 35 > 30 = true
    assert!(node_outputs["cmp"]["output"].as_bool());
    // Actuator: triggered
    assert!(node_outputs["act"]["state"].as_bool());
}

#[test]
fn test_full_pipeline_sensor_below_threshold_no_trigger() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("c1", "constant", json!({"value": 30.0})),
            make_node("cmp", "comparison", json!({"operator": ">"})),
            make_node("act", "actuator_output", json!({"actuator_id": 5})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp", Some("b")),
            make_edge("e3", "cmp", Some("output"), "act", Some("trigger")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs);
        let outputs = evaluate_node(node, &inputs, 1, 25.0); // below threshold
        node_outputs.insert(node_id.clone(), outputs);
    }

    // 25 > 30 = false
    assert!(!node_outputs["cmp"]["output"].as_bool());
    assert!(!node_outputs["act"]["state"].as_bool());
}

#[test]
fn test_full_pipeline_with_logic_gate() {
    // Two sensors, both must be above threshold (AND gate)
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("s2", "sensor_input", json!({"sensor_id": 1})), // same sensor for test
            make_node("t1", "constant", json!({"value": 20.0})),
            make_node("t2", "constant", json!({"value": 10.0})),
            make_node("cmp1", "comparison", json!({"operator": ">"})),
            make_node("cmp2", "comparison", json!({"operator": ">"})),
            make_node("and", "logic_gate", json!({"operator": "AND"})),
            make_node("act", "actuator_output", json!({"actuator_id": 1})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp1", Some("a")),
            make_edge("e2", "t1", Some("output"), "cmp1", Some("b")),
            make_edge("e3", "s2", Some("value"), "cmp2", Some("a")),
            make_edge("e4", "t2", Some("output"), "cmp2", Some("b")),
            make_edge("e5", "cmp1", Some("output"), "and", Some("a")),
            make_edge("e6", "cmp2", Some("output"), "and", Some("b")),
            make_edge("e7", "and", Some("output"), "act", Some("trigger")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();

    // Value 25: > 20 (true) AND > 10 (true) => true
    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs);
        let outputs = evaluate_node(node, &inputs, 1, 25.0);
        node_outputs.insert(node_id.clone(), outputs);
    }
    assert!(node_outputs["act"]["state"].as_bool());

    // Value 15: > 20 (false) AND > 10 (true) => false
    let mut node_outputs2: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs2);
        let outputs = evaluate_node(node, &inputs, 1, 15.0);
        node_outputs2.insert(node_id.clone(), outputs);
    }
    assert!(!node_outputs2["act"]["state"].as_bool());
}

#[test]
fn test_full_pipeline_not_gate_inverts() {
    // Sensor > 30 → NOT → actuator (turns OFF when temp is high)
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("c1", "constant", json!({"value": 30.0})),
            make_node("cmp", "comparison", json!({"operator": ">"})),
            make_node("not", "logic_gate", json!({"operator": "NOT"})),
            make_node("act", "actuator_output", json!({"actuator_id": 1})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp", Some("b")),
            make_edge("e3", "cmp", Some("output"), "not", Some("a")),
            make_edge("e4", "not", Some("output"), "act", Some("trigger")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();

    // 35 > 30 = true, NOT true = false
    let mut outs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &outs);
        let outputs = evaluate_node(node, &inputs, 1, 35.0);
        outs.insert(node_id.clone(), outputs);
    }
    assert!(!outs["act"]["state"].as_bool());

    // 25 > 30 = false, NOT false = true
    let mut outs2: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &outs2);
        let outputs = evaluate_node(node, &inputs, 1, 25.0);
        outs2.insert(node_id.clone(), outputs);
    }
    assert!(outs2["act"]["state"].as_bool());
}

// ───────────────────────────────────────────────
// Model serialization tests
// ───────────────────────────────────────────────

#[test]
fn test_new_flow_serialization() {
    use homesoil::models::NewFlow;

    let flow = NewFlow::new("Test Flow", r#"{"nodes":[],"edges":[]}"#);
    assert_eq!(flow.get_title(), "Test Flow");
    assert_eq!(flow.get_graph(), r#"{"nodes":[],"edges":[]}"#);
    assert!(!flow.get_enabled());
}

#[test]
fn test_update_flow_deserialization() {
    use homesoil::models::UpdateFlow;

    let json = r#"{"id":1,"title":"Updated","graph":"{}","enabled":true}"#;
    let flow: UpdateFlow = serde_json::from_str(json).unwrap();
    assert_eq!(flow.get_id(), 1);
    assert_eq!(flow.get_title(), "Updated");
    assert!(flow.get_enabled());
}

#[test]
fn test_toggle_flow_deserialization() {
    use homesoil::models::ToggleFlow;

    let json = r#"{"id":5,"enabled":true}"#;
    let toggle: ToggleFlow = serde_json::from_str(json).unwrap();
    assert_eq!(toggle.get_id(), 5);
    assert!(toggle.get_enabled());
}

#[test]
fn test_delete_flow_deserialization() {
    use homesoil::models::DeleteFlow;

    let json = r#"{"id":3}"#;
    let del: DeleteFlow = serde_json::from_str(json).unwrap();
    assert_eq!(del.get_id(), 3);
}

// ───────────────────────────────────────────────
// Edge cases and additional coverage
// ───────────────────────────────────────────────

/// Test that sensor_input with sourceHandle "value" propagates correctly
#[test]
fn test_sensor_input_source_handle_value() {
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("cmp", "comparison", json!({"operator": ">"})),
            make_node("c1", "constant", json!({"value": 20.0})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp", Some("a")),
            make_edge("e2", "c1", Some("output"), "cmp", Some("b")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();
    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();

    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs);
        let outputs = evaluate_node(node, &inputs, 1, 42.5);
        node_outputs.insert(node_id.clone(), outputs);
    }

    // Verify sensor outputs to "value" handle
    assert!(
        (node_outputs["s1"]["value"].as_number() - 42.5).abs() < f64::EPSILON
    );
    // Verify comparison receives it via sourceHandle: "value"
    assert!(node_outputs["cmp"]["output"].as_bool()); // 42.5 > 20.0
}

/// Test that comparison nodes handle NodeValue::Bool inputs (convert via as_number)
#[test]
fn test_comparison_with_boolean_inputs() {
    let node = make_node("cmp", "comparison", json!({"operator": ">"}));

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(true));  // true -> 1.0
    inputs.insert("b".to_string(), NodeValue::Bool(false)); // false -> 0.0

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    // 1.0 > 0.0 = true
    assert!(outputs.get("output").unwrap().as_bool());
}

/// Test that comparison nodes handle mixed bool/number inputs
#[test]
fn test_comparison_mixed_bool_number() {
    let node = make_node("cmp", "comparison", json!({"operator": "=="}));

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(true));   // true -> 1.0
    inputs.insert("b".to_string(), NodeValue::Number(1.0));

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    // 1.0 == 1.0 = true
    assert!(outputs.get("output").unwrap().as_bool());
}

/// Test that logic gate handles NodeValue::Number inputs (convert via as_bool)
#[test]
fn test_logic_gate_with_numeric_inputs() {
    let node = make_node("lg", "logic_gate", json!({"operator": "AND"}));

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Number(1.0));  // 1.0 -> true
    inputs.insert("b".to_string(), NodeValue::Number(5.0));  // 5.0 -> true

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    // true AND true = true
    assert!(outputs.get("output").unwrap().as_bool());
}

/// Test that logic gate converts 0.0 to false
#[test]
fn test_logic_gate_numeric_zero_is_false() {
    let node = make_node("lg", "logic_gate", json!({"operator": "OR"}));

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Number(0.0));  // 0.0 -> false
    inputs.insert("b".to_string(), NodeValue::Number(0.0));  // 0.0 -> false

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    // false OR false = false
    assert!(!outputs.get("output").unwrap().as_bool());
}

/// Test that logic gate handles mixed bool/number inputs
#[test]
fn test_logic_gate_mixed_bool_number() {
    let node = make_node("lg", "logic_gate", json!({"operator": "AND"}));

    let mut inputs = HashMap::new();
    inputs.insert("a".to_string(), NodeValue::Bool(true));
    inputs.insert("b".to_string(), NodeValue::Number(-3.0)); // nonzero -> true

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    // true AND true = true
    assert!(outputs.get("output").unwrap().as_bool());
}

/// Full pipeline test with two independent actuator outputs
#[test]
fn test_full_pipeline_multiple_actuators() {
    // Sensor feeds two comparisons, each triggers a different actuator
    let graph = FlowGraph {
        nodes: vec![
            make_node("s1", "sensor_input", json!({"sensor_id": 1})),
            make_node("t1", "constant", json!({"value": 30.0})),
            make_node("t2", "constant", json!({"value": 20.0})),
            make_node("cmp1", "comparison", json!({"operator": ">"})),  // > 30
            make_node("cmp2", "comparison", json!({"operator": ">"})),  // > 20
            make_node("act1", "actuator_output", json!({"actuator_id": 1})),
            make_node("act2", "actuator_output", json!({"actuator_id": 2})),
        ],
        edges: vec![
            make_edge("e1", "s1", Some("value"), "cmp1", Some("a")),
            make_edge("e2", "t1", Some("output"), "cmp1", Some("b")),
            make_edge("e3", "s1", Some("value"), "cmp2", Some("a")),
            make_edge("e4", "t2", Some("output"), "cmp2", Some("b")),
            make_edge("e5", "cmp1", Some("output"), "act1", Some("trigger")),
            make_edge("e6", "cmp2", Some("output"), "act2", Some("trigger")),
        ],
    };

    let sorted = topological_sort(&graph).unwrap();

    // Test with value = 25: act1 should be OFF (25 > 30 is false), act2 should be ON (25 > 20 is true)
    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs);
        let outputs = evaluate_node(node, &inputs, 1, 25.0);
        node_outputs.insert(node_id.clone(), outputs);
    }

    assert!(!node_outputs["act1"]["state"].as_bool()); // 25 > 30 = false
    assert!(node_outputs["act2"]["state"].as_bool());  // 25 > 20 = true

    // Test with value = 35: both should be ON
    let mut node_outputs2: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs2);
        let outputs = evaluate_node(node, &inputs, 1, 35.0);
        node_outputs2.insert(node_id.clone(), outputs);
    }

    assert!(node_outputs2["act1"]["state"].as_bool());  // 35 > 30 = true
    assert!(node_outputs2["act2"]["state"].as_bool());  // 35 > 20 = true

    // Test with value = 15: both should be OFF
    let mut node_outputs3: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();
    for node_id in &sorted {
        let node = graph.nodes.iter().find(|n| &n.id == node_id).unwrap();
        let inputs = collect_inputs(node_id, &graph, &node_outputs3);
        let outputs = evaluate_node(node, &inputs, 1, 15.0);
        node_outputs3.insert(node_id.clone(), outputs);
    }

    assert!(!node_outputs3["act1"]["state"].as_bool()); // 15 > 30 = false
    assert!(!node_outputs3["act2"]["state"].as_bool()); // 15 > 20 = false
}

/// Test that constant node handles integer JSON values (as_f64 conversion)
#[test]
fn test_constant_node_integer_value() {
    let node = make_node("c1", "constant", json!({"value": 42})); // integer, not float
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - 42.0).abs() < f64::EPSILON);
}

/// Test constant with negative integer
#[test]
fn test_constant_node_negative_integer() {
    let node = make_node("c1", "constant", json!({"value": -99}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - (-99.0)).abs() < f64::EPSILON);
}

/// Test constant with zero integer
#[test]
fn test_constant_node_zero_integer() {
    let node = make_node("c1", "constant", json!({"value": 0}));
    let inputs = HashMap::new();

    let outputs = evaluate_node(&node, &inputs, 0, 0.0);

    let val = outputs.get("output").unwrap().as_number();
    assert!((val - 0.0).abs() < f64::EPSILON);
}
