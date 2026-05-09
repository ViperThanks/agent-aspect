//! Completion Observer DAO — transcript scanner 观察者的持久化层。
//!
//! `completion_observers` 表记录每个活跃 scanner 观察者的状态，
//! 包括 cursor 位置、idle/hard deadline、完成信号等。
//!
//! 状态机：running → completed / failed / timed_out / cancelled。
//! `maybe_idle` 是中间状态，不触发 workflow 推进。
//!
//! 所有时间字段为 ISO 8601 字符串，与 audit 表其他表保持一致。

use crate::audit::AuditStore;
use crate::error::{AgentAspectError, AgentAspectResult};

/// Completion Observer 行 — 对应 completion_observers 表所有列。
#[derive(Debug, Clone)]
pub struct CompletionObserverRow {
    /// 主键（UUID v4）。
    pub id: String,
    /// 关联的 job ID。
    pub job_id: Option<String>,
    /// 关联的 workflow ID。
    pub workflow_id: Option<String>,
    /// 关联的 workflow step ID。
    pub workflow_step_id: Option<String>,
    /// 关联的 conversation ID。
    pub conversation_id: Option<String>,
    /// 观察的 agent 类型。
    pub agent: String,
    /// transcript 文件路径。
    pub transcript_path: Option<String>,
    /// 文件指纹（path + size + mtime）。
    pub file_fingerprint: Option<String>,
    /// scanner cursor — 当前字节偏移量。
    pub cursor_byte_offset: i64,
    /// 最后一行行号（调试用）。
    pub last_line_no: i64,
    /// 最后一行内容的 SHA256 hash（检测文件重写）。
    pub last_line_hash: Option<String>,
    /// 最后一行的短预览（≤240 字符，不存敏感全文）。
    pub last_line_preview: Option<String>,
    /// 最后观察到的文件修改时间。
    pub last_observed_mtime: Option<String>,
    /// 最后观察到的文件大小。
    pub last_observed_size: Option<i64>,
    /// observer 创建时间（ISO 8601）。
    pub started_at: String,
    /// 最后一次检测到 transcript 变化的时间（ISO 8601）。
    pub last_activity_at: String,
    /// idle 超时截止时间（ISO 8601，基于 last_activity_at 滚动计算）。
    pub idle_deadline_at: String,
    /// 硬超时截止时间（ISO 8601，基于 started_at 固定，不因 delta 延长）。
    pub hard_deadline_at: String,
    /// 当前尝试次数（保留字段，当前固定 1）。
    pub attempt: i64,
    /// 最大尝试次数（保留字段，当前固定 1）。
    pub max_attempts: i64,
    /// observer 状态：running / completed / failed / timed_out / cancelled / maybe_idle。
    pub status: String,
    /// 完成信号类型（CompletionSignalKind 的 serde 序列化）。
    pub completion_signal: Option<String>,
    /// 完成信号权威度（CompletionAuthority 的 serde 序列化）。
    pub completion_authority: Option<String>,
    /// 完成原因描述。
    pub completion_reason: Option<String>,
    /// 记录创建时间（ISO 8601）。
    pub created_at: String,
    /// 记录最后更新时间（ISO 8601）。
    pub updated_at: String,
}

