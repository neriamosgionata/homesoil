use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use homesoil::models::{Flow, NewFlow, UpdateFlow};
use homesoil::schema::flows;

fn setup_test_db() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:")
        .expect("Failed to create in-memory SQLite DB");

    diesel::sql_query(
        "CREATE TABLE flows (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL DEFAULT 'Default',
            graph TEXT NOT NULL DEFAULT '{}',
            enabled TINYINT NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NULL
        )",
    )
    .execute(&mut conn)
    .expect("Failed to create flows table");

    conn
}

fn insert_flow(conn: &mut SqliteConnection, title: &str, graph: &str, enabled: bool) -> Flow {
    let now = chrono::Local::now().naive_local();

    diesel::insert_into(flows::table)
        .values((
            flows::title.eq(title),
            flows::graph.eq(graph),
            flows::enabled.eq(enabled),
            flows::created_at.eq(now),
        ))
        .execute(conn)
        .expect("Failed to insert flow");

    flows::table
        .order(flows::id.desc())
        .first(conn)
        .expect("Failed to load inserted flow")
}

// ───────────────────────────────────────────────
// CRUD operations
// ───────────────────────────────────────────────

#[test]
fn test_insert_and_load_flow() {
    let mut conn = setup_test_db();

    let flow = insert_flow(&mut conn, "Test Flow", r#"{"nodes":[],"edges":[]}"#, false);

    assert_eq!(flow.get_title(), "Test Flow");
    assert_eq!(flow.get_graph(), r#"{"nodes":[],"edges":[]}"#);
    assert!(!flow.get_enabled());
}

#[test]
fn test_load_all_flows() {
    let mut conn = setup_test_db();

    insert_flow(&mut conn, "Flow 1", "{}", false);
    insert_flow(&mut conn, "Flow 2", "{}", true);
    insert_flow(&mut conn, "Flow 3", "{}", false);

    let all: Vec<Flow> = flows::table.load(&mut conn).unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn test_load_enabled_flows_only() {
    let mut conn = setup_test_db();

    insert_flow(&mut conn, "Disabled", "{}", false);
    insert_flow(&mut conn, "Enabled 1", "{}", true);
    insert_flow(&mut conn, "Enabled 2", "{}", true);

    let enabled: Vec<Flow> = flows::table
        .filter(flows::enabled.eq(true))
        .load(&mut conn)
        .unwrap();

    assert_eq!(enabled.len(), 2);
    assert!(enabled.iter().all(|f| f.get_enabled()));
}

#[test]
fn test_update_flow() {
    let mut conn = setup_test_db();

    let flow = insert_flow(&mut conn, "Original", r#"{"nodes":[],"edges":[]}"#, false);
    let now = chrono::Local::now().naive_local();

    diesel::update(flows::table.find(flow.get_id()))
        .set((
            flows::title.eq("Updated Title"),
            flows::graph.eq(r#"{"nodes":[{"id":"n1"}],"edges":[]}"#),
            flows::enabled.eq(true),
            flows::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .unwrap();

    let updated: Flow = flows::table.find(flow.get_id()).first(&mut conn).unwrap();
    assert_eq!(updated.get_title(), "Updated Title");
    assert!(updated.get_enabled());
    assert!(updated.get_updated_at().is_some());
}

#[test]
fn test_toggle_flow_enabled() {
    let mut conn = setup_test_db();

    let flow = insert_flow(&mut conn, "Toggle Me", "{}", false);
    assert!(!flow.get_enabled());

    diesel::update(flows::table.find(flow.get_id()))
        .set(flows::enabled.eq(true))
        .execute(&mut conn)
        .unwrap();

    let toggled: Flow = flows::table.find(flow.get_id()).first(&mut conn).unwrap();
    assert!(toggled.get_enabled());

    diesel::update(flows::table.find(flow.get_id()))
        .set(flows::enabled.eq(false))
        .execute(&mut conn)
        .unwrap();

    let toggled_back: Flow = flows::table.find(flow.get_id()).first(&mut conn).unwrap();
    assert!(!toggled_back.get_enabled());
}

#[test]
fn test_delete_flow() {
    let mut conn = setup_test_db();

    let flow = insert_flow(&mut conn, "Delete Me", "{}", false);

    diesel::delete(flows::table.find(flow.get_id()))
        .execute(&mut conn)
        .unwrap();

    let result: Result<Flow, _> = flows::table.find(flow.get_id()).first(&mut conn);
    assert!(result.is_err());
}

#[test]
fn test_delete_nonexistent_flow() {
    let mut conn = setup_test_db();

    let count = diesel::delete(flows::table.find(99999))
        .execute(&mut conn)
        .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn test_empty_table_returns_empty_vec() {
    let mut conn = setup_test_db();

    let all: Vec<Flow> = flows::table.load(&mut conn).unwrap();
    assert!(all.is_empty());
}

// ───────────────────────────────────────────────
// Graph content preservation
// ───────────────────────────────────────────────

#[test]
fn test_complex_graph_json_preserved() {
    let mut conn = setup_test_db();

    let complex_graph = serde_json::json!({
        "nodes": [
            {"id": "s1", "type": "sensor_input", "data": {"sensor_id": 1}, "position": {"x": 100, "y": 200}},
            {"id": "c1", "type": "constant", "data": {"value": 30.0}, "position": {"x": 300, "y": 200}},
            {"id": "cmp1", "type": "comparison", "data": {"operator": ">"}, "position": {"x": 500, "y": 200}},
            {"id": "a1", "type": "actuator_output", "data": {"actuator_id": 5, "pulse": false}, "position": {"x": 700, "y": 200}}
        ],
        "edges": [
            {"id": "e1", "source": "s1", "sourceHandle": "value", "target": "cmp1", "targetHandle": "a"},
            {"id": "e2", "source": "c1", "sourceHandle": "output", "target": "cmp1", "targetHandle": "b"},
            {"id": "e3", "source": "cmp1", "sourceHandle": "output", "target": "a1", "targetHandle": "trigger"}
        ]
    })
    .to_string();

    let flow = insert_flow(&mut conn, "Complex", &complex_graph, true);

    let loaded: Flow = flows::table.find(flow.get_id()).first(&mut conn).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(loaded.get_graph()).unwrap();

    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["nodes"][0]["type"], "sensor_input");
}

// ───────────────────────────────────────────────
// Model deserialization
// ───────────────────────────────────────────────

#[test]
fn test_new_flow_from_json() {
    let json = r#"{"title":"My Flow","graph":"{\"nodes\":[],\"edges\":[]}","enabled":false,"created_at":"2026-03-17T10:00:00"}"#;
    let flow: NewFlow = serde_json::from_str(json).unwrap();
    assert_eq!(flow.get_title(), "My Flow");
    assert!(!flow.get_enabled());
}

#[test]
fn test_update_flow_from_json() {
    let json = r#"{"id":1,"title":"Updated","graph":"{}","enabled":true}"#;
    let mut flow: UpdateFlow = serde_json::from_str(json).unwrap();

    assert_eq!(flow.get_id(), 1);
    assert_eq!(flow.get_title(), "Updated");
    assert!(flow.get_enabled());
    assert!(flow.get_updated_at().is_none());

    flow.set_updated_at(chrono::Local::now().naive_local());
    assert!(flow.get_updated_at().is_some());
}

// ───────────────────────────────────────────────
// Multiple operations sequencing
// ───────────────────────────────────────────────

#[test]
fn test_insert_update_toggle_delete_sequence() {
    let mut conn = setup_test_db();

    // Insert
    let flow = insert_flow(&mut conn, "Lifecycle", "{}", false);
    let id = flow.get_id();

    // Update
    diesel::update(flows::table.find(id))
        .set(flows::title.eq("Lifecycle Updated"))
        .execute(&mut conn)
        .unwrap();

    let updated: Flow = flows::table.find(id).first(&mut conn).unwrap();
    assert_eq!(updated.get_title(), "Lifecycle Updated");

    // Toggle on
    diesel::update(flows::table.find(id))
        .set(flows::enabled.eq(true))
        .execute(&mut conn)
        .unwrap();

    let toggled: Flow = flows::table.find(id).first(&mut conn).unwrap();
    assert!(toggled.get_enabled());

    // Delete
    diesel::delete(flows::table.find(id))
        .execute(&mut conn)
        .unwrap();

    let count: i64 = flows::table
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(count, 0);
}
