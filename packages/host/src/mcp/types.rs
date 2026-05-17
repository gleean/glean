//! JSON-RPC 2.0 types shared by MCP-oriented handlers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcErrorBody {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub error: JsonRpcErrorBody,
}

pub fn parse_error_response(err: serde_json::Error) -> String {
    JsonRpcErrorResponse {
        jsonrpc: "2.0".to_string(),
        id: Value::Null,
        error: JsonRpcErrorBody {
            code: -32700,
            message: "Parse error".to_string(),
            data: Some(json!({"detail": err.to_string()})),
        },
    }
    .serialize_to_line()
}

pub fn jsonrpc_error(
    id: Value,
    code: i32,
    message: impl Into<String>,
    data: Option<Value>,
) -> String {
    JsonRpcErrorResponse {
        jsonrpc: "2.0".to_string(),
        id,
        error: JsonRpcErrorBody {
            code,
            message: message.into(),
            data,
        },
    }
    .serialize_to_line()
}

trait SerializeLine {
    fn serialize_to_line(&self) -> String;
}

impl<T: Serialize> SerializeLine for T {
    fn serialize_to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"serialization failure"}}"#
                .to_string()
        })
    }
}
