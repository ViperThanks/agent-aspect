//! 环境变量兼容层 — 新名 AGENT_ASPECT_* 优先，旧名 CHECKPOINT_* 回退。
//!
//! 读取优先级：AGENT_ASPECT_* → CHECKPOINT_* → 调用方默认值。
//! 写入/文档只用新名。

/// 读取环境变量：新名优先，旧名回退，均不存在返回 None。
pub fn env_var(new: &str, legacy: &str) -> Option<String> {
    std::env::var(new)
        .ok()
        .or_else(|| std::env::var(legacy).ok())
}

/// 读取环境变量：新名优先，旧名回退，均不存在返回 default。
pub fn env_var_or(new: &str, legacy: &str, default: String) -> String {
    env_var(new, legacy).unwrap_or(default)
}

/// 检查环境变量是否设置（新名或旧名）。
pub fn env_var_is_set(new: &str, legacy: &str) -> bool {
    std::env::var(new).is_ok() || std::env::var(legacy).is_ok()
}
