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

#[derive(Debug, Serialize, Deserialize, Clone)]
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


use flate2::write::DeflateEncoder;
use flate2::read::DeflateDecoder;
use flate2::Compression;
use std::io::{Write, Read};

pub fn compress_data(data: String) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
    match encoder.write_all(data.as_bytes()) {
        Ok(_) =>
            match encoder.finish() {
                Ok(compressed) => { compressed }
                Err(e) => {
                    println!("Compression error: {}", e);
                    data.into_bytes()
                }
            }
        Err(e) => {
            println!("Compression error: {}", e);
            data.into_bytes()
        }
    }
}

pub fn decompress_data(data: Vec<u8>) -> Result<String, String> {

    // First, try raw DEFLATE decompression (like Python's zlib with wbits=-15)
    let decompressed = match DeflateDecoder::new(&data[..]).bytes().collect::<Result<Vec<u8>, _>>() {
        Ok(decompressed_bytes) => {
            decompressed_bytes
        }
        Err(e) => {
            println!("Decompression failed: {}, trying as regular UTF-8 JSON", e);
            return String::from_utf8(data)
                .map_err(|e| {
                    println!("UTF-8 decode error on fallback: {}", e);
                    format!("UTF-8 decode error: {}", e)
                });
        }
    };

    String::from_utf8(decompressed).map_err(|e| {
        println!("UTF-8 decode error after decompression: {}", e);
        format!("UTF-8 decode error: {}", e)
    })
}
