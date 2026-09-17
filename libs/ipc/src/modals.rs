use serde::{Deserialize, Serialize};
use crate::send_command;

pub const MODALS_SOCKET_NAME: &str = "finick-modals";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModalRequest {
    WifiPassword { ssid: String, security: String },
    BluetoothPair { name: String, mac: String },
    PamAuth { prompt: String },
    Clipboard,
    Screenshot,
    CloseAll,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModalResponse {
    Success { data: Option<String> },
    Canceled,
    Error { message: String },
}

pub fn send_modal_request(req: ModalRequest) -> std::io::Result<ModalResponse> {
    let (tx, rx) = std::sync::mpsc::channel();
    send_command(
        MODALS_SOCKET_NAME,
        &req,
        Some(move |resp: ModalResponse| {
            let _ = tx.send(resp);
        }),
    )?;
    Ok(rx.recv().unwrap_or(ModalResponse::Error { message: "No response".into() }))
}
