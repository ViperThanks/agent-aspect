//! Job 终态写入器 — 统一所有 job 完成信号的写入入口。
//!
//! 架构角色：将散落在 jobs.rs 各处的裸字符串终态写入收敛为结构化的
//! `CompletionSignal` → DB status/completed_reason/failure_reason 更新。
//!
//! 核心不变量：
//! - 所有 job 终态写入都经过 `CompletionSink::apply()`，不再直接调用
//!   `store.update_job_finished_with_completed_reason()`
//! - stop hook 产出 `succeeded` + `completed_reason=stop_hook`
//! - process exit 0 产出 `succeeded` + `completed_reason=process_exit`
//! - process exit non-zero 产出 `failed` + `completed_reason=process_exit_nonzero`
//! - scanner timeout 产出 `failed` + `completed_reason=scanner_timeout`
//! - manual cancel 保持 `cancelled`
//! - apply 后自动关闭对应的 completion observer 并 SSE 广播

use agent_aspect_core::audit::AuditStore;
use agent_aspect_core::error::AgentAspectResult;
use agent_aspect_core::lifecycle::{
    CompletionAuthority, CompletionOutcome, CompletionSignal, CompletionSignalKind,
};
use std::sync::{Arc, Mutex};

use crate::sse::{self, SharedBroadcaster};

/// Job 终态写入器 — 所有终态信号的统一消费入口。
///
/// 持有共享的 AuditStore 和 SSE broadcaster，
/// 将 `CompletionSignal` 映射为 job DB 行更新 + observer 关闭 + SSE 广播。
#[derive(Clone)]
pub struct CompletionSink {
    store: Arc<Mutex<AuditStore>>,
    broadcaster: SharedBroadcaster,
}

impl CompletionSink {
    /// 创建 CompletionSink。
    pub fn new(store: Arc<Mutex<AuditStore>>, broadcaster: SharedBroadcaster) -> Self {
        Self { store, broadcaster }
    }

    /// 暴露内部 broadcaster 引用 — exec_job 内部仍需 broadcaster 做日志流推送。
    pub fn broadcaster(&self) -> SharedBroadcaster {
        self.broadcaster.clone()
    }

    /// spawn 失败的 SSE 广播 — spawn 失败不走 apply（job 还没真正开始）。
    pub fn broadcast_spawn_failed(&self, job_id: &str) {
        self.broadcaster.lock().unwrap().broadcast(sse::SseEvent {
            event_type: "job_status".to_string(),
            data: serde_json::json!({
                "job_id": job_id,
                "status": "failed",
                "failure_reason": "spawn failed",
            })
            .to_string(),
        });
    }

    /// 应用 completion signal 到 job — 唯一的终态写入入口。
    ///
    /// 流程：
    /// 1. 根据 signal.outcome 决定 job 新状态
    /// 2. 更新 jobs 表 status/completed_reason/failure_reason
    /// 3. 关闭对应的 completion observer
    /// 4. SSE 广播 job 状态变更
    ///
    /// 不处理 Running / MaybeIdle — 这两个 outcome 不写终态。
    pub fn apply(&self, signal: &CompletionSignal) -> AgentAspectResult<()> {
        let completed_reason = canonical_completed_reason(signal);
        let (job_status, failure_reason) = match signal.outcome {
            CompletionOutcome::Running | CompletionOutcome::MaybeIdle => {
                return Ok(());
            }
            CompletionOutcome::Completed => ("succeeded", None::<String>),
            CompletionOutcome::Failed => ("failed", Some(signal.reason.clone())),
            CompletionOutcome::TimedOut => ("failed", Some(signal.reason.clone())),
            CompletionOutcome::Cancelled => ("cancelled", None::<String>),
        };

        // 1. 更新 jobs 表
        let now = &signal.observed_at;
        {
            let store = self.store.lock().unwrap();
            store.update_job_finished_with_completed_reason(
                signal.job_id.as_deref().unwrap_or(""),
                job_status,
                now,
                None,
                failure_reason.as_deref(),
                Some(completed_reason),
            )?;
        }

        // 2. 关闭对应的 completion observer（如果有）
        self.close_observer(signal);

        // 3. SSE 广播
        self.broadcast_job_status(signal, job_status);

        Ok(())
    }