impl AuditStore {
    /// 行映射器 — 从 SQL 行提取 completion_observers 所有列。
    pub(crate) fn map_completion_observer_row(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<CompletionObserverRow> {
        Ok(CompletionObserverRow {
            id: row.get(0)?,
            job_id: row.get(1)?,
            workflow_id: row.get(2)?,
            workflow_step_id: row.get(3)?,
            conversation_id: row.get(4)?,
            agent: row.get(5)?,
            transcript_path: row.get(6)?,
            file_fingerprint: row.get(7)?,
            cursor_byte_offset: row.get(8)?,
            last_line_no: row.get(9)?,
            last_line_hash: row.get(10)?,
            last_line_preview: row.get(11)?,
            last_observed_mtime: row.get(12)?,
            last_observed_size: row.get(13)?,
            started_at: row.get(14)?,
            last_activity_at: row.get(15)?,
            idle_deadline_at: row.get(16)?,
            hard_deadline_at: row.get(17)?,
            attempt: row.get(18)?,
            max_attempts: row.get(19)?,
            status: row.get(20)?,
            completion_signal: row.get(21)?,
            completion_authority: row.get(22)?,
            completion_reason: row.get(23)?,
            created_at: row.get(24)?,
            updated_at: row.get(25)?,
        })
    }

    /// 创建新 observer — 写入 completion_observers 表。
    pub fn create_completion_observer(
        &self,
        id: &str,
        job_id: Option<&str>,
        workflow_id: Option<&str>,
        workflow_step_id: Option<&str>,
        conversation_id: Option<&str>,
        agent: &str,
        transcript_path: Option<&str>,
        file_fingerprint: Option<&str>,
        started_at: &str,
        last_activity_at: &str,
        idle_deadline_at: &str,
        hard_deadline_at: &str,
        max_attempts: i64,
        created_at: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "INSERT INTO completion_observers (
                    id, job_id, workflow_id, workflow_step_id, conversation_id,
                    agent, transcript_path, file_fingerprint,
                    cursor_byte_offset, last_line_no, last_line_hash, last_line_preview,
                    last_observed_mtime, last_observed_size,
                    started_at, last_activity_at, idle_deadline_at, hard_deadline_at,
                    attempt, max_attempts, status,
                    completion_signal, completion_authority, completion_reason,
                    created_at, updated_at
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,0,0,NULL,NULL,NULL,NULL,?9,?10,?11,?12,1,?13,'running',NULL,NULL,NULL,?14,?15)",
                rusqlite::params![
                    id, job_id, workflow_id, workflow_step_id, conversation_id,
                    agent, transcript_path, file_fingerprint,
                    started_at, last_activity_at, idle_deadline_at, hard_deadline_at,
                    max_attempts, created_at, updated_at,
                ],
            )
            .map_err(AgentAspectError::InsertCompletionObserver)?;
        Ok(())
    }

    /// 查询所有活跃 observer。
    ///
    /// scanner 主循环每次 tick 调用此方法获取待检查的 observer 列表。
    /// `maybe_idle` 只是观察态，不是终态，必须继续参与 hard deadline 检查。
    pub fn get_active_observers(&self) -> AgentAspectResult<Vec<CompletionObserverRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, job_id, workflow_id, workflow_step_id, conversation_id,
                        agent, transcript_path, file_fingerprint,
                        cursor_byte_offset, last_line_no, last_line_hash, last_line_preview,
                        last_observed_mtime, last_observed_size,
                        started_at, last_activity_at, idle_deadline_at, hard_deadline_at,
                        attempt, max_attempts, status,
                        completion_signal, completion_authority, completion_reason,
                        created_at, updated_at
                 FROM completion_observers
                 WHERE status IN ('running', 'maybe_idle')",
            )
            .map_err(AgentAspectError::QueryCompletionObserver)?;
        let rows = stmt
            .query_map([], Self::map_completion_observer_row)
            .map_err(AgentAspectError::QueryCompletionObserver)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AgentAspectError::QueryCompletionObserver)
    }

    /// 更新 scanner cursor — transcript 有增量时调用。
    ///
    /// 同时更新 `last_activity_at` 为当前时间，刷新 idle 计时，并把
    /// `maybe_idle` observer 拉回 `running`。
    pub fn update_observer_cursor(
        &self,
        id: &str,
        cursor_byte_offset: i64,
        last_line_no: i64,
        last_line_hash: Option<&str>,
        last_line_preview: Option<&str>,
        last_observed_mtime: Option<&str>,
        last_observed_size: Option<i64>,
        last_activity_at: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET cursor_byte_offset = ?2,
                     last_line_no = ?3,
                     last_line_hash = ?4,
                     last_line_preview = ?5,
                     last_observed_mtime = ?6,
                     last_observed_size = ?7,
                     last_activity_at = ?8,
                     status = 'running',
                     completion_signal = 'TranscriptDelta',
                     completion_authority = 'Informational',
                     completion_reason = '[aspect-transcript] delta observed',
                     updated_at = ?9
                 WHERE id = ?1",
                rusqlite::params![
                    id,
                    cursor_byte_offset,
                    last_line_no,
                    last_line_hash,
                    last_line_preview,
                    last_observed_mtime,
                    last_observed_size,
                    last_activity_at,
                    updated_at,
                ],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 标记 observer 为 maybe_idle — idle deadline 已超但未达 hard deadline。
    ///
    /// `ScannerIdle` 不是终态，不触发 workflow 推进。
    pub fn mark_observer_idle(&self, id: &str, updated_at: &str) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET status = 'maybe_idle',
                     completion_signal = 'ScannerIdle',
                     completion_authority = 'Inferred',
                     completion_reason = '[aspect-idle] no transcript delta',
                     updated_at = ?2
                 WHERE id = ?1",
                rusqlite::params![id, updated_at],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 标记 observer 为 completed — stop hook 或 process exit 0 触发。
    pub fn mark_observer_completed(
        &self,
        id: &str,
        signal: &str,
        authority: &str,
        reason: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET status = 'completed',
                     completion_signal = ?2,
                     completion_authority = ?3,
                     completion_reason = ?4,
                     updated_at = ?5
                 WHERE id = ?1",
                rusqlite::params![id, signal, authority, reason, updated_at],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 标记 observer 为 failed — process exit non-zero 等确定失败终态。
    pub fn mark_observer_failed(
        &self,
        id: &str,
        signal: &str,
        authority: &str,
        reason: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET status = 'failed',
                     completion_signal = ?2,
                     completion_authority = ?3,
                     completion_reason = ?4,
                     updated_at = ?5
                 WHERE id = ?1",
                rusqlite::params![id, signal, authority, reason, updated_at],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 标记 observer 为 timed_out — hard deadline 超出。
    pub fn mark_observer_timed_out(
        &self,
        id: &str,
        reason: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET status = 'timed_out',
                     completion_signal = 'ScannerTimeout',
                     completion_authority = 'Authoritative',
                     completion_reason = ?2,
                     updated_at = ?3
                 WHERE id = ?1",
                rusqlite::params![id, reason, updated_at],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 标记 observer 为 cancelled — 用户手动取消。
    pub fn mark_observer_cancelled(
        &self,
        id: &str,
        reason: &str,
        updated_at: &str,
    ) -> AgentAspectResult<()> {
        self.conn
            .execute(
                "UPDATE completion_observers
                 SET status = 'cancelled',
                     completion_signal = 'ManualCancel',
                     completion_authority = 'Authoritative',
                     completion_reason = ?2,
                     updated_at = ?3
                 WHERE id = ?1",
                rusqlite::params![id, reason, updated_at],
            )
            .map_err(AgentAspectError::UpdateCompletionObserver)?;
        Ok(())
    }

    /// 按 job_id 查询 observer — 用于 job 完成时查找对应的 observer。
    pub fn get_observer_by_job_id(
        &self,
        job_id: &str,
    ) -> AgentAspectResult<Option<CompletionObserverRow>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, job_id, workflow_id, workflow_step_id, conversation_id,
                        agent, transcript_path, file_fingerprint,
                        cursor_byte_offset, last_line_no, last_line_hash, last_line_preview,
                        last_observed_mtime, last_observed_size,
                        started_at, last_activity_at, idle_deadline_at, hard_deadline_at,
                        attempt, max_attempts, status,
                        completion_signal, completion_authority, completion_reason,
                        created_at, updated_at
                 FROM completion_observers
                 WHERE job_id = ?1",
            )
            .map_err(AgentAspectError::QueryCompletionObserver)?;
        let result = stmt
            .query_row(rusqlite::params![job_id], Self::map_completion_observer_row)
            .ok();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::audit::AuditStore;

    fn insert_observer(store: &AuditStore) {
        store
            .create_completion_observer(
                "obs-1",
                Some("job-1"),
                None,
                None,
                Some("conv-1"),
                "claude_code",
                Some("/tmp/transcript.jsonl"),
                None,
                "2026-05-10T00:00:00Z",
                "2026-05-10T00:00:00Z",
                "2026-05-10T00:01:00Z",
                "2026-05-10T00:10:00Z",
                1,
                "2026-05-10T00:00:00Z",
                "2026-05-10T00:00:00Z",
            )
            .expect("insert observer");
    }

    #[test]
    fn maybe_idle_observer_stays_active_for_hard_deadline_scan() {
        let store = AuditStore::open_in_memory().expect("open db");
        insert_observer(&store);

        store
            .mark_observer_idle("obs-1", "2026-05-10T00:02:00Z")
            .expect("mark idle");

        let active = store.get_active_observers().expect("active observers");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "obs-1");
        assert_eq!(active[0].status, "maybe_idle");
        assert_eq!(active[0].completion_signal.as_deref(), Some("ScannerIdle"));
    }

    #[test]
    fn transcript_delta_reactivates_maybe_idle_observer() {
        let store = AuditStore::open_in_memory().expect("open db");
        insert_observer(&store);

        store
            .mark_observer_idle("obs-1", "2026-05-10T00:02:00Z")
            .expect("mark idle");
        store
            .update_observer_cursor(
                "obs-1",
                128,
                3,
                Some("line-hash"),
                Some("{\"type\":\"assistant\"}"),
                Some("2026-05-10T00:03:00Z"),
                Some(128),
                "2026-05-10T00:03:00Z",
                "2026-05-10T00:03:00Z",
            )
            .expect("update cursor");

        let observer = store
            .get_observer_by_job_id("job-1")
            .expect("query observer")
            .expect("observer exists");
        assert_eq!(observer.status, "running");
        assert_eq!(observer.cursor_byte_offset, 128);
        assert_eq!(observer.last_line_no, 3);
        assert_eq!(
            observer.completion_signal.as_deref(),
            Some("TranscriptDelta")
        );
        assert_eq!(
            observer.completion_authority.as_deref(),
            Some("Informational")
        );
    }

    #[test]
    fn failed_observer_keeps_failed_status() {
        let store = AuditStore::open_in_memory().expect("open db");
        insert_observer(&store);

        store
            .mark_observer_failed(
                "obs-1",
                "ProcessExit",
                "Authoritative",
                "[aspect-process] exit code 1",
                "2026-05-10T00:04:00Z",
            )
            .expect("mark failed");

        let observer = store
            .get_observer_by_job_id("job-1")
            .expect("query observer")
            .expect("observer exists");
        assert_eq!(observer.status, "failed");
        assert_eq!(observer.completion_signal.as_deref(), Some("ProcessExit"));
        assert_eq!(
            observer.completion_authority.as_deref(),
            Some("Authoritative")
        );
    }
}
