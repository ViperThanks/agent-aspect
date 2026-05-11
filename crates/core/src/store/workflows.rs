//! 工作流 DAO — 本地编排引擎的持久化层。
//!
//! workflows 表存储工作流定义，workflow_steps 存储每一步的执行参数和状态。
//! 状态机：draft → running → succeeded / failed / cancelled。
//! 每一步关联一个 job_id，通过 context_strategy 控制日志传递。

use crate::audit::AuditStore;
use crate::error::{AgentAspectError, AgentAspectResult};

const WORKFLOW_STEP_COLUMNS: &str =
    "id, workflow_id, step_order, kind, provider, project_path, prompt,
    context_strategy, context_from_step, status, job_id, created_at, finished_at,
    started_at, attempt_id, idempotency_key, attempt, max_attempts, retry_budget,
    heartbeat_at, hard_deadline_at, input_context_bytes, output_context_bytes,
    redaction_policy, failure_class, fallback_provider";

const WORKFLOW_ATTEMPT_COLUMNS: &str = "id, workflow_step_id, workflow_id, attempt,
    idempotency_key, job_id, status, failure_class, failure_reason, started_at, hard_deadline_at,
    finished_at, input_context_bytes, output_context_bytes, created_at, updated_at";

/// 工作流步骤状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl WorkflowStepStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

/// 工作流 step attempt 状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowAttemptStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
}

impl WorkflowAttemptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

/// 工作流失败分类枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowFailureClass {
    Timeout,
    ProcessFailed,
    SubmitFailed,
    Cancelled,
    BridgeRestart,
}

impl WorkflowFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::ProcessFailed => "process_failed",
            Self::SubmitFailed => "submit_failed",
            Self::Cancelled => "cancelled",
            Self::BridgeRestart => "bridge_restart",
        }
    }
}

/// 工作流行 — 对应 workflows 表所有列。
#[derive(Debug, Clone)]
pub struct WorkflowRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub advance_mode: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 工作流步骤行 — 对应 workflow_steps 表所有列。
#[derive(Debug, Clone)]
pub struct WorkflowStepRow {
    pub id: String,
    pub workflow_id: String,
    pub step_order: i64,
    pub kind: String,
    pub provider: Option<String>,
    pub project_path: Option<String>,
    pub prompt: String,
    pub context_strategy: String,
    pub context_from_step: Option<i64>,
    pub status: String,
    pub job_id: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub started_at: Option<String>,
    pub attempt_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub attempt: i64,
    pub max_attempts: i64,
    pub retry_budget: i64,
    pub heartbeat_at: Option<String>,
    pub hard_deadline_at: Option<String>,
    pub input_context_bytes: i64,
    pub output_context_bytes: i64,
    pub redaction_policy: String,
    pub failure_class: Option<String>,
    pub fallback_provider: Option<String>,
}

