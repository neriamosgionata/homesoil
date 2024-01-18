use serde_json::json;
use socketioxide::SendError;
use socketioxide::extract::SocketRef;
use crate::events::MESSAGE_SENT_EVENT;

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

pub fn send_message_to_dashboard(socket: &SocketRef, message: String, message_type: DashboardMessageType) -> Result<(), SendError> {
    socket.emit(
        MESSAGE_SENT_EVENT,
        json!({
                "message": message,
                "type": message_type.get_class(),
            }),
    )
}