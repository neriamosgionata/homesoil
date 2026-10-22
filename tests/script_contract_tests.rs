use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use homesoil::models::{NewScript, Script, UpdateScript};
use homesoil::schema::scripts;

fn setup_test_db() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:")
        .expect("Failed to create in-memory SQLite DB");

    diesel::sql_query(
        "CREATE TABLE scripts (
            id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            code TEXT NOT NULL,
            schedule TEXT NULL,
            status INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NULL
        )",
    )
    .execute(&mut conn)
    .expect("Failed to create scripts table");

    conn
}

// Payload emitted by the Android app (kotlinx.serialization omits null defaults).
const ANDROID_NEW_SCRIPT_PAYLOAD: &str = r#"{
    "id": 0,
    "title": "Water the garden",
    "code": "RUN\nPULSE 1\nSTOP",
    "schedule": "0 6 * * *",
    "status": 0,
    "created_at": "2026-10-22T12:34:56.789"
}"#;

// Payload emitted by the frontend (created_at comes from the server).
const FRONTEND_NEW_SCRIPT_PAYLOAD: &str = r#"{
    "title": "Frontend script",
    "code": "RUN\nSEND_TO_DASHBOARD \"hello\"\nSTOP",
    "schedule": null,
    "status": 0,
    "created_at": "2026-10-22T12:34:56"
}"#;

fn insert_new_script(conn: &mut SqliteConnection, payload: &str) -> Script {
    let new_script: NewScript =
        serde_json::from_str(payload).expect("NewScript payload should deserialize");

    diesel::insert_into(scripts::table)
        .values(new_script)
        .execute(conn)
        .expect("Failed to insert script");

    scripts::table
        .order(scripts::id.desc())
        .first(conn)
        .expect("Failed to load inserted script")
}

#[test]
fn android_add_script_payload_is_accepted() {
    let mut conn = setup_test_db();
    let script = insert_new_script(&mut conn, ANDROID_NEW_SCRIPT_PAYLOAD);

    assert_eq!(script.get_title(), "Water the garden");
    assert_eq!(script.get_code(), "RUN\nPULSE 1\nSTOP");
    assert_eq!(script.get_schedule().as_deref(), Some("0 6 * * *"));
    assert_eq!(script.get_status(), 0);
    assert_eq!(
        script.get_created_at().to_string(),
        "2026-10-22 12:34:56.789"
    );
}

#[test]
fn frontend_add_script_payload_is_accepted() {
    let mut conn = setup_test_db();
    let script = insert_new_script(&mut conn, FRONTEND_NEW_SCRIPT_PAYLOAD);

    assert_eq!(script.get_title(), "Frontend script");
    assert_eq!(script.get_schedule(), &None);
    assert_eq!(script.get_status(), 0);
}

#[test]
fn update_script_payload_ignores_client_timestamps() {
    let mut conn = setup_test_db();
    let script = insert_new_script(&mut conn, ANDROID_NEW_SCRIPT_PAYLOAD);

    let payload = format!(
        r#"{{
            "id": {},
            "title": "Renamed",
            "code": "RUN\nDEACTIVATE 1\nSTOP",
            "schedule": null,
            "status": 1,
            "created_at": "2026-10-22T12:34:56.789",
            "updated_at": "2020-01-01T00:00:00"
        }}"#,
        script.get_id()
    );

    let update: UpdateScript =
        serde_json::from_str(&payload).expect("UpdateScript payload should deserialize");

    assert_eq!(update.get_id(), script.get_id());
    assert_eq!(update.get_title(), "Renamed");
    assert_eq!(update.get_schedule(), &None);

    // Mirrors script_methods::update_script: content from the payload,
    // timestamp stamped server-side.
    let server_now = chrono::Local::now().naive_local();
    diesel::update(scripts::table.find(update.get_id()))
        .set((
            scripts::title.eq(update.get_title()),
            scripts::code.eq(update.get_code()),
            scripts::schedule.eq(update.get_schedule()),
            scripts::updated_at.eq(server_now),
        ))
        .execute(&mut conn)
        .expect("Failed to update script");

    let reloaded: Script = scripts::table
        .find(script.get_id())
        .first(&mut conn)
        .expect("Failed to reload script");

    assert_eq!(reloaded.get_title(), "Renamed");
    assert_eq!(reloaded.get_code(), "RUN\nDEACTIVATE 1\nSTOP");
    assert_eq!(reloaded.get_schedule(), &None);
    assert_eq!(reloaded.get_updated_at(), &Some(server_now));
}

#[test]
fn schedule_can_be_added_and_removed() {
    let mut conn = setup_test_db();
    let script = insert_new_script(&mut conn, ANDROID_NEW_SCRIPT_PAYLOAD);

    diesel::update(scripts::table.find(script.get_id()))
        .set(scripts::schedule.eq(Some("*/5 * * * *")))
        .execute(&mut conn)
        .unwrap();

    let scheduled: Script = scripts::table.find(script.get_id()).first(&mut conn).unwrap();
    assert_eq!(scheduled.get_schedule().as_deref(), Some("*/5 * * * *"));

    diesel::update(scripts::table.find(script.get_id()))
        .set(scripts::schedule.eq(Option::<String>::None))
        .execute(&mut conn)
        .unwrap();

    let cleared: Script = scripts::table.find(script.get_id()).first(&mut conn).unwrap();
    assert_eq!(cleared.get_schedule(), &None);
}

#[test]
fn script_status_values_round_trip() {
    let mut conn = setup_test_db();

    for status in [0, 1, -1, -2] {
        let payload = format!(
            r#"{{"title":"status {status}","code":"RUN\nSTOP","schedule":null,"status":{status},"created_at":"2026-10-22T12:34:56"}}"#
        );
        let script = insert_new_script(&mut conn, &payload);
        assert_eq!(script.get_status(), status);
    }
}

#[test]
fn serialized_script_exposes_client_contract_keys() {
    let mut conn = setup_test_db();
    let script = insert_new_script(&mut conn, ANDROID_NEW_SCRIPT_PAYLOAD);

    let value = serde_json::to_value(&script).expect("Script should serialize");

    for key in [
        "id",
        "title",
        "code",
        "schedule",
        "status",
        "created_at",
        "updated_at",
    ] {
        assert!(
            value.get(key).is_some(),
            "serialized script should contain {key}"
        );
    }

    assert_eq!(value["id"], script.get_id());
    assert_eq!(value["schedule"], "0 6 * * *");
}
