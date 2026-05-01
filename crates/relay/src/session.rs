//! WebSocket 会话注册表 — 管理 Mac Bridge 的活跃连接和待处理请求。
//!
//! 职责：维护 sid → RelaySession 映射，提供请求发送/完成/失败操作，
//! 支持同 sid 重连时踢掉旧连接。
//!
//! 架构角色：Relay 的核心状态管理。所有代理请求通过此注册表路由到对应的
//! Mac Bridge WebSocket 连接。
//!
//! 不变量：
//! - 每个 sid 最多有一个活跃 session（新连接踢掉旧连接）。
//! - connection_id 用于区分同 sid 的不同连接实例，防止旧连接清理新连接。
//! - pending_requests 使用 oneshot channel，每个 request_id 只能被完成或失败一次。

use crate::protocol::ProxyResponse;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};

/// 每个 sid 允许的最大待处理请求数。超出时拒绝新请求，防止内存耗尽。
const MAX_PENDING_REQUESTS: usize = 100;

/// 待处理响应的发送端。完成时发送 Ok(response)，失败时发送 Err(error_msg)。
type PendingResponse = oneshot::Sender<Result<ProxyResponse, String>>;

/// 一个 Mac Bridge 的 WebSocket 会话。
pub struct RelaySession {
    /// 唯一连接 ID，用于区分同 sid 的不同连接实例（防止旧连接误删新连接）。
    pub connection_id: String,
    /// WebSocket 写通道：通过此 sender 向 Bridge 推送消息。
    pub ws_tx: mpsc::Sender<String>,
    /// 连接建立时间（RFC3339）。
    pub connected_at: String,
    /// 待处理请求：request_id → oneshot sender。
    /// 代理请求发送后在此等待 Bridge 返回 ProxyResponse。
    pub pending_requests: HashMap<String, PendingResponse>,
    /// shutdown 信号发送端。Drop 时通知 WS handler 退出（用于踢掉旧连接）。
    pub shutdown_tx: oneshot::Sender<()>,
}

/// 会话注册表：sid → RelaySession。
///
/// 所有操作需要 &mut self，外部通过 Arc<Mutex<SessionRegistry>> 保护。
pub struct SessionRegistry {
    /// sid → 活跃的 Bridge 会话。
    sessions: HashMap<String, RelaySession>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 注册新的 Mac Bridge 会话。
    ///
    /// 若该 sid 已有旧会话，先踢掉旧连接（drop shutdown_tx 触发旧 WS handler 退出）。
    /// 返回 shutdown receiver（用于监听被踢信号）和 connection_id。
    pub fn register(
        &mut self,
        sid: String,
        ws_tx: mpsc::Sender<String>,
    ) -> (oneshot::Receiver<()>, String) {
        if let Some(old) = self.sessions.remove(&sid) {
            eprintln!(
                "relay: evicting old session for sid {}...",
                &sid[..8.min(sid.len())]
            );
            // Drop old shutdown_tx to close the old WS
            let _ = old;
        }
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let connection_id = uuid::Uuid::now_v7().to_string();
        self.sessions.insert(
            sid.clone(),
            RelaySession {
                connection_id: connection_id.clone(),
                ws_tx,
                connected_at: chrono::Utc::now().to_rfc3339(),
                pending_requests: HashMap::new(),
                shutdown_tx,
            },
        );
        (shutdown_rx, connection_id)
    }

    /// 无条件注销会话（不检查 connection_id）。
    ///
    /// 用于 handle_unregister 等明确要求删除的场景。
    pub fn unregister(&mut self, sid: &str) {
        if self.sessions.remove(sid).is_some() {
            eprintln!(
                "relay: session unregistered for sid {}...",
                &sid[..8.min(sid.len())]
            );
        }
    }

    /// 仅当 connection_id 匹配时才注销会话。
    ///
    /// 防止旧连接的 WS handler 在退出时误删新连接。返回是否执行了注销。
    pub fn unregister_if_current(&mut self, sid: &str, connection_id: &str) -> bool {
        let is_current = self
            .sessions
            .get(sid)
            .map(|s| s.connection_id == connection_id)
            .unwrap_or(false);
        if is_current {
            self.unregister(sid);
            true
        } else {
            false
        }
    }

    /// 检查指定 sid 是否有活跃连接。
    pub fn is_online(&self, sid: &str) -> bool {
        self.sessions.contains_key(sid)
    }

    /// 获取指定 sid 的 WS 写通道引用（用于心跳发送）。
    pub fn get_sender(&self, sid: &str) -> Option<&mpsc::Sender<String>> {
        self.sessions.get(sid).map(|s| &s.ws_tx)
    }

    /// 向指定 sid 的 Bridge 发送代理请求，返回 oneshot receiver 等待响应。
    ///
    /// 使用 try_send（非阻塞），若通道满则返回错误。
    /// 拒绝超出 MAX_PENDING_REQUESTS 的请求，防止内存耗尽。
    pub fn send_request(
        &mut self,
        sid: &str,
        request_id: String,
        message: String,
    ) -> Result<oneshot::Receiver<Result<ProxyResponse, String>>, String> {
        let session = self
            .sessions
            .get_mut(sid)
            .ok_or_else(|| "mac_offline".to_string())?;

        if session.pending_requests.len() >= MAX_PENDING_REQUESTS {
            return Err(format!(
                "too_many_pending: {} (limit {MAX_PENDING_REQUESTS})",
                session.pending_requests.len()
            ));
        }

        // 注册 oneshot channel，Bridge 响应到达时通过此 channel 通知调用者
        let (tx, rx) = oneshot::channel();
        session.pending_requests.insert(request_id.clone(), tx);

        if let Err(e) = session.ws_tx.try_send(message) {
            session.pending_requests.remove(&request_id);
            return Err(format!("send_failed: {e}"));
        }

        Ok(rx)
    }

