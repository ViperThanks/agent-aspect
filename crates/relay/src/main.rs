//! Relay 服务器入口。所有逻辑在 lib.rs 中，main 只做 tokio runtime 启动。

#[tokio::main]
async fn main() {
    checkpoint_relay::run_server().await;
}
