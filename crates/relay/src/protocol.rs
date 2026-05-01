//! WebSocket 帧协议定义（Relay ↔ Mac Bridge 通信协议）。
//!
//! 职责：定义 Relay 与 Mac Bridge 之间通过 WebSocket 交换的所有消息类型。
//!
//! 架构角色：通信协议层。所有 WS 消息都序列化为 JSON，通过 type 字段区分类型。
//! 新增消息类型必须在此模块添加对应的结构体和 IncomingFrame 变体。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mac → Relay: 注册 Bridge 会话，携带 mac_token 进行认证。
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterFrame {
    pub r#type: String,
    pub mac_token: String,
}

/// Relay → Mac: 转发的手机端 HTTP 请求。
///
/// 包含完整的请求信息（method/path/query/headers/body），
/// Bridge 收到后构造本地 HTTP 请求发送到本地服务。
#[derive(Serialize, Deserialize, Debug)]
pub struct ProxyRequest {
    pub r#type: String,
    /// 唯一请求 ID，用于匹配 ProxyResponse。
    pub request_id: String,
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Mac → Relay: 对代理请求的响应。
///
/// 包含 Bridge 本地 HTTP 服务的完整响应（status/headers/body）。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxyResponse {
    pub r#type: String,
    /// 与 ProxyRequest.request_id 对应，用于匹配。
    pub request_id: String,
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Relay → Mac: 心跳 ping 帧。
#[derive(Serialize, Deserialize, Debug)]
pub struct PingFrame {
    pub r#type: String,
    pub timestamp: String,
}

/// Mac → Relay: 心跳 pong 帧。
#[derive(Serialize, Deserialize, Debug)]
pub struct PongFrame {
    pub r#type: String,
    pub timestamp: String,
}

/// 双向: 错误通知帧。
#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorFrame {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub message: String,
}

/// 标签枚举：通过 type 字段自动反序列化 incoming WS 帧。
///
/// 用于 Bridge → Relay 方向的消息解析。新增消息类型需在此添加变体。
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum IncomingFrame {
    #[serde(rename = "register")]
    Register(RegisterFrame),
    #[serde(rename = "proxy_response")]
    ProxyResponse(ProxyResponse),
    #[serde(rename = "pong")]
    Pong(PongFrame),
    #[serde(rename = "error")]
    Error(ErrorFrame),
}
