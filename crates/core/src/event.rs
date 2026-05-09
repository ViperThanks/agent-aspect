//! 核心事件类型 — `UnifiedEvent`、`AgentId`、`Phase`、`Risk` 等。
//!
//! `UnifiedEvent` 是所有 provider 工具使用事件的统一归一化表示，
//! 由各 provider 的 normalize 函数生成，供规则引擎评估。
//!
//! `AgentId` 标识 AI provider，同时是 adapter 模式的分发键。

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Before,
    Mid,
    After,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::Mid => "mid",
            Self::After => "after",
        }
    }
}

impl Display for Phase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Agent hook 生命周期事件类型 — 标识 hook 触发的阶段。
///
/// 不同 AI agent 支持的 hook 事件集合不同。例如 Codex CLI 支持
/// PermissionRequest，而 Claude Code 不支持。`blocking()` 区分
/// 哪些事件需要同步等待策略决策（PreToolUse/PermissionRequest），
/// 哪些只是通知（SessionStart/PostToolUse 等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LifecycleEvent {
    /// 工具调用前 — 最核心的拦截点，所有 agent 都支持。
    PreToolUse,
    /// Codex CLI 权限请求 — 不同于 PreToolUse 的独立审批流。
    PermissionRequest,
    /// 工具调用后 — 用于审计记录，不参与规则评估。
    PostToolUse,
    /// 会话开始 — agent 进程启动时触发。
    SessionStart,
    /// 用户提交 prompt — 用户向 agent 发送消息时触发。
    UserPromptSubmit,
    /// 停止 — agent 会话结束时触发。
    Stop,
}

impl LifecycleEvent {
    /// 返回事件名的静态字符串表示。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PermissionRequest => "PermissionRequest",
            Self::PostToolUse => "PostToolUse",
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::Stop => "Stop",
        }
    }

    /// 该事件是否为阻塞型 — 需要同步等待策略决策。
    ///
    /// 只有 PreToolUse 和 PermissionRequest 会阻塞 agent 执行，
    /// 其余事件仅用于审计记录和状态追踪。
    pub fn blocking(&self) -> bool {
        matches!(self, Self::PreToolUse | Self::PermissionRequest)
    }
}

impl Display for LifecycleEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LifecycleEvent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PreToolUse" => Ok(Self::PreToolUse),
            "PermissionRequest" => Ok(Self::PermissionRequest),
            "PostToolUse" => Ok(Self::PostToolUse),
            "SessionStart" => Ok(Self::SessionStart),
            "UserPromptSubmit" => Ok(Self::UserPromptSubmit),
            "Stop" => Ok(Self::Stop),
            _ => Err(format!("unknown lifecycle event: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentId {
    ClaudeCode,
    CodexCli,
    GeminiCli,
    KimiCode,
    ZCode,
    Opencode,
}

impl AgentId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude_code",
            Self::CodexCli => "codex_cli",
            Self::GeminiCli => "gemini_cli",
            Self::KimiCode => "kimi_code",
            Self::ZCode => "z_code",
            Self::Opencode => "opencode",
        }
    }
}

impl Display for AgentId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude_code" | "claude" => Ok(Self::ClaudeCode),
            "codex_cli" | "codex" => Ok(Self::CodexCli),
            "gemini_cli" | "gemini" => Ok(Self::GeminiCli),
            "kimi_code" | "kimi" => Ok(Self::KimiCode),
            "z_code" | "z" => Ok(Self::ZCode),
            "opencode" => Ok(Self::Opencode),
            _ => Err(format!("unknown agent: {s}")),
        }
    }
}

/// 事件作用域 — 标记事件涉及的仓库、分支、文件模式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub file_pattern: Option<String>,
}

/// 风险等级 — 规则匹配后的评估结果，当前默认 Low。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

/// 工具调用输入 — 按 provider 差异归一化为统一字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_all: Option<bool>,
}

/// 统一事件 — 所有 provider 的工具使用事件归一化后的标准结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEvent {
    pub id: String,
    pub phase: Phase,
    #[serde(rename = "type")]
    pub event_type: String,
    pub agent: AgentId,
    pub tool_name: String,
    pub scope: Scope,
    pub risk: Risk,
    pub tool_input: ToolInput,
    pub timestamp: String,
}

impl UnifiedEvent {
    /// 便捷构造：Claude Code 事件。
    pub fn new_for_claude(
        phase: Phase,
        event_type: &str,
        tool_name: &str,
        tool_input: ToolInput,
    ) -> Self {
        Self::new_for_agent(
            AgentId::ClaudeCode,
            phase,
            event_type,
            tool_name,
            tool_input,
        )
    }

    /// 通用构造：生成 UUIDv7 作为 id、当前 UTC 时间戳。
    pub fn new_for_agent(
        agent: AgentId,
        phase: Phase,
        event_type: &str,
        tool_name: &str,
        tool_input: ToolInput,
    ) -> Self {
        let id = uuid::Uuid::now_v7().to_string();
        let ts = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            phase,
            event_type: event_type.to_string(),
            agent,
            tool_name: tool_name.to_string(),
            scope: Scope {
                repo: None,
                branch: None,
                file_pattern: tool_input.file_path.clone(),
            },
            risk: Risk::Low,
            tool_input,
            timestamp: ts,
        }
    }
}
