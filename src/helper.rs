use serde_json::json;
use socketioxide::SendError;
use socketioxide::extract::SocketRef;
use crate::events::MESSAGE_SENT_EVENT;

pub fn send_message_to_dashboard(socket: &SocketRef, message: String) -> Result<(), SendError> {
    socket.emit(
        MESSAGE_SENT_EVENT,
        json!({
                "message": message,
            }),
    )
}