//! 完成策略核心类型 — 统一 stop hook / process exit / scanner timeout 等完成信号。
//!
//! 在 M47 之前，job 完成判定散落在 bridge jobs.rs 各处，用裸字符串表示完成原因。
//! 本模块定义结构化的 `CompletionSignal`，让 scanner / stop hook / process exit
//! 都产出统一信号，由 `CompletionSink` 消费。
//!
//! 关键不变量：
//! - cancel 优先级 > process exit > stop hook > scanner timeout > scanner idle
//! - `ScannerIdle` 不是完成信号，只产生 `MaybeIdle` 状态
//! - `CompletionAuthority` 区分 Authoritative（确定性终态）、Inferred（推断）、Informational（通知）

use crate::event::AgentId;
use serde::{Deserialize, Serialize};

/// 完成信号来源 — 标识信号由哪个子系统产生。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CompletionSignalKind {
    /// Stop hook 触发 — agent 会话正常结束。
    StopHook,
    /// 进程退出 — 通过 waitpid 检测到子进程终止。
    ProcessExit,
    /// Scanner 硬超时 — 超过 hard_deadline_at 仍未收到终态信号。
    ScannerTimeout,
    /// Scanner idle 检测 — 超过 idle_deadline_at 无 transcript 变化。
    ScannerIdle,
    /// Transcript 有增量 — 有新行写入，表示 agent 仍在工作。
    TranscriptDelta,
    /// 用户手动取消 — 通过 API 或 UI 触发。
    ManualCancel,
}

/// 完成信号权威度 — 区分确定性终态和推断性判断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CompletionAuthority {
    /// 确定性终态（stop hook / process exit / scanner timeout / manual cancel）。
    Authoritative,
    /// 推断性判断（scanner idle 推断 agent 可能已停止）。
    Inferred,
    /// 通知性信号（transcript delta，不改变终态判断）。
    Informational,
}

/// 完成结果 — 对应 observer 的终态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CompletionOutcome {
    /// 正常完成（exit 0 / stop hook / scanner 观察到终止）。
    Completed,
    /// 失败（exit non-zero）。
    Failed,
    /// 超时（hard deadline 超出）。
    TimedOut,
    /// 取消（用户手动触发）。
    Cancelled,
    /// 仍在运行（transcript 有增量）。
    Running,
    /// 可能空闲（idle 超出但未达 hard deadline）。
    MaybeIdle,
}

/// 完成信号 — 一个观察源产出的结构化完成判定。
///
/// 每种信号源（stop hook / process exit / scanner）产出一个 `CompletionSignal`，
/// 由 `CompletionSink` 按优先级合并后写入 job/workflow 终态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSignal {
    /// 信号来源类型。
    pub kind: CompletionSignalKind,
    /// 信号权威度。
    pub authority: CompletionAuthority,
    /// 完成结果。
    pub outcome: CompletionOutcome,
    /// 产生此信号的 agent。
    pub agent: AgentId,
    /// 关联的 job ID（bridge 远程任务）。
    pub job_id: Option<String>,
    /// 关联的 workflow ID。
    pub workflow_id: Option<String>,
    /// 关联的 workflow step ID。
    pub workflow_step_id: Option<String>,
    /// 关联的 conversation ID。
    pub conversation_id: Option<String>,
    /// 人类可读的完成原因描述。
    pub reason: String,
    /// 信号观测时间（ISO 8601）。
    pub observed_at: String,
}

/// 完成策略配置 — 控制 scanner 行为和超时阈值。
///
/// 默认值适用于大多数 provider，各 provider adapter 可通过覆写调整。
/// `idle_timeout_secs` 和 `hard_timeout_secs` 的默认值需由调用方从 config 注入。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionPolicy {
    /// 是否启用 transcript scanner（默认 true）。
    pub scanner_enabled: bool,
    /// scanner 轮询间隔（秒）。
    pub poll_interval_secs: u64,
    /// idle 超时阈值（秒）— 基于 `last_activity_at` 滚动计算。
    pub idle_timeout_secs: u64,
    /// 硬超时阈值（秒）— 基于 `started_at` 固定，不因 transcript delta 延长。
    pub hard_timeout_secs: u64,
    /// stop hook 后等待进程退出的宽限期（秒）。
    pub stop_grace_secs: u64,
    /// 最大重试次数（保留字段，当前固定 1）。
    pub max_attempts: u32,
}

impl Default for CompletionPolicy {
    /// 默认策略：scanner 开启，5 秒轮询，3 秒 stop 宽限，单次执行。
    ///
    /// `idle_timeout_secs` 和 `hard_timeout_secs` 设为 0 表示"未配置"，
    /// 实际值由调用方从 `config.agent_prompt_timeout_secs` 注入。
    fn default() -> Self {
        Self {
            scanner_enabled: true,
            poll_interval_secs: 5,
            idle_timeout_secs: 0,
            hard_timeout_secs: 0,
            stop_grace_secs: 3,
            max_attempts: 1,
        }
    }
}