    /// 用 Bridge 的成功响应完成指定的 pending request。
    ///
    /// 返回是否成功匹配并完成。
    pub fn complete_request(
        &mut self,
        sid: &str,
        request_id: &str,
        response: ProxyResponse,
    ) -> bool {
        if let Some(session) = self.sessions.get_mut(sid) {
            if let Some(tx) = session.pending_requests.remove(request_id) {
                tx.send(Ok(response)).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 标记单个 pending request 为失败（用于请求级超时）。
    pub fn fail_pending_request(&mut self, sid: &str, request_id: &str, error: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(sid) {
            if let Some(tx) = session.pending_requests.remove(request_id) {
                tx.send(Err(error.to_string())).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 标记指定 sid 的所有 pending request 为失败（用于 Bridge 断开连接）。
    pub fn fail_pending(&mut self, sid: &str, error: &str) {
        if let Some(session) = self.sessions.get_mut(sid) {
            for (_, tx) in session.pending_requests.drain() {
                let _ = tx.send(Err(error.to_string()));
            }
        }
    }

    /// 仅当 connection_id 匹配时，标记该会话的所有 pending request 为失败。
    ///
    /// 防止旧连接退出时误失败新连接上的请求。
    pub fn fail_pending_if_current(&mut self, sid: &str, connection_id: &str, error: &str) -> bool {
        if let Some(session) = self.sessions.get_mut(sid) {
            if session.connection_id == connection_id {
                for (_, tx) in session.pending_requests.drain() {
                    let _ = tx.send(Err(error.to_string()));
                }
                return true;
            }
        }
        false
    }
}

/// 共享注册表类型：Arc<Mutex<SessionRegistry>>。
pub type SharedRegistry = Arc<Mutex<SessionRegistry>>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn setup() -> SharedRegistry {
        Arc::new(Mutex::new(SessionRegistry::new()))
    }

    fn fake_register(reg: &SharedRegistry, sid: &str) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(32);
        reg.blocking_lock().register(sid.to_string(), tx);
        rx
    }

    #[test]
    fn register_and_lookup() {
        let reg = setup();
        fake_register(&reg, "sid-abc");
        assert!(reg.blocking_lock().is_online("sid-abc"));
        assert!(!reg.blocking_lock().is_online("no-such-sid"));
    }

    #[test]
    fn timeout_only_fails_specific_request() {
        let reg = setup();
        fake_register(&reg, "sid-1");

        let mut locked = reg.blocking_lock();

        let (tx1, rx1) = oneshot::channel();
        let (tx2, _rx2) = oneshot::channel();
        locked
            .sessions
            .get_mut("sid-1")
            .unwrap()
            .pending_requests
            .insert("req-1".to_string(), tx1);
        locked
            .sessions
            .get_mut("sid-1")
            .unwrap()
            .pending_requests
            .insert("req-2".to_string(), tx2);

        assert!(locked.fail_pending_request("sid-1", "req-1", "timeout"));

        assert!(matches!(
            rx1.blocking_recv(),
            Ok(Err(ref e)) if e == "timeout"
        ));

        assert!(
            locked
                .sessions
                .get("sid-1")
                .unwrap()
                .pending_requests
                .contains_key("req-2")
        );
        drop(locked);

        assert!(reg.blocking_lock().complete_request(
            "sid-1",
            "req-2",
            ProxyResponse {
                r#type: "proxy_response".to_string(),
                request_id: "req-2".to_string(),
                status: 200,
                headers: HashMap::new(),
                body: "{}".to_string(),
            }
        ));
    }

    #[test]
    fn disconnect_fails_all_pending() {
        let reg = setup();
        fake_register(&reg, "sid-1");

        let mut locked = reg.blocking_lock();

        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        locked
            .sessions
            .get_mut("sid-1")
            .unwrap()
            .pending_requests
            .insert("r1".to_string(), tx1);
        locked
            .sessions
            .get_mut("sid-1")
            .unwrap()
            .pending_requests
            .insert("r2".to_string(), tx2);

        locked.fail_pending("sid-1", "mac_disconnected");

        assert!(matches!(
            rx1.blocking_recv(),
            Ok(Err(ref e)) if e == "mac_disconnected"
        ));
        assert!(matches!(
            rx2.blocking_recv(),
            Ok(Err(ref e)) if e == "mac_disconnected"
        ));

        assert!(
            locked
                .sessions
                .get("sid-1")
                .unwrap()
                .pending_requests
                .is_empty()
        );
    }

    #[test]
    fn old_connection_cannot_unregister_new_reconnect() {
        let reg = setup();

        let (old_tx, _old_rx) = mpsc::channel(32);
        let (_, old_connection_id) = reg.blocking_lock().register("sid-1".to_string(), old_tx);

        let (new_tx, _new_rx) = mpsc::channel(32);
        let (_, new_connection_id) = reg.blocking_lock().register("sid-1".to_string(), new_tx);

        let mut locked = reg.blocking_lock();
        assert!(!locked.unregister_if_current("sid-1", &old_connection_id));
        assert!(locked.is_online("sid-1"));
        assert!(locked.unregister_if_current("sid-1", &new_connection_id));
        assert!(!locked.is_online("sid-1"));
    }
}
