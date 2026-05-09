//! Transcript Scanner — 后台线程定时扫描活跃 observer 的 transcript 增量。
//!
//! Scanner 通过 DAO 方法更新 observer 状态（cursor / idle / timed_out）。
//! job 终态由 JobRunner 的 CompletionSink 写入，scanner 负责提供独立观测证据。
//!
//! 关键不变量：
//! - ScannerIdle 永远不等于 completed
//! - idle_deadline_at 基于 last_activity_at 滚动
//! - hard_deadline_at 基于 started_at 固定，不因 transcript delta 延长
//! - cursor 必须能处理文件截断/重写（size < cursor → reset）
//! - scanner 处理 running / maybe_idle observer；maybe_idle 必须继续等待 hard deadline

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use agent_aspect_core::audit::AuditStore;
use agent_aspect_core::store::completion::CompletionObserverRow;
use agent_aspect_core::utils::truncate_str;

use sha2::{Digest, Sha256};

/// scanner 主循环入口 — 在独立线程中运行。
///
/// 每次 tick：
/// 1. 查询 status in ('running', 'maybe_idle') 的 observers
/// 2. 对每个 observer 执行 deadline 检查 + transcript 增量扫描
/// 3. 根据结果更新 observer 状态
///
/// 打开独立 DB 连接，避免与请求线程争锁。
pub fn start_scanner_loop(db_path: std::path::PathBuf, poll_interval_secs: u64) {
    let store = match AuditStore::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[aspect-scanner] failed to open DB: {e}");
            return;
        }
    };

    let interval = Duration::from_secs(poll_interval_secs.max(1));
    eprintln!(
        "[aspect-scanner] started (poll_interval={}s)",
        poll_interval_secs
    );

    loop {
        std::thread::sleep(interval);

        let observers = match store.get_active_observers() {
            Ok(obs) => obs,
            Err(e) => {
                eprintln!("[aspect-scanner] query active observers failed: {e}");
                continue;
            }
        };

        if !observers.is_empty() {
            eprintln!(
                "[aspect-scanner] tick: {} active observers",
                observers.len()
            );
        }

        for observer in &observers {
            if let Err(e) = process_observer(&store, observer) {
                eprintln!("[aspect-scanner] observer {} error: {e}", observer.id);
            }
        }
    }
}

/// 增量扫描单个 observer — 检查 deadline + 读 transcript。
///
/// 处理顺序（优先级从高到低）：
/// 1. hard_deadline_at 已超 → TimedOut（Authoritative 终态）
/// 2. idle_deadline_at 已超 → MaybeIdle（Inferred 中间态）
/// 3. transcript 有增量 → 更新 cursor + 刷新 last_activity_at
/// 4. 无 delta 且未超 idle → 静默（不动 DB）
fn process_observer(store: &AuditStore, observer: &CompletionObserverRow) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let now_ts = parse_iso8601(&now).ok_or("failed to parse current time")?;

    // 1. hard deadline 检查（最高优先级）
    if let Some(hard_ts) = parse_iso8601(&observer.hard_deadline_at) {
        if now_ts >= hard_ts {
            let reason = "[aspect-timeout] hard deadline exceeded";
            store
                .mark_observer_timed_out(&observer.id, reason, &now)
                .map_err(|e| format!("mark_timed_out: {e}"))?;
            eprintln!(
                "[aspect-scanner] observer {} → timed_out (hard deadline)",
                observer.id
            );
            return Ok(());
        }
    }

    // 2. idle deadline 检查
    let over_idle = parse_iso8601(&observer.idle_deadline_at)
        .map(|idle_ts| now_ts >= idle_ts)
        .unwrap_or(false);

    // 3. transcript 增量扫描
    let scan_result = scan_transcript(observer);

    match scan_result {
        ScanResult::NoTranscriptPath => {
            if over_idle {
                store
                    .mark_observer_idle(&observer.id, &now)
                    .map_err(|e| format!("mark_idle: {e}"))?;
                eprintln!(
                    "[aspect-scanner] observer {} → maybe_idle (no transcript path, deadline fallback)",
                    observer.id
                );
            }
        }
        ScanResult::NoDelta => {
            if over_idle {
                store
                    .mark_observer_idle(&observer.id, &now)
                    .map_err(|e| format!("mark_idle: {e}"))?;
                eprintln!("[aspect-scanner] observer {} → maybe_idle", observer.id);
            }
        }
        ScanResult::Truncated {
            new_offset,
            last_line_no,
            last_line_hash,
            last_line_preview,
        } => {
            // 文件被截断/重写 → cursor 归零，刷新 activity
            store
                .update_observer_cursor(
                    &observer.id,
                    new_offset,
                    last_line_no,
                    last_line_hash.as_deref(),
                    last_line_preview.as_deref(),
                    None,
                    Some(new_offset),
                    &now,
                    &now,
                )
                .map_err(|e| format!("update_cursor (truncated): {e}"))?;
            eprintln!(
                "[aspect-scanner] observer {} → truncated reset, cursor={}",
                observer.id, new_offset
            );
        }
        ScanResult::Delta {
            new_offset,
            last_line_no,
            last_line_hash,
            last_line_preview,
            file_size,
        } => {
            // 有增量 → 更新 cursor + 刷新 last_activity_at
            store
                .update_observer_cursor(
                    &observer.id,
                    new_offset,
                    last_line_no,
                    last_line_hash.as_deref(),
                    last_line_preview.as_deref(),
                    None,
                    Some(file_size),
                    &now,
                    &now,
                )
                .map_err(|e| format!("update_cursor: {e}"))?;
            eprintln!(
                "[aspect-scanner] observer {} → delta, cursor={} line={}",
                observer.id, new_offset, last_line_no
            );
        }
    }

    Ok(())
}

