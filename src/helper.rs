use crate::events::MESSAGE_SENT_EVENT;
use serde_json::{json, Value};
use socketioxide::extract::SocketRef;
use socketioxide::SocketIo;
use socketioxide::SendError;
use std::sync::OnceLock;

/// Global socket handle, set once at startup. Used to emit to all dashboards
/// from threads that don't hold a per-socket reference.
static SOCKET_IO: OnceLock<SocketIo> = OnceLock::new();

pub fn set_socket_io(io: SocketIo) {
    let _ = SOCKET_IO.set(io);
}

/// Emit an event to every connected client in the default namespace.
pub fn broadcast_event(event: &'static str, payload: Value) {
    if let Some(socket) = SOCKET_IO.get() {
        if let Some(ns) = socket.of("/") {
            let _ = ns.emit(event, &payload);
        }
    }
}

pub enum DashboardMessageType {
    Info,
    Success,
    Warning,
    Error,
}

impl DashboardMessageType {
    pub fn get_class(&self) -> String {
        match self {
            DashboardMessageType::Info => "info".to_string(),
            DashboardMessageType::Success => "success".to_string(),
            DashboardMessageType::Warning => "warning".to_string(),
            DashboardMessageType::Error => "error".to_string(),
        }
    }
}

pub fn send_message_to_dashboard(
    socket: &SocketRef,
    message: String,
    message_type: DashboardMessageType,
) -> Result<(), SendError> {
    socket.emit(
        MESSAGE_SENT_EVENT,
        json!({
            "message": message,
            "type": message_type.get_class(),
        }),
    )
}

/// Dashboard message sent to all clients (used from threads without a socket ref).
pub fn broadcast_message_to_dashboard(message: String, message_type: DashboardMessageType) {
    broadcast_event(
        MESSAGE_SENT_EVENT,
        json!({
            "message": message,
            "type": message_type.get_class(),
        }),
    );
}

