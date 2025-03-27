use serde::{ Serialize, Deserialize };
use crate::whiteboard::{ CursorPosition, WhiteBoardData };

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEventReceive {
    #[serde(rename = "auth")] AUTH {
        token: String,
    },
    #[serde(rename = "drawing_update")] DrawingUpdate {
        data: WhiteBoardData,
        user: String,
    },
    #[serde(rename = "cursor_update")] CursorUpdate {
        data: CursorPosition,
        user: String,
    },
}

impl WsEventReceive {
    pub fn get_name(&self) -> &str {
        match self {
            Self::AUTH { token } => "AUTH",
            Self::DrawingUpdate { data, user } => "DrawingUpdate",
            Self::CursorUpdate { data, user } => "CursorUpdate",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEventSend {
    #[serde(rename = "auth_success")] AuthSuccess {
        message: String,
        user_token: String,
    },
    #[serde(rename = "drawing_update")] DrawingUpdate {
        data: WhiteBoardData,
    },
    #[serde(rename = "cursor_update")] CursorUpdate {
        data: CursorPosition,
    },
    #[serde(rename = "error")] Error {
        message: String,
    },
}

impl WsEventSend {
    pub fn get_name(&self) -> &str {
        match self {
            Self::AuthSuccess { message, user_token } => "[ :) ]AuthSuccess",
            Self::DrawingUpdate { data } => "[ x ]DrawingUpdate",
            Self::CursorUpdate { data } => "[ . ]CursorUpdate",
            Self::Error { message } => "[ :-(  ]Error",
        }
    }
}

impl From<&WsEventReceive> for WsEventSend {
    fn from(value: &WsEventReceive) -> Self {
        match value {
            WsEventReceive::DrawingUpdate { data, user } =>
                Self::DrawingUpdate { data: data.clone() },
            WsEventReceive::CursorUpdate { data, user } =>
                Self::CursorUpdate { data: data.clone() },
            _ => Self::Error { message: "Invalid event at this state!".to_string() },
        }
    }
}
