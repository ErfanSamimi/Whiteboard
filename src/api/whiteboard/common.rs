use serde::{Serialize, Deserialize};
use crate::whiteboard::{CursorPosition, WhiteBoardData};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEventReceive {
    #[serde(rename = "auth")]
    AUTH {token: String},
    #[serde(rename = "drawing_update")]
    DrawingUpdate {data: WhiteBoardData, user: String},
    #[serde(rename = "cursor_update")]
    CursorUpdate {data: CursorPosition, user: String},
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEventSend {
    #[serde(rename = "auth_success")]
    AuthSuccess {message: String, user_token: String},
    #[serde(rename = "drawing_update")]
    DrawingUpdate {data: WhiteBoardData},
    #[serde(rename = "cursor_update")]
    CursorUpdate {data: CursorPosition},
    #[serde(rename = "error")]
    Error {message: String},
}