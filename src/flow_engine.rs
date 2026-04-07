use crate::flow_methods::get_enabled_flows;
use crate::intermittent::{start_intermittent, stop_intermittent};
use crate::models::Flow;
use crate::schema::actuators;
use crate::db::connect;
use crate::events::ACTUATOR_STATE_CHANGE_EVENT;
use crate::coap_client;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use socketioxide::SocketIo;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use log::{error, info};

static FLOW_OUTPUT_STATES: LazyLock<Mutex<HashMap<i32, HashMap<String, bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowGraph {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub data: serde_json::Value,
    pub position: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    #[serde(rename = "sourceHandle")]
    pub source_handle: Option<String>,
    pub target: String,
    #[serde(rename = "targetHandle")]
    pub target_handle: Option<String>,
}

#[derive(Debug, Clone)]
pub enum NodeValue {
    Number(f64),
    Bool(bool),
    None,
}

impl NodeValue {
    pub fn as_number(&self) -> f64 {
        match self {
            NodeValue::Number(n) => *n,
            NodeValue::Bool(b) => if *b { 1.0 } else { 0.0 },
            NodeValue::None => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            NodeValue::Number(n) => *n != 0.0,
            NodeValue::Bool(b) => *b,
            NodeValue::None => false,
        }
    }
}

pub fn evaluate_flows_for_sensor(sensor_id: i32, sensor_value: &str, socket: &SocketIo) {
    let flows = match get_enabled_flows() {
        Ok(f) => f,
        Err(e) => {
            error!("Error loading enabled flows: {:?}", e);
            return;
        }
    };

    let value: f64 = match sensor_value.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    for flow in flows {
        let graph: FlowGraph = match serde_json::from_str(flow.get_graph()) {
            Ok(g) => g,
            Err(_) => continue,
        };

        let has_sensor = graph.nodes.iter().any(|n| {
            n.node_type == "sensor_input"
                && n.data.get("sensor_id").and_then(|v| v.as_i64()) == Some(sensor_id as i64)
        });

        if !has_sensor {
            continue;
        }

        evaluate_flow(&flow, &graph, sensor_id, value, socket);
    }
}

fn evaluate_flow(
    flow: &Flow,
    graph: &FlowGraph,
    trigger_sensor_id: i32,
    trigger_value: f64,
    socket: &SocketIo,
) {
    let sorted = match topological_sort(graph) {
        Some(s) => s,
        None => {
            info!("Flow {} has cycles, skipping", flow.get_id());
            return;
        }
    };

    let mut node_outputs: HashMap<String, HashMap<String, NodeValue>> = HashMap::new();

    for node_id in &sorted {
        let node = match graph.nodes.iter().find(|n| &n.id == node_id) {
            Some(n) => n,
            None => continue,
        };

        let inputs = collect_inputs(node_id, graph, &node_outputs);

        let outputs = evaluate_node(node, &inputs, trigger_sensor_id, trigger_value);
        node_outputs.insert(node_id.clone(), outputs);
    }

    let mut states = FLOW_OUTPUT_STATES.lock().unwrap();
    let prev_states = states.entry(flow.get_id()).or_insert_with(HashMap::new);

    for node in &graph.nodes {
        if node.node_type != "actuator_output" {
            continue;
        }

        let actuator_id = match node.data.get("actuator_id").and_then(|v| v.as_i64()) {
            Some(id) => id as i32,
            None => continue,
        };

        let outputs = match node_outputs.get(&node.id) {
            Some(o) => o,
            None => continue,
        };

        let new_state = outputs
            .get("state")
            .map(|v| v.as_bool())
            .unwrap_or(false);

        let prev_state = prev_states.get(&node.id).copied().unwrap_or(!new_state);

        if new_state != prev_state {
            prev_states.insert(node.id.clone(), new_state);

            let is_pulse = node
                .data
                .get("pulse")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let is_intermittent = node
                .data
                .get("intermittent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Pulse actuators only fire on rising edge (false→true)
            if is_pulse && !new_state {
                continue;
            }

            if is_intermittent {
                if new_state {
                    let on_ms = node
                        .data
                        .get("intermittent_on_ms")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1000) as i32;
                    let off_ms = node
                        .data
                        .get("intermittent_off_ms")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1000) as i32;
                    start_intermittent(actuator_id, on_ms, off_ms);
                } else {
                    stop_intermittent(actuator_id);
                }
            } else {
                send_actuator_command(actuator_id, new_state, is_pulse, socket);
            }
        }
    }
}

pub fn collect_inputs(
    node_id: &str,
    graph: &FlowGraph,
    node_outputs: &HashMap<String, HashMap<String, NodeValue>>,
) -> HashMap<String, NodeValue> {
    let mut inputs: HashMap<String, NodeValue> = HashMap::new();

    for edge in &graph.edges {
        if edge.target != node_id {
            continue;
        }

        let source_outputs = match node_outputs.get(&edge.source) {
            Some(o) => o,
            None => continue,
        };

        let source_handle = edge.source_handle.as_deref().unwrap_or("output");
        let target_handle = edge.target_handle.as_deref().unwrap_or("input");

        if let Some(val) = source_outputs.get(source_handle) {
            inputs.insert(target_handle.to_string(), val.clone());
        }
    }

    inputs
}