    /// 关闭与 job 关联的 completion observer。
    ///
    /// 根据 signal.kind 选择 observer 终态标记方法。
    /// 如果 observer 不存在（非 agent_prompt job），静默忽略。
    fn close_observer(&self, signal: &CompletionSignal) {
        let job_id = match signal.job_id {
            Some(ref id) => id.as_str(),
            None => return,
        };

        let store = self.store.lock().unwrap();

        let observer = match store.get_observer_by_job_id(job_id) {
            Ok(Some(o)) => o,
            _ => return,
        };

        let signal_str = format!("{:?}", signal.kind);
        let authority_str = format!("{:?}", signal.authority);

        match signal.outcome {
            CompletionOutcome::Completed => {
                let _ = store.mark_observer_completed(
                    &observer.id,
                    &signal_str,
                    &authority_str,
                    &signal.reason,
                    &signal.observed_at,
                );
            }
            CompletionOutcome::Failed | CompletionOutcome::TimedOut => {
                if signal.kind == CompletionSignalKind::ScannerTimeout {
                    let _ = store.mark_observer_timed_out(
                        &observer.id,
                        &signal.reason,
                        &signal.observed_at,
                    );
                } else {
                    let _ = store.mark_observer_failed(
                        &observer.id,
                        &signal_str,
                        &authority_str,
                        &signal.reason,
                        &signal.observed_at,
                    );
                }
            }
            CompletionOutcome::Cancelled => {
                let _ = store.mark_observer_cancelled(
                    &observer.id,
                    &signal.reason,
                    &signal.observed_at,
                );
            }
            CompletionOutcome::Running | CompletionOutcome::MaybeIdle => {}
        }
    }

    /// SSE 广播 job 状态变更。
    fn broadcast_job_status(&self, signal: &CompletionSignal, status: &str) {
        let job_id = match signal.job_id {
            Some(ref id) => id.clone(),
            None => return,
        };

        self.broadcaster.lock().unwrap().broadcast(sse::SseEvent {
            event_type: "job_status".to_string(),
            data: serde_json::json!({
                "job_id": job_id,
                "status": status,
                "failure_reason": if status == "failed" { Some(&signal.reason) } else { None },
                "completed_reason": canonical_completed_reason(signal),
                "completion_signal": format!("{:?}", signal.kind),
                "completion_authority": format!("{:?}", signal.authority),
            })
            .to_string(),
        });
    }
}

/// 将结构化 signal 映射为 jobs.completed_reason 的稳定枚举值。
pub(crate) fn canonical_completed_reason(signal: &CompletionSignal) -> &'static str {
    match signal.outcome {
        CompletionOutcome::Cancelled => "cancelled",
        CompletionOutcome::TimedOut => "scanner_timeout",
        CompletionOutcome::Failed => match signal.kind {
            CompletionSignalKind::ProcessExit => "process_exit_nonzero",
            CompletionSignalKind::ScannerTimeout => "scanner_timeout",
            _ => "failed",
        },
        CompletionOutcome::Completed => match signal.kind {
            CompletionSignalKind::StopHook => "stop_hook",
            CompletionSignalKind::ProcessExit => "process_exit",
            _ => "completed",
        },
        CompletionOutcome::Running | CompletionOutcome::MaybeIdle => "running",
    }
}

/// 根据 exit status 构建 process exit signal。
///
/// exit 0 → Completed + process_exit
/// exit non-zero → Failed + process_exit_nonzero
pub fn signal_from_exit_status(
    exit_status: &std::process::ExitStatus,
    job_id: &str,
    observed_at: &str,
) -> CompletionSignal {
    let agent = agent_aspect_core::event::AgentId::ClaudeCode;

    if exit_status.success() {
        CompletionSignal {
            kind: CompletionSignalKind::ProcessExit,
            authority: CompletionAuthority::Authoritative,
            outcome: CompletionOutcome::Completed,
            agent,
            job_id: Some(job_id.to_string()),
            workflow_id: None,
            workflow_step_id: None,
            conversation_id: None,
            reason: "process_exit".to_string(),
            observed_at: observed_at.to_string(),
        }
    } else {
        let failure_msg = match exit_status.code() {
            Some(c) => format!("[aspect-process] exit code {c}"),
            None => "[aspect-process] exit without status code".to_string(),
        };
        CompletionSignal {
            kind: CompletionSignalKind::ProcessExit,
            authority: CompletionAuthority::Authoritative,
            outcome: CompletionOutcome::Failed,
            agent,
            job_id: Some(job_id.to_string()),
            workflow_id: None,
            workflow_step_id: None,
            conversation_id: None,
            reason: failure_msg,
            observed_at: observed_at.to_string(),
        }
    }
}

/// 为无 exit status 的终止（被信号杀死等）构建 signal。
pub fn signal_for_killed(is_cancelled: bool, job_id: &str, observed_at: &str) -> CompletionSignal {
    let agent = agent_aspect_core::event::AgentId::ClaudeCode;

    if is_cancelled {
        CompletionSignal {
            kind: CompletionSignalKind::ManualCancel,
            authority: CompletionAuthority::Authoritative,
            outcome: CompletionOutcome::Cancelled,
            agent,
            job_id: Some(job_id.to_string()),
            workflow_id: None,
            workflow_step_id: None,
            conversation_id: None,
            reason: "cancelled by user".to_string(),
            observed_at: observed_at.to_string(),
        }
    } else {
        CompletionSignal {
            kind: CompletionSignalKind::ScannerTimeout,
            authority: CompletionAuthority::Authoritative,
            outcome: CompletionOutcome::Failed,
            agent,
            job_id: Some(job_id.to_string()),
            workflow_id: None,
            workflow_step_id: None,
            conversation_id: None,
            reason: "[aspect-process] killed without exit status".to_string(),
            observed_at: observed_at.to_string(),
        }
    }
}
