use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── JSON-RPC 2.0 Core Types ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

// ─── Standard Error Codes ───────────────────────────────────────────

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

// ─── Constructors ───────────────────────────────────────────────────

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>, id: impl Into<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: id.into(),
        }
    }

    pub fn with_string_id(
        method: impl Into<String>,
        params: Option<Value>,
        id: impl Into<String>,
    ) -> Self {
        Self::new(method, params, Value::String(id.into()))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn success_with_string_id(id: impl Into<String>, result: Value) -> Self {
        Self::success(Value::String(id.into()), result)
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }

    pub fn error_with_string_id(
        id: impl Into<String>,
        code: i32,
        message: impl Into<String>,
    ) -> Self {
        Self::error(Value::String(id.into()), code, message)
    }

    pub fn internal_error(id: Value, message: impl Into<String>) -> Self {
        Self::error(id, INTERNAL_ERROR, message)
    }

    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── Parsing Helpers ────────────────────────────────────────────────

pub fn parse_rpc_message(raw: &str) -> Option<JsonRpcRequest> {
    serde_json::from_str::<JsonRpcRequest>(raw).ok()
}

pub fn parse_rpc_response(raw: &str) -> Option<JsonRpcResponse> {
    serde_json::from_str::<JsonRpcResponse>(raw).ok()
}

pub fn parse_rpc_notification(raw: &str) -> Option<JsonRpcNotification> {
    serde_json::from_str::<JsonRpcNotification>(raw).ok()
}

/// Try to parse as any JSON-RPC message type. Returns tagged enum.
pub enum RpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

pub fn parse_rpc_any(raw: &str) -> Option<RpcMessage> {
    if let Some(notif) = parse_rpc_notification(raw) {
        Some(RpcMessage::Notification(notif))
    } else if let Some(resp) = parse_rpc_response(raw) {
        Some(RpcMessage::Response(resp))
    } else if let Some(req) = parse_rpc_message(raw) {
        Some(RpcMessage::Request(req))
    } else {
        None
    }
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(
            "indicator.analyze",
            Some(serde_json::json!({"rsi": 45.0})),
            1,
        );
        let json = req.to_json();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"indicator.analyze\""));
    }

    #[test]
    fn test_response_success_serialization() {
        let resp = JsonRpcResponse::success_with_string_id(
            "1",
            serde_json::json!({"signal": "BULLISH", "confidence": 85}),
        );
        let json = resp.to_json();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_response_error_serialization() {
        let resp = JsonRpcResponse::error_with_string_id("2", -32600, "Invalid Request");
        let json = resp.to_json();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }

    #[test]
    fn test_notification_serialization() {
        let notif = JsonRpcNotification::new(
            "broadcast.market_snapshot",
            serde_json::json!({"symbol": "BTC", "price": 50000}),
        );
        let json = notif.to_json();
        assert!(json.contains("\"method\":\"broadcast.market_snapshot\""));
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_parse_roundtrip_request() {
        let req = JsonRpcRequest::with_string_id("test.method", None, "abc");
        let json = req.to_json();
        let parsed = parse_rpc_message(&json).unwrap();
        assert_eq!(parsed.method, "test.method");
    }

    #[test]
    fn test_parse_roundtrip_response() {
        let resp = JsonRpcResponse::success_with_string_id("x", serde_json::json!({"ok": true}));
        let json = resp.to_json();
        let parsed = parse_rpc_response(&json).unwrap();
        assert!(parsed.is_success());
    }

    #[test]
    fn test_parse_roundtrip_notification() {
        let notif = JsonRpcNotification::new("event.fired", serde_json::json!({"data": 42}));
        let json = notif.to_json();
        let parsed = parse_rpc_notification(&json).unwrap();
        assert_eq!(parsed.method, "event.fired");
    }
}