/// 增量扫描 transcript 的结果。
enum ScanResult {
    /// observer 无 transcript_path 字段。
    NoTranscriptPath,
    /// 文件存在但无新增内容（size == cursor 或文件不存在）。
    NoDelta,
    /// 文件被截断/重写（size < cursor），cursor 需归零。
    Truncated {
        new_offset: i64,
        last_line_no: i64,
        last_line_hash: Option<String>,
        last_line_preview: Option<String>,
    },
    /// 有新内容（size > cursor），返回新的 cursor 位置和最后一行信息。
    Delta {
        new_offset: i64,
        last_line_no: i64,
        last_line_hash: Option<String>,
        last_line_preview: Option<String>,
        file_size: i64,
    },
}

/// 增量读取 transcript — 从 cursor_byte_offset 开始读新 bytes。
///
/// 1. stat 文件 → 不存在则 NoDelta
/// 2. size < cursor → 文件被重写/截断 → cursor 归零（Truncated）
/// 3. size == cursor → 无新增 → NoDelta
/// 4. size > cursor → 读新增部分，提取最后一行 → Delta
///
/// 只统计行数和 hash，不解析 JSONL 内容。
fn scan_transcript(observer: &CompletionObserverRow) -> ScanResult {
    let path_str = match observer.transcript_path.as_deref() {
        Some(p) => p,
        None => return ScanResult::NoTranscriptPath,
    };
    let path = std::path::Path::new(path_str);

    // 1. stat 文件
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return ScanResult::NoDelta,
    };
    let file_size = metadata.len() as i64;

    // 2. 文件截断检测
    if file_size < observer.cursor_byte_offset {
        return ScanResult::Truncated {
            new_offset: 0,
            last_line_no: 0,
            last_line_hash: None,
            last_line_preview: None,
        };
    }

    // 3. 无新增
    if file_size == observer.cursor_byte_offset {
        return ScanResult::NoDelta;
    }

    // 4. 有增量 → 读取新增部分
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return ScanResult::NoDelta,
    };

    let bytes_to_read = (file_size - observer.cursor_byte_offset) as usize;
    file.seek(SeekFrom::Start(observer.cursor_byte_offset as u64))
        .ok();

    let to_read = bytes_to_read.min(READ_CHUNK_LIMIT);
    let mut chunk = vec![0u8; to_read];
    let n = match file.read(&mut chunk) {
        Ok(n) if n > 0 => n,
        _ => return ScanResult::NoDelta,
    };

    // 提取最后一行
    let content = String::from_utf8_lossy(&chunk[..n]);
    let (last_line_no, last_line_hash, last_line_preview) =
        extract_last_line(&content, observer.last_line_no);

    ScanResult::Delta {
        new_offset: file_size,
        last_line_no,
        last_line_hash,
        last_line_preview,
        file_size,
    }
}

/// 从增量内容中提取最后一行信息。
///
/// 返回 (新的总行数, 最后一行 SHA256 hash, 最后一行预览≤240字符)。
fn extract_last_line(content: &str, prev_line_no: i64) -> (i64, Option<String>, Option<String>) {
    let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return (prev_line_no, None, None);
    }

    let new_line_count = lines.len() as i64;
    let total_line_no = prev_line_no + new_line_count;

    let last_line = lines.last().unwrap();
    let hash = format!("{:x}", Sha256::digest(last_line.as_bytes()));
    let preview = truncate_str(last_line, LAST_LINE_PREVIEW_LEN);

    (total_line_no, Some(hash), Some(preview))
}

/// 解析 ISO 8601 字符串为 Unix 时间戳（秒），用于比较 deadline。
fn parse_iso8601(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp())
}

/// 单次读取的最大字节数（1MB）— 防止单次扫描读取过多数据。
const READ_CHUNK_LIMIT: usize = 1024 * 1024;

/// 最后一行预览的最大字符数。
const LAST_LINE_PREVIEW_LEN: usize = 240;