/// 工作流步骤尝试行 — 每次 retry 都产生一条独立记录。
#[derive(Debug, Clone)]
pub struct WorkflowStepAttemptRow {
    pub id: String,
    pub workflow_step_id: String,
    pub workflow_id: String,
    pub attempt: i64,
    pub idempotency_key: String,
    pub job_id: Option<String>,
    pub status: String,
    pub failure_class: Option<String>,
    pub failure_reason: Option<String>,
    pub started_at: String,
    pub hard_deadline_at: Option<String>,
    pub finished_at: Option<String>,
    pub input_context_bytes: i64,
    pub output_context_bytes: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 工作流推进信号行 — daemon stop hook 写入，bridge 消费。
#[derive(Debug, Clone)]
pub struct WorkflowAdvanceSignalRow {
    pub id: i64,
    pub workflow_id: String,
    pub step_id: Option<String>,
    pub agent: String,
    pub signal_type: String,
    pub consumed_at: Option<String>,
    pub created_at: String,
}

impl AuditStore {
    pub(crate) fn map_workflow_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkflowRow> {
        Ok(WorkflowRow {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            advance_mode: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    pub(crate) fn map_workflow_step_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<WorkflowStepRow> {
        Ok(WorkflowStepRow {
            id: row.get(0)?,
            workflow_id: row.get(1)?,
            step_order: row.get(2)?,
            kind: row.get(3)?,
            provider: row.get(4)?,
            project_path: row.get(5)?,
            prompt: row.get(6)?,
            context_strategy: row.get(7)?,
            context_from_step: row.get(8)?,
            status: row.get(9)?,
            job_id: row.get(10)?,
            created_at: row.get(11)?,
            finished_at: row.get(12)?,
            started_at: row.get(13)?,
            attempt_id: row.get(14)?,
            idempotency_key: row.get(15)?,
            attempt: row.get(16)?,
            max_attempts: row.get(17)?,
            retry_budget: row.get(18)?,
            heartbeat_at: row.get(19)?,
            hard_deadline_at: row.get(20)?,
            input_context_bytes: row.get(21)?,
            output_context_bytes: row.get(22)?,
            redaction_policy: row.get(23)?,
            failure_class: row.get(24)?,
            fallback_provider: row.get(25)?,
        })
    }

    pub(crate) fn map_workflow_step_attempt_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<WorkflowStepAttemptRow> {
        Ok(WorkflowStepAttemptRow {
            id: row.get(0)?,
            workflow_step_id: row.get(1)?,
            workflow_id: row.get(2)?,
            attempt: row.get(3)?,
            idempotency_key: row.get(4)?,
            job_id: row.get(5)?,
            status: row.get(6)?,
            failure_class: row.get(7)?,
            failure_reason: row.get(8)?,
            started_at: row.get(9)?,
            hard_deadline_at: row.get(10)?,
            finished_at: row.get(11)?,
            input_context_bytes: row.get(12)?,
            output_context_bytes: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    }

    /// 插入新工作流。
    pub fn insert_workflow(
        &self,
        id: &str,
        name: &str,
        description: &str,
        created_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "INSERT INTO workflows (id, name, description, status, advance_mode, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'draft', 'auto', ?4, ?4)",
                rusqlite::params![id, name, description, created_at],
            )
            .map_err(AgentAspectError::InsertWorkflow)?;
        Ok(())
    }

    /// 获取单个工作流。
    pub fn get_workflow(&self, id: &str) -> AgentAspectResult<Option<WorkflowRow>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, status, advance_mode, created_at, updated_at FROM workflows WHERE id = ?1")
            .map_err(AgentAspectError::QueryWorkflow)?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], Self::map_workflow_row)
            .map_err(AgentAspectError::QueryWorkflow)?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(AgentAspectError::QueryWorkflow)?)),
            None => Ok(None),
        }
    }

    /// 列出所有工作流，按创建时间倒序。
    pub fn list_workflows(&self, limit: i64, offset: i64) -> AgentAspectResult<Vec<WorkflowRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, description, status, advance_mode, created_at, updated_at
                 FROM workflows ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(AgentAspectError::QueryWorkflow)?;
        let rows = stmt
            .query_map(rusqlite::params![limit, offset], Self::map_workflow_row)
            .map_err(AgentAspectError::QueryWorkflow)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryWorkflow)
    }

    /// 更新工作流状态。允许从任何状态转移到新状态。
    pub fn update_workflow_status(
        &self,
        id: &str,
        status: &str,
        updated_at: &str,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflows SET status = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![id, status, updated_at],
            )
            .map_err(AgentAspectError::UpdateWorkflow)?;
        Ok(rows)
    }

    /// 更新工作流名称和描述。只允许 draft/failed/cancelled 状态的工作流。
    pub fn update_workflow(
        &self,
        id: &str,
        name: &str,
        description: &str,
        updated_at: &str,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflows SET name = ?2, description = ?3, updated_at = ?4
                 WHERE id = ?1 AND status IN ('draft', 'failed', 'cancelled')",
                rusqlite::params![id, name, description, updated_at],
            )
            .map_err(AgentAspectError::UpdateWorkflow)?;
        Ok(rows)
    }

    /// 更新工作流 advance_mode。允许任何状态。
    pub fn update_workflow_advance_mode(
        &self,
        id: &str,
        advance_mode: &str,
        updated_at: &str,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflows SET advance_mode = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![id, advance_mode, updated_at],
            )
            .map_err(AgentAspectError::UpdateWorkflow)?;
        Ok(rows)
    }

    /// 删除工作流及其所有步骤。只允许 draft/failed/cancelled 状态。
    /// 返回：Ok(true) = 已删除，Ok(false) = not found，Err = running。
    pub fn delete_workflow(&self, id: &str) -> AgentAspectResult<bool> {
        let wf = self.get_workflow(id)?;
        match wf {
            Some(w) if w.status == "running" => Err(AgentAspectError::WorkflowNotRunning),
            Some(_) => {
                let tx = self
                    .conn
                    .unchecked_transaction()
                    .map_err(AgentAspectError::UpdateWorkflow)?;
                tx.execute(
                    "DELETE FROM workflow_step_attempts WHERE workflow_id = ?1",
                    rusqlite::params![id],
                )
                .map_err(AgentAspectError::UpdateWorkflowStep)?;
                tx.execute(
                    "DELETE FROM workflow_steps WHERE workflow_id = ?1",
                    rusqlite::params![id],
                )
                .map_err(AgentAspectError::UpdateWorkflowStep)?;
                tx.execute("DELETE FROM workflows WHERE id = ?1", rusqlite::params![id])
                    .map_err(AgentAspectError::UpdateWorkflow)?;
                tx.commit().map_err(AgentAspectError::UpdateWorkflow)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Workflow Advance Signals
    // ═══════════════════════════════════════════════════════════════════

    /// 写入 workflow 推进信号（daemon stop hook → bridge）。
    pub fn insert_workflow_advance_signal(
        &self,
        workflow_id: &str,
        step_id: Option<&str>,
        agent: &str,
        signal_type: &str,
        created_at: &str,
    ) -> AgentAspectResult<i64> {
        self.conn
            .execute(
                "INSERT INTO workflow_advance_signals (workflow_id, step_id, agent, signal_type, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![workflow_id, step_id, agent, signal_type, created_at],
            )
            .map_err(AgentAspectError::InsertWorkflow)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 轮询未消费的推进信号（bridge 后台线程调用）。
    pub fn poll_workflow_advance_signals(
        &self,
        workflow_id: &str,
    ) -> AgentAspectResult<Vec<WorkflowAdvanceSignalRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, workflow_id, step_id, agent, signal_type, consumed_at, created_at
                 FROM workflow_advance_signals
                 WHERE workflow_id = ?1 AND consumed_at IS NULL
                 ORDER BY id ASC",
            )
            .map_err(AgentAspectError::QueryWorkflow)?;
        let rows = stmt
            .query_map(rusqlite::params![workflow_id], |row| {
                Ok(WorkflowAdvanceSignalRow {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    step_id: row.get(2)?,
                    agent: row.get(3)?,
                    signal_type: row.get(4)?,
                    consumed_at: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })
            .map_err(AgentAspectError::QueryWorkflow)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryWorkflow)
    }

    /// 消费推进信号（bridge resume 后标记已处理）。
    pub fn consume_workflow_advance_signal(
        &self,
        id: i64,
        consumed_at: &str,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_advance_signals SET consumed_at = ?2 WHERE id = ?1",
                rusqlite::params![id, consumed_at],
            )
            .map_err(AgentAspectError::UpdateWorkflow)?;
        Ok(rows)
    }

    /// 重新排序工作流步骤。step_orders 是 (step_id, new_order) 的列表。
    pub fn reorder_workflow_steps(
        &self,
        step_orders: &[(String, i64)],
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        for (step_id, new_order) in step_orders {
            tx.execute(
                "UPDATE workflow_steps SET step_order = ?2 WHERE id = ?1",
                rusqlite::params![step_id, new_order],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        }
        tx.commit().map_err(AgentAspectError::UpdateWorkflowStep)?;
        let _ = updated_at; // reserved for future use
        Ok(())
    }

    /// 插入工作流步骤。
    pub fn insert_workflow_step(
        &self,
        id: &str,
        workflow_id: &str,
        step_order: i64,
        kind: &str,
        provider: Option<&str>,
        project_path: Option<&str>,
        prompt: &str,
        context_strategy: &str,
        context_from_step: Option<i64>,
        created_at: &str,
    ) -> AgentAspectResult<()> {
        self.insert_workflow_step_with_ha(
            id,
            workflow_id,
            step_order,
            kind,
            provider,
            project_path,
            prompt,
            context_strategy,
            context_from_step,
            0,
            "basic",
            None,
            created_at,
        )
    }

    /// 插入带 HA 元数据的工作流步骤。
    ///
    /// `retry_budget` 表示失败后最多可自动重试的次数；`max_attempts = retry_budget + 1`。
    /// `idempotency_key` 以 workflow + step_order + attempt 固定生成，恢复时可识别同一次尝试。
    pub fn insert_workflow_step_with_ha(
        &self,
        id: &str,
        workflow_id: &str,
        step_order: i64,
        kind: &str,
        provider: Option<&str>,
        project_path: Option<&str>,
        prompt: &str,
        context_strategy: &str,
        context_from_step: Option<i64>,
        retry_budget: i64,
        redaction_policy: &str,
        fallback_provider: Option<&str>,
        created_at: &str,
    ) -> AgentAspectResult<()> {
        let retry_budget = retry_budget.clamp(0, 5);
        let max_attempts = retry_budget + 1;
        let attempt_id = format!("{id}:attempt:1");
        let idempotency_key = format!("{workflow_id}:{step_order}:1");
        let redaction_policy = match redaction_policy {
            "none" | "basic" => redaction_policy,
            _ => "basic",
        };
        self.conn
            .execute(
                "INSERT INTO workflow_steps
                 (id, workflow_id, step_order, kind, provider, project_path, prompt,
                  context_strategy, context_from_step, status, attempt_id, idempotency_key,
                  attempt, max_attempts, retry_budget, redaction_policy, fallback_provider, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10, ?11, 1, ?12, ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    id,
                    workflow_id,
                    step_order,
                    kind,
                    provider,
                    project_path,
                    prompt,
                    context_strategy,
                    context_from_step,
                    attempt_id,
                    idempotency_key,
                    max_attempts,
                    retry_budget,
                    redaction_policy,
                    fallback_provider,
                    created_at
                ],
            )
            .map_err(AgentAspectError::InsertWorkflowStep)?;
        Ok(())
    }

    /// 获取单个步骤。
    pub fn get_workflow_step(&self, id: &str) -> AgentAspectResult<Option<WorkflowStepRow>> {
        let sql = format!("SELECT {WORKFLOW_STEP_COLUMNS} FROM workflow_steps WHERE id = ?1");
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let mut rows = stmt
            .query_map(rusqlite::params![id], Self::map_workflow_step_row)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(AgentAspectError::QueryWorkflowStep)?)),
            None => Ok(None),
        }
    }

    /// 获取工作流的所有步骤，按 step_order 排序。
    pub fn get_workflow_steps(&self, workflow_id: &str) -> AgentAspectResult<Vec<WorkflowStepRow>> {
        let sql = format!(
            "SELECT {WORKFLOW_STEP_COLUMNS} FROM workflow_steps
             WHERE workflow_id = ?1 ORDER BY step_order ASC"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let rows = stmt
            .query_map(rusqlite::params![workflow_id], Self::map_workflow_step_row)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryWorkflowStep)
    }

    /// 更新步骤状态。
    pub fn update_workflow_step_status(
        &self,
        id: &str,
        status: &str,
        finished_at: Option<&str>,
    ) -> AgentAspectResult<usize> {
        let now = chrono::Utc::now().to_rfc3339();
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps
                 SET status = ?2,
                     started_at = CASE WHEN ?2 = 'running' THEN COALESCE(started_at, ?4) ELSE started_at END,
                     heartbeat_at = CASE WHEN ?2 = 'running' THEN ?4 ELSE heartbeat_at END,
                     finished_at = COALESCE(?3, finished_at)
                 WHERE id = ?1
                   AND (
                     status = ?2
                     OR (status = 'pending' AND ?2 IN ('running','succeeded','failed','cancelled','skipped'))
                     OR (status = 'running' AND ?2 IN ('succeeded','failed','cancelled','running'))
                     OR (status IN ('failed','cancelled','skipped') AND ?2 = 'pending')
                   )",
                rusqlite::params![id, status, finished_at, now],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// 更新步骤上下文体积指标。
    pub fn update_workflow_step_context_metrics(
        &self,
        id: &str,
        input_context_bytes: i64,
        output_context_bytes: Option<i64>,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps
                 SET input_context_bytes = ?2,
                     output_context_bytes = COALESCE(?3, output_context_bytes)
                 WHERE id = ?1",
                rusqlite::params![id, input_context_bytes, output_context_bytes],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// 开始一次 step attempt，并把 step 当前态推进到 running。
    ///
    /// attempts 表保留每次运行证据；workflow_steps 只保存当前 attempt 摘要。
    pub fn begin_workflow_step_attempt(
        &self,
        step: &WorkflowStepRow,
        input_context_bytes: i64,
        hard_deadline_at: Option<&str>,
        started_at: &str,
    ) -> AgentAspectResult<String> {
        let attempt = step.attempt.max(1);
        let attempt_id = format!("{}:attempt:{attempt}", step.id);
        let idempotency_key = format!("{}:{}:{attempt}", step.workflow_id, step.step_order);
        let running = WorkflowAttemptStatus::Running.as_str();
        self.conn
            .execute(
                "INSERT OR IGNORE INTO workflow_step_attempts
                 (id, workflow_step_id, workflow_id, attempt, idempotency_key, status,
                  started_at, hard_deadline_at, input_context_bytes, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?7, ?7)",
                rusqlite::params![
                    attempt_id,
                    step.id,
                    step.workflow_id,
                    attempt,
                    idempotency_key,
                    running,
                    started_at,
                    hard_deadline_at,
                    input_context_bytes,
                ],
            )
            .map_err(AgentAspectError::InsertWorkflowStep)?;
        self.conn
            .execute(
                "UPDATE workflow_steps
                 SET status = 'running',
                     started_at = COALESCE(started_at, ?5),
                     heartbeat_at = ?5,
                     hard_deadline_at = ?6,
                     finished_at = NULL,
                     attempt_id = ?2,
                     idempotency_key = ?3,
                     input_context_bytes = ?4,
                     failure_class = NULL
                 WHERE id = ?1 AND status IN ('pending','failed')",
                rusqlite::params![
                    step.id,
                    attempt_id,
                    idempotency_key,
                    input_context_bytes,
                    started_at,
                    hard_deadline_at,
                ],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(attempt_id)
    }

    /// 把当前 step job_id 指向最新 attempt 的 job，同时写入 attempt 历史。
    pub fn set_workflow_step_current_job(
        &self,
        step_id: &str,
        attempt_id: &str,
        job_id: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE workflow_steps SET job_id = ?2 WHERE id = ?1",
                rusqlite::params![step_id, job_id],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        self.conn
            .execute(
                "UPDATE workflow_step_attempts
                 SET job_id = ?2, updated_at = ?3
                 WHERE id = ?1",
                rusqlite::params![attempt_id, job_id, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(())
    }

    /// 完成一次 step attempt。
    pub fn finish_workflow_step_attempt(
        &self,
        attempt_id: &str,
        status: WorkflowAttemptStatus,
        failure_class: Option<WorkflowFailureClass>,
        failure_reason: Option<&str>,
        finished_at: &str,
        output_context_bytes: i64,
    ) -> AgentAspectResult<usize> {
        let status = status.as_str();
        let failure_class = failure_class.map(WorkflowFailureClass::as_str);
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_step_attempts
                 SET status = ?2,
                     failure_class = ?3,
                     failure_reason = ?4,
                     finished_at = ?5,
                     output_context_bytes = ?6,
                     updated_at = ?5
                 WHERE id = ?1",
                rusqlite::params![
                    attempt_id,
                    status,
                    failure_class,
                    failure_reason,
                    finished_at,
                    output_context_bytes,
                ],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// 如果 retry_budget 仍有余量，把 step 推进到下一次 attempt 的 pending 状态。
    pub fn prepare_workflow_step_retry(
        &self,
        step_id: &str,
        timestamp: &str,
    ) -> AgentAspectResult<Option<i64>> {
        let step = match self.get_workflow_step(step_id)? {
            Some(step) => step,
            None => return Ok(None),
        };
        if step.attempt >= step.max_attempts {
            return Ok(None);
        }

        let next_attempt = step.attempt + 1;
        let attempt_id = format!("{step_id}:attempt:{next_attempt}");
        let idempotency_key = format!("{}:{}:{next_attempt}", step.workflow_id, step.step_order);
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps
                 SET status = 'pending',
                     job_id = NULL,
                     started_at = NULL,
                     finished_at = NULL,
                     heartbeat_at = NULL,
                     hard_deadline_at = NULL,
                     attempt = ?2,
                     attempt_id = ?3,
                     idempotency_key = ?4,
                     failure_class = NULL
                 WHERE id = ?1 AND status = 'failed'",
                rusqlite::params![step_id, next_attempt, attempt_id, idempotency_key],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        let _ = timestamp;
        if rows == 0 {
            Ok(None)
        } else {
            Ok(Some(next_attempt))
        }
    }

    /// 将失败步骤切换到 fallback provider，并开启新的 pending attempt。
    ///
    /// fallback 只消费一次：切换成功后清空 `fallback_provider`，防止同一步在多个 provider
    /// 之间来回跳转。调用方负责判定失败分类和 provider 能力。
    pub fn prepare_workflow_step_fallback(
        &self,
        step_id: &str,
        fallback_provider: &str,
        timestamp: &str,
    ) -> AgentAspectResult<Option<i64>> {
        let step = match self.get_workflow_step(step_id)? {
            Some(step) => step,
            None => return Ok(None),
        };
        let next_attempt = step.max_attempts + 1;
        let attempt_id = format!("{step_id}:attempt:{next_attempt}");
        let idempotency_key = format!("{}:{}:{next_attempt}", step.workflow_id, step.step_order);
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps
                 SET status = 'pending',
                     provider = ?2,
                     fallback_provider = NULL,
                     job_id = NULL,
                     started_at = NULL,
                     finished_at = NULL,
                     heartbeat_at = NULL,
                     hard_deadline_at = NULL,
                     attempt = ?3,
                     max_attempts = ?3,
                     attempt_id = ?4,
                     idempotency_key = ?5,
                     failure_class = NULL
                 WHERE id = ?1 AND status = 'failed'",
                rusqlite::params![
                    step_id,
                    fallback_provider,
                    next_attempt,
                    attempt_id,
                    idempotency_key
                ],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        let _ = timestamp;
        if rows == 0 {
            Ok(None)
        } else {
            Ok(Some(next_attempt))
        }
    }

    /// 准备一次新的 workflow run。
    ///
    /// 已有 attempt 历史的 step 会开启新的 attempt 区间，避免重新运行 failed workflow 时覆盖旧 job。
    /// 从未执行过的 pending step 保持 attempt 1。
    pub fn prepare_workflow_step_for_run(
        &self,
        step: &WorkflowStepRow,
    ) -> AgentAspectResult<usize> {
        let attempt_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM workflow_step_attempts WHERE workflow_step_id = ?1",
                rusqlite::params![step.id],
                |row| row.get(0),
            )
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let has_history = attempt_count > 0 || step.job_id.is_some();
        let next_attempt = if has_history {
            step.max_attempts + 1
        } else {
            step.attempt.max(1)
        };
        let max_attempts = if has_history {
            next_attempt + step.retry_budget
        } else {
            step.max_attempts.max(next_attempt)
        };
        let attempt_id = format!("{}:attempt:{next_attempt}", step.id);
        let idempotency_key = format!("{}:{}:{next_attempt}", step.workflow_id, step.step_order);
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps
                 SET status = 'pending',
                     job_id = NULL,
                     started_at = NULL,
                     finished_at = NULL,
                     heartbeat_at = NULL,
                     hard_deadline_at = NULL,
                     attempt = ?2,
                     max_attempts = ?3,
                     attempt_id = ?4,
                     idempotency_key = ?5,
                     failure_class = NULL,
                     input_context_bytes = 0,
                     output_context_bytes = 0
                 WHERE id = ?1",
                rusqlite::params![
                    step.id,
                    next_attempt,
                    max_attempts,
                    attempt_id,
                    idempotency_key
                ],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// 查询某个 step 的 attempt 历史。
    pub fn list_workflow_step_attempts(
        &self,
        step_id: &str,
    ) -> AgentAspectResult<Vec<WorkflowStepAttemptRow>> {
        let sql = format!(
            "SELECT {WORKFLOW_ATTEMPT_COLUMNS}
             FROM workflow_step_attempts
             WHERE workflow_step_id = ?1
             ORDER BY attempt ASC"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let rows = stmt
            .query_map(
                rusqlite::params![step_id],
                Self::map_workflow_step_attempt_row,
            )
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryWorkflowStep)
    }

    /// 批量查询一个 workflow 下的所有 step attempt。
    pub fn list_workflow_attempts(
        &self,
        workflow_id: &str,
    ) -> AgentAspectResult<Vec<WorkflowStepAttemptRow>> {
        let sql = format!(
            "SELECT {WORKFLOW_ATTEMPT_COLUMNS}
             FROM workflow_step_attempts
             WHERE workflow_id = ?1
             ORDER BY workflow_step_id ASC, attempt ASC"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let rows = stmt
            .query_map(
                rusqlite::params![workflow_id],
                Self::map_workflow_step_attempt_row,
            )
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryWorkflowStep)
    }

    /// 标记步骤失败分类，便于 recovery/retry/fallback 后续策略读取。
    pub fn update_workflow_step_failure_class(
        &self,
        id: &str,
        failure_class: WorkflowFailureClass,
    ) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps SET failure_class = ?2 WHERE id = ?1",
                rusqlite::params![id, failure_class.as_str()],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// 绑定步骤的 job_id（仅当当前值为空时更新）。
    pub fn update_workflow_step_job(&self, id: &str, job_id: &str) -> AgentAspectResult<bool> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps SET job_id = ?2 WHERE id = ?1 AND (job_id IS NULL OR job_id = '')",
                rusqlite::params![id, job_id],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows > 0)
    }

    /// 获取工作流中指定 step_order 的步骤。
    pub fn get_workflow_step_by_order(
        &self,
        workflow_id: &str,
        step_order: i64,
    ) -> AgentAspectResult<Option<WorkflowStepRow>> {
        let sql = format!(
            "SELECT {WORKFLOW_STEP_COLUMNS} FROM workflow_steps
             WHERE workflow_id = ?1 AND step_order = ?2"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let mut rows = stmt
            .query_map(
                rusqlite::params![workflow_id, step_order],
                Self::map_workflow_step_row,
            )
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(AgentAspectError::QueryWorkflowStep)?)),
            None => Ok(None),
        }
    }

    /// 获取工作流当前待执行的下一步（第一个 pending 状态的步骤）。
    pub fn get_next_pending_step(
        &self,
        workflow_id: &str,
    ) -> AgentAspectResult<Option<WorkflowStepRow>> {
        let sql = format!(
            "SELECT {WORKFLOW_STEP_COLUMNS} FROM workflow_steps
             WHERE workflow_id = ?1 AND status = 'pending'
             ORDER BY step_order ASC LIMIT 1"
        );
        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        let mut rows = stmt
            .query_map(rusqlite::params![workflow_id], Self::map_workflow_step_row)
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(AgentAspectError::QueryWorkflowStep)?)),
            None => Ok(None),
        }
    }

    /// 取消工作流中所有未完成的步骤。
    pub fn cancel_workflow_steps(&self, workflow_id: &str) -> AgentAspectResult<usize> {
        let rows = self
            .conn
            .execute(
                "UPDATE workflow_steps SET status = 'cancelled'
                 WHERE workflow_id = ?1 AND status IN ('pending', 'running')",
                rusqlite::params![workflow_id],
            )
            .map_err(AgentAspectError::UpdateWorkflowStep)?;
        Ok(rows)
    }

    /// Bridge 启动恢复：把失去内存 runner 的 running/paused workflow 收敛到 failed。
    ///
    /// 已成功的步骤保留；running 步骤标记 failed；pending 步骤标记 skipped。
    pub fn recover_stale_workflows(&self, timestamp: &str) -> AgentAspectResult<usize> {
        let ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM workflows WHERE status IN ('running','paused')")
                .map_err(AgentAspectError::QueryWorkflow)?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(AgentAspectError::QueryWorkflow)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(AgentAspectError::QueryWorkflow)?
        };

        for workflow_id in &ids {
            self.conn
                .execute(
                    "UPDATE workflow_steps
                     SET status = 'failed',
                         finished_at = COALESCE(finished_at, ?2),
                         heartbeat_at = ?2,
                         failure_class = 'bridge_restart'
                     WHERE workflow_id = ?1 AND status = 'running'",
                    rusqlite::params![workflow_id, timestamp],
                )
                .map_err(AgentAspectError::UpdateWorkflowStep)?;
            self.conn
                .execute(
                    "UPDATE workflow_step_attempts
                     SET status = 'failed',
                         failure_class = 'bridge_restart',
                         failure_reason = COALESCE(failure_reason, 'bridge restarted before workflow step completed'),
                         finished_at = COALESCE(finished_at, ?2),
                         updated_at = ?2
                     WHERE workflow_id = ?1 AND status = 'running'",
                    rusqlite::params![workflow_id, timestamp],
                )
                .map_err(AgentAspectError::UpdateWorkflowStep)?;
            self.conn
                .execute(
                    "UPDATE workflow_steps
                     SET status = 'skipped',
                         finished_at = COALESCE(finished_at, ?2)
                     WHERE workflow_id = ?1 AND status = 'pending'",
                    rusqlite::params![workflow_id, timestamp],
                )
                .map_err(AgentAspectError::UpdateWorkflowStep)?;
            self.update_workflow_status(workflow_id, "failed", timestamp)?;
        }

        Ok(ids.len())
    }

    /// 统计工作流总数。
    pub fn count_workflows(&self) -> AgentAspectResult<i64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM workflows", [], |row| row.get(0))
            .map_err(AgentAspectError::QueryWorkflow)?;
        Ok(count)
    }

    /// 统计工作流中各状态的步骤数。(total, succeeded, failed, pending, skipped)
    pub fn workflow_step_counts(
        &self,
        workflow_id: &str,
    ) -> AgentAspectResult<(i64, i64, i64, i64, i64)> {
        let (total, succeeded, failed, pending, skipped): (i64, i64, i64, i64, i64) = self
            .conn
            .query_row(
                "SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status IN ('failed', 'cancelled') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status IN ('pending', 'running') THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM workflow_steps WHERE workflow_id = ?1",
                rusqlite::params![workflow_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(AgentAspectError::QueryWorkflowStep)?;
        Ok((total, succeeded, failed, pending, skipped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_crud_lifecycle() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store
            .insert_workflow("wf1", "Test Workflow", "A test workflow", now)
            .unwrap();

        let wf = store.get_workflow("wf1").unwrap().unwrap();
        assert_eq!(wf.name, "Test Workflow");
        assert_eq!(wf.status, "draft");

        store.update_workflow_status("wf1", "running", now).unwrap();
        let wf = store.get_workflow("wf1").unwrap().unwrap();
        assert_eq!(wf.status, "running");
    }

    #[test]
    fn workflow_steps_crud() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                Some("claude_code"),
                Some("/tmp/proj"),
                "step 1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                Some("claude_code"),
                Some("/tmp/proj"),
                "step 2",
                "last_50_lines",
                Some(0),
                now,
            )
            .unwrap();

        let steps = store.get_workflow_steps("wf1").unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].step_order, 0);
        assert_eq!(steps[1].step_order, 1);
        assert_eq!(steps[1].context_strategy, "last_50_lines");
        assert_eq!(steps[1].context_from_step, Some(0));
    }

    #[test]
    fn workflow_step_ha_fields_have_defaults_and_metrics() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step_with_ha(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                Some("claude_code"),
                None,
                "step 1",
                "last_50_lines",
                None,
                2,
                "basic",
                None,
                now,
            )
            .unwrap();

        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.attempt, 1);
        assert_eq!(step.max_attempts, 3);
        assert_eq!(step.retry_budget, 2);
        assert_eq!(step.redaction_policy, "basic");
        assert_eq!(step.attempt_id.as_deref(), Some("s1:attempt:1"));
        assert_eq!(step.idempotency_key.as_deref(), Some("wf1:0:1"));

        store
            .update_workflow_step_context_metrics("s1", 128, Some(256))
            .unwrap();
        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.input_context_bytes, 128);
        assert_eq!(step.output_context_bytes, 256);
    }

    #[test]
    fn workflow_step_fallback_switches_provider_once() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step_with_ha(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                Some("claude_code"),
                None,
                "step 1",
                "none",
                None,
                0,
                "basic",
                Some("codex_cli"),
                now,
            )
            .unwrap();
        store
            .update_workflow_step_status("s1", "failed", Some(now))
            .unwrap();

        assert_eq!(
            store
                .prepare_workflow_step_fallback("s1", "codex_cli", now)
                .unwrap(),
            Some(2)
        );
        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.status, "pending");
        assert_eq!(step.provider.as_deref(), Some("codex_cli"));
        assert_eq!(step.fallback_provider, None);
        assert_eq!(step.attempt, 2);
        assert_eq!(step.max_attempts, 2);
    }

    #[test]
    fn workflow_attempt_history_tracks_retry_without_overwriting_job() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";
        let later = "2026-05-04T10:01:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step_with_ha(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "step 1",
                "none",
                None,
                1,
                "basic",
                None,
                now,
            )
            .unwrap();

        let step = store.get_workflow_step("s1").unwrap().unwrap();
        let attempt_1 = store
            .begin_workflow_step_attempt(&step, 11, Some(later), now)
            .unwrap();
        store
            .set_workflow_step_current_job("s1", &attempt_1, "job-1")
            .unwrap();
        store
            .finish_workflow_step_attempt(
                &attempt_1,
                WorkflowAttemptStatus::Failed,
                Some(WorkflowFailureClass::Timeout),
                Some("timed out"),
                later,
                22,
            )
            .unwrap();
        store
            .update_workflow_step_status("s1", "failed", Some(later))
            .unwrap();

        assert_eq!(
            store.prepare_workflow_step_retry("s1", later).unwrap(),
            Some(2)
        );

        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.attempt, 2);
        assert_eq!(step.job_id, None);
        let attempt_2 = store
            .begin_workflow_step_attempt(&step, 33, Some(later), later)
            .unwrap();
        store
            .set_workflow_step_current_job("s1", &attempt_2, "job-2")
            .unwrap();
        store
            .finish_workflow_step_attempt(
                &attempt_2,
                WorkflowAttemptStatus::Succeeded,
                None,
                None,
                later,
                44,
            )
            .unwrap();

        let attempts = store.list_workflow_step_attempts("s1").unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].attempt, 1);
        assert_eq!(attempts[0].job_id.as_deref(), Some("job-1"));
        assert_eq!(attempts[0].status, "failed");
        assert_eq!(attempts[0].hard_deadline_at.as_deref(), Some(later));
        assert_eq!(attempts[1].attempt, 2);
        assert_eq!(attempts[1].job_id.as_deref(), Some("job-2"));
        assert_eq!(attempts[1].status, "succeeded");
    }

    #[test]
    fn prepare_workflow_step_for_run_starts_new_attempt_epoch() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step_with_ha(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "step 1",
                "none",
                None,
                1,
                "basic",
                None,
                now,
            )
            .unwrap();

        let step = store.get_workflow_step("s1").unwrap().unwrap();
        let attempt_1 = store
            .begin_workflow_step_attempt(&step, 0, Some(now), now)
            .unwrap();
        store
            .set_workflow_step_current_job("s1", &attempt_1, "job-1")
            .unwrap();
        store
            .finish_workflow_step_attempt(
                &attempt_1,
                WorkflowAttemptStatus::Failed,
                Some(WorkflowFailureClass::ProcessFailed),
                Some("failed"),
                now,
                0,
            )
            .unwrap();
        store
            .update_workflow_step_status("s1", "failed", Some(now))
            .unwrap();
        store.prepare_workflow_step_retry("s1", now).unwrap();

        let step = store.get_workflow_step("s1").unwrap().unwrap();
        let attempt_2 = store
            .begin_workflow_step_attempt(&step, 0, Some(now), now)
            .unwrap();
        store
            .set_workflow_step_current_job("s1", &attempt_2, "job-2")
            .unwrap();
        store
            .finish_workflow_step_attempt(
                &attempt_2,
                WorkflowAttemptStatus::Failed,
                Some(WorkflowFailureClass::ProcessFailed),
                Some("failed again"),
                now,
                0,
            )
            .unwrap();
        store
            .update_workflow_step_status("s1", "failed", Some(now))
            .unwrap();

        let exhausted = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(exhausted.attempt, 2);
        assert_eq!(exhausted.max_attempts, 2);

        store.prepare_workflow_step_for_run(&exhausted).unwrap();
        let rerun = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(rerun.status, "pending");
        assert_eq!(rerun.job_id, None);
        assert_eq!(rerun.attempt, 3);
        assert_eq!(rerun.max_attempts, 4);

        let attempts = store.list_workflow_attempts("wf1").unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].job_id.as_deref(), Some("job-1"));
        assert_eq!(attempts[1].job_id.as_deref(), Some("job-2"));
    }

    #[test]
    fn recover_stale_workflows_closes_running_and_pending_steps() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";
        let recovered_at = "2026-05-04T10:05:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store.update_workflow_status("wf1", "running", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "done",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "run",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s3",
                "wf1",
                2,
                "agent_prompt",
                None,
                None,
                "pending",
                "none",
                None,
                now,
            )
            .unwrap();

        store
            .update_workflow_step_status("s1", "succeeded", Some(now))
            .unwrap();
        store
            .update_workflow_step_status("s2", "running", None)
            .unwrap();

        let count = store.recover_stale_workflows(recovered_at).unwrap();
        assert_eq!(count, 1);

        let wf = store.get_workflow("wf1").unwrap().unwrap();
        assert_eq!(wf.status, "failed");
        let s1 = store.get_workflow_step("s1").unwrap().unwrap();
        let s2 = store.get_workflow_step("s2").unwrap().unwrap();
        let s3 = store.get_workflow_step("s3").unwrap().unwrap();
        assert_eq!(s1.status, "succeeded");
        assert_eq!(s2.status, "failed");
        assert_eq!(s2.failure_class.as_deref(), Some("bridge_restart"));
        assert_eq!(s3.status, "skipped");
    }

    #[test]
    fn next_pending_step_returns_first_pending() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "step 1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "step 2",
                "none",
                None,
                now,
            )
            .unwrap();

        let next = store.get_next_pending_step("wf1").unwrap().unwrap();
        assert_eq!(next.id, "s1");

        store
            .update_workflow_step_status("s1", "succeeded", Some(now))
            .unwrap();
        let next = store.get_next_pending_step("wf1").unwrap().unwrap();
        assert_eq!(next.id, "s2");

        store
            .update_workflow_step_status("s2", "succeeded", Some(now))
            .unwrap();
        let next = store.get_next_pending_step("wf1").unwrap();
        assert!(next.is_none());
    }

    #[test]
    fn cancel_workflow_steps_skips_completed() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "step 1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "step 2",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s3",
                "wf1",
                2,
                "agent_prompt",
                None,
                None,
                "step 3",
                "none",
                None,
                now,
            )
            .unwrap();

        store
            .update_workflow_step_status("s1", "succeeded", Some(now))
            .unwrap();
        let cancelled = store.cancel_workflow_steps("wf1").unwrap();
        assert_eq!(cancelled, 2);

        let s1 = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(s1.status, "succeeded");
        let s2 = store.get_workflow_step("s2").unwrap().unwrap();
        assert_eq!(s2.status, "cancelled");
        let s3 = store.get_workflow_step("s3").unwrap().unwrap();
        assert_eq!(s3.status, "cancelled");
    }

    #[test]
    fn step_counts() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "p1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "p2",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s3",
                "wf1",
                2,
                "agent_prompt",
                None,
                None,
                "p3",
                "none",
                None,
                now,
            )
            .unwrap();

        store
            .update_workflow_step_status("s1", "succeeded", Some(now))
            .unwrap();

        let (total, succeeded, failed, pending, skipped) =
            store.workflow_step_counts("wf1").unwrap();
        assert_eq!(total, 3);
        assert_eq!(succeeded, 1);
        assert_eq!(failed, 0);
        assert_eq!(pending, 2);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn step_counts_empty_workflow() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf-empty", "Empty", "", now).unwrap();
        let (total, succeeded, failed, pending, skipped) =
            store.workflow_step_counts("wf-empty").unwrap();
        assert_eq!(total, 0);
        assert_eq!(succeeded, 0);
        assert_eq!(failed, 0);
        assert_eq!(pending, 0);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn get_workflow_nonexistent() {
        let store = AuditStore::open_in_memory().unwrap();
        let result = store.get_workflow("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_workflow_step_nonexistent() {
        let store = AuditStore::open_in_memory().unwrap();
        let result = store.get_workflow_step("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_workflows_pagination() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        for i in 0..5 {
            store
                .insert_workflow(&format!("wf{i}"), &format!("WF {i}"), "", now)
                .unwrap();
        }

        let page1 = store.list_workflows(2, 0).unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = store.list_workflows(2, 2).unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = store.list_workflows(2, 4).unwrap();
        assert_eq!(page3.len(), 1);

        let page4 = store.list_workflows(2, 6).unwrap();
        assert_eq!(page4.len(), 0);
    }

    #[test]
    fn update_workflow_step_job_only_when_empty() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "p",
                "none",
                None,
                now,
            )
            .unwrap();

        // First bind succeeds
        assert!(store.update_workflow_step_job("s1", "job-1").unwrap());
        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.job_id.as_deref(), Some("job-1"));

        // Second bind is no-op (already bound)
        assert!(!store.update_workflow_step_job("s1", "job-2").unwrap());
        let step = store.get_workflow_step("s1").unwrap().unwrap();
        assert_eq!(step.job_id.as_deref(), Some("job-1"));
    }

    #[test]
    fn cancel_workflow_steps_mixed_statuses() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "p1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "p2",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s3",
                "wf1",
                2,
                "agent_prompt",
                None,
                None,
                "p3",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s4",
                "wf1",
                3,
                "agent_prompt",
                None,
                None,
                "p4",
                "none",
                None,
                now,
            )
            .unwrap();

        store
            .update_workflow_step_status("s1", "succeeded", Some(now))
            .unwrap();
        store
            .update_workflow_step_status("s2", "failed", Some(now))
            .unwrap();
        // s3, s4 are pending

        let cancelled = store.cancel_workflow_steps("wf1").unwrap();
        // Only pending and running are cancelled (s3, s4)
        assert_eq!(cancelled, 2);

        assert_eq!(
            store.get_workflow_step("s1").unwrap().unwrap().status,
            "succeeded"
        );
        assert_eq!(
            store.get_workflow_step("s2").unwrap().unwrap().status,
            "failed"
        );
        assert_eq!(
            store.get_workflow_step("s3").unwrap().unwrap().status,
            "cancelled"
        );
        assert_eq!(
            store.get_workflow_step("s4").unwrap().unwrap().status,
            "cancelled"
        );
    }

    #[test]
    fn count_workflows_returns_total() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        assert_eq!(store.count_workflows().unwrap(), 0);

        store.insert_workflow("wf1", "A", "", now).unwrap();
        assert_eq!(store.count_workflows().unwrap(), 1);

        store.insert_workflow("wf2", "B", "", now).unwrap();
        assert_eq!(store.count_workflows().unwrap(), 2);
    }

    #[test]
    fn get_next_pending_respects_step_order() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";

        store.insert_workflow("wf1", "Test", "", now).unwrap();
        // Insert out of order
        store
            .insert_workflow_step(
                "s2",
                "wf1",
                2,
                "agent_prompt",
                None,
                None,
                "step 2",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s1",
                "wf1",
                1,
                "agent_prompt",
                None,
                None,
                "step 1",
                "none",
                None,
                now,
            )
            .unwrap();
        store
            .insert_workflow_step(
                "s0",
                "wf1",
                0,
                "agent_prompt",
                None,
                None,
                "step 0",
                "none",
                None,
                now,
            )
            .unwrap();

        // Should return step_order=0 first
        let next = store.get_next_pending_step("wf1").unwrap().unwrap();
        assert_eq!(next.id, "s0");
        assert_eq!(next.step_order, 0);
    }

    #[test]
    fn workflow_advance_mode_roundtrip() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";
        store.insert_workflow("wf-adv", "Test", "", now).unwrap();

        let wf = store.get_workflow("wf-adv").unwrap().unwrap();
        assert_eq!(wf.advance_mode, "auto");

        store
            .update_workflow_advance_mode("wf-adv", "manual", now)
            .unwrap();
        let wf = store.get_workflow("wf-adv").unwrap().unwrap();
        assert_eq!(wf.advance_mode, "manual");
    }

    #[test]
    fn workflow_advance_signal_poll_and_consume() {
        let store = AuditStore::open_in_memory().unwrap();
        let now = "2026-05-04T10:00:00Z";
        store.insert_workflow("wf-sig", "Test", "", now).unwrap();

        // Insert signals
        store
            .insert_workflow_advance_signal("wf-sig", Some("s1"), "kimi_code", "stop", now)
            .unwrap();
        store
            .insert_workflow_advance_signal("wf-sig", None, "claude_code", "next_step", now)
            .unwrap();

        // Poll unconsumed
        let signals = store.poll_workflow_advance_signals("wf-sig").unwrap();
        assert_eq!(signals.len(), 2);

        // Consume first
        store
            .consume_workflow_advance_signal(signals[0].id, now)
            .unwrap();
        let signals = store.poll_workflow_advance_signals("wf-sig").unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].agent, "claude_code");
    }
}