pub fn evaluate_node(
    node: &FlowNode,
    inputs: &HashMap<String, NodeValue>,
    trigger_sensor_id: i32,
    trigger_value: f64,
) -> HashMap<String, NodeValue> {
    let mut outputs: HashMap<String, NodeValue> = HashMap::new();

    match node.node_type.as_str() {
        "sensor_input" => {
            let sid = node.data.get("sensor_id").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
            if sid == trigger_sensor_id {
                outputs.insert("value".to_string(), NodeValue::Number(trigger_value));
            } else {
                let val = get_latest_sensor_value(sid);
                outputs.insert("value".to_string(), NodeValue::Number(val));
            }
        }
        "constant" => {
            let val = node.data.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
            outputs.insert("output".to_string(), NodeValue::Number(val));
        }
        "comparison" => {
            let a = inputs.get("a").map(|v| v.as_number()).unwrap_or(0.0);
            let b = inputs.get("b").map(|v| v.as_number()).unwrap_or(0.0);
            let op = node.data.get("operator").and_then(|v| v.as_str()).unwrap_or(">");

            let result = match op {
                ">" => a > b,
                "<" => a < b,
                ">=" => a >= b,
                "<=" => a <= b,
                "==" => (a - b).abs() < f64::EPSILON,
                "!=" => (a - b).abs() >= f64::EPSILON,
                _ => false,
            };

            outputs.insert("output".to_string(), NodeValue::Bool(result));
        }
        "logic_gate" => {
            let a = inputs.get("a").map(|v| v.as_bool()).unwrap_or(false);
            let b = inputs.get("b").map(|v| v.as_bool()).unwrap_or(false);
            let op = node.data.get("operator").and_then(|v| v.as_str()).unwrap_or("AND");

            let result = match op {
                "AND" => a && b,
                "OR" => a || b,
                "NOT" => !a,
                _ => false,
            };

            outputs.insert("output".to_string(), NodeValue::Bool(result));
        }
        "actuator_output" => {
            let trigger = inputs.get("trigger").map(|v| v.as_bool()).unwrap_or(false);
            outputs.insert("state".to_string(), NodeValue::Bool(trigger));
        }
        _ => {}
    }

    outputs
}

fn get_latest_sensor_value(sensor_id: i32) -> f64 {
    use crate::schema::sensor_reads;

    let conn = &mut match connect() {
        Ok(c) => c,
        Err(_) => return 0.0,
    };

    let result: Result<String, _> = sensor_reads::table
        .filter(sensor_reads::sensor_id.eq(sensor_id))
        .order(sensor_reads::created_at.desc())
        .select(sensor_reads::sensor_value)
        .first(conn);

    match result {
        Ok(val) => val.parse().unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

pub fn topological_sort(graph: &FlowGraph) -> Option<Vec<String>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();

    for node in &graph.nodes {
        in_degree.entry(node.id.clone()).or_insert(0);
        adj.entry(node.id.clone()).or_insert_with(Vec::new);
    }

    for edge in &graph.edges {
        *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
        adj.entry(edge.source.clone())
            .or_insert_with(Vec::new)
            .push(edge.target.clone());
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut sorted = Vec::new();

    while let Some(node) = queue.pop() {
        sorted.push(node.clone());
        if let Some(neighbors) = adj.get(&node) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
    }

    if sorted.len() == graph.nodes.len() {
        Some(sorted)
    } else {
        None
    }
}

fn send_actuator_command(actuator_id: i32, state: bool, pulse: bool, socket: &SocketIo) {
    let conn = &mut match connect() {
        Ok(c) => c,
        Err(e) => {
            error!("Flow engine: error connecting to DB: {:?}", e);
            return;
        }
    };

    let actuator: crate::models::Actuator = match actuators::table
        .find(actuator_id)
        .first(conn)
    {
        Ok(a) => a,
        Err(e) => {
            info!("Flow engine: actuator {} not found: {:?}", actuator_id, e);
            return;
        }
    };

    let command: &[u8] = if pulse {
        b"ON-PULSE"
    } else if state {
        b"ON"
    } else {
        b"OFF"
    };

    match coap_client::send_actuator_command(
        actuator.get_ip_address(),
        actuator.get_port(),
        command,
    ) {
        Ok(payload) => {
            let new_state = payload == "ON" || payload == "ON-PULSE";

            let _ = diesel::update(actuators::table.find(actuator_id))
                .set((
                    actuators::state.eq(new_state),
                    actuators::updated_at.eq(chrono::Local::now().naive_local()),
                ))
                .execute(conn);

            if let Some(ns) = socket.of("/") {
                let now = chrono::Local::now().naive_local();
                let _ = ns.emit(
                    ACTUATOR_STATE_CHANGE_EVENT,
                    json!({
                        "actuator_id": actuator_id,
                        "actuator_state": new_state,
                        "updated_at": now,
                    }),
                );
            }

            info!(
                "Flow engine: actuator {} set to {} (pulse={})",
                actuator_id, new_state, pulse
            );
        }
        Err(e) => {
            info!(
                "Flow engine: error sending command to actuator {}: {:?}",
                actuator_id, e
            );
        }
    }
}
