//! 会话管理 — conversation ID 生成、元数据提取、标题推断。
//!
//! 核心不变量：
//! - conversation_db_id = SHA-256("{agent}:{provider_cid}")，全局唯一、跨机器稳定
//! - 标题来源优先级：provider transcript > first_prompt > fallback
//! - 同一 session_id 在不同 agent 下不会碰撞

use sha2::{Digest, Sha256};

/// Origin of a conversation title, ordered by priority (lowest value = highest priority).
#[derive(Debug, Clone, PartialEq)]
pub enum TitleSource {
    Provider,
    FirstPrompt,
    Fallback,
}

impl TitleSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TitleSource::Provider => "provider",
            TitleSource::FirstPrompt => "first_prompt",
            TitleSource::Fallback => "fallback",
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            TitleSource::Provider => 0,
            TitleSource::FirstPrompt => 1,
            TitleSource::Fallback => 2,
        }
    }
}

/// Metadata extracted from SessionStart / UserPromptSubmit hook payloads.
pub struct MetadataUpdate {
    pub session_id: Option<String>,
    pub project_path: Option<String>,
    pub transcript_path: Option<String>,
    pub title: Option<String>,
    pub title_source: Option<String>,
}

/// Extract metadata from a SessionStart hook payload.
pub fn extract_session_start_metadata(raw_payload: &str) -> MetadataUpdate {
    let payload: serde_json::Value = match serde_json::from_str(raw_payload) {
        Ok(v) => v,
        Err(_) => {
            return MetadataUpdate {
                session_id: None,
                project_path: None,
                transcript_path: None,
                title: None,
                title_source: None,
            };
        }
    };

    MetadataUpdate {
        session_id: payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        project_path: payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        transcript_path: payload
            .get("transcript_path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        title: None,
        title_source: None,
    }
}

/// Extract metadata from a UserPromptSubmit hook payload.
pub fn extract_prompt_metadata(raw_payload: &str) -> MetadataUpdate {
    let payload: serde_json::Value = match serde_json::from_str(raw_payload) {
        Ok(v) => v,
        Err(_) => {
            return MetadataUpdate {
                session_id: None,
                project_path: None,
                transcript_path: None,
                title: None,
                title_source: None,
            };
        }
    };

    let prompt = payload.get("prompt").and_then(|v| v.as_str()).unwrap_or("");

    let title = if prompt.is_empty() {
        None
    } else {
        Some(truncate_first_line(prompt, crate::constants::TITLE_MAX_LEN))
    };

    let title_source = if title.is_some() {
        Some("first_prompt".to_string())
    } else {
        None
    };

    MetadataUpdate {
        session_id: payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        project_path: payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        transcript_path: payload
            .get("transcript_path")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        title,
        title_source,
    }
}

/// Truncate the first line of a string to `max_chars` characters, appending
/// an ellipsis if truncated.
fn truncate_first_line(s: &str, max_chars: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    crate::utils::truncate_str(first_line, max_chars)
}

/// 从 raw_payload 中提取 provider 会话 ID。
/// Claude/Kimi 用 session_id；Codex 优先 session_id，回退 turn_id。
pub fn extract_conversation_id(agent: &str, raw_payload: &str) -> Option<String> {
    let payload: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    match agent {
        "claude_code" | "kimi_code" => payload.get("session_id")?.as_str().map(|s| s.to_string()),
        "codex_cli" => payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("turn_id").and_then(|v| v.as_str()))
            .map(|s| s.to_string()),
        "gemini_cli" => payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// 从 raw_payload 提取项目路径。
/// 大部分 provider 用 cwd；Codex 可从 transcript_path 的父目录推导。
pub fn extract_project_path(agent: &str, raw_payload: &str) -> Option<String> {
    let payload: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    let cwd = payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if cwd.is_some() {
        return cwd;
    }
    // Codex may have transcript_path instead of cwd
    if agent == "codex_cli" {
        payload
            .get("transcript_path")
            .and_then(|v| v.as_str())
            .and_then(|s| std::path::Path::new(s).parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.to_str())
            .map(|s| s.to_string())
    } else {
        None
    }
}

/// 从 raw_payload 提取 transcript 路径（非空且有值时返回）。
pub fn extract_transcript_path(_agent: &str, raw_payload: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    v.get("transcript_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 从 hook payload 提取权限模式。
///
/// 只提取 permission_mode，不从 hook payload 写入 model/profile。
/// 原因：payload 中的 model 可能是 provider 原始长名，而 runtime probe 会标准化；
/// 混写会制造假的 model drift。
pub fn extract_permission_mode(raw_payload: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw_payload).ok()?;
    v.get("permission_mode")
        .or_else(|| v.get("permissionMode"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 生成 fallback 标题："{AgentName} · {project_dir_name}"。
/// 仅在无 provider / first_prompt 标题时使用。
pub fn generate_title(
    agent: &str,
    project_path: Option<&str>,
    _raw_payload: Option<&str>,
) -> String {
    let agent_name = match agent {
        "claude_code" => "Claude Code",
        "kimi_code" => "Kimi Code",
        "codex_cli" => "Codex CLI",
        "gemini_cli" => "Gemini CLI",
        "bridge" => "Bridge",
        _ => agent,
    };

    let project = project_path
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    format!("{agent_name} · {project}")
}

/// Stable DB primary key for a conversation.
/// Uses SHA-256 over "{agent}:{conversation_id}" to guarantee:
/// - same input → same output across runs, machines, Rust versions
/// - different agents with same conversation_id are isolated
pub fn conversation_db_id(agent: &str, conversation_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(agent.as_bytes());
    hasher.update(b":");
    hasher.update(conversation_id.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_claude_session_id() {
        let payload = r#"{"session_id":"sess-123","cwd":"/tmp/proj","tool_name":"Bash"}"#;
        assert_eq!(
            extract_conversation_id("claude_code", payload),
            Some("sess-123".to_string())
        );
    }

    #[test]
    fn extract_kimi_session_id() {
        let payload = r#"{"session_id":"sess-456","cwd":"/tmp/proj2","tool_name":"Shell"}"#;
        assert_eq!(
            extract_conversation_id("kimi_code", payload),
            Some("sess-456".to_string())
        );
    }

    #[test]
    fn extract_codex_prefers_session_id() {
        let payload = r#"{"session_id":"sess-789","turn_id":"turn-abc","cwd":"/tmp/proj3","tool_name":"Bash"}"#;
        assert_eq!(
            extract_conversation_id("codex_cli", payload),
            Some("sess-789".to_string())
        );
    }

    #[test]
    fn extract_codex_falls_back_to_turn_id() {
        let payload = r#"{"turn_id":"turn-abc","cwd":"/tmp/proj3","tool_name":"Bash"}"#;
        assert_eq!(
            extract_conversation_id("codex_cli", payload),
            Some("turn-abc".to_string())
        );
    }

    #[test]
    fn extract_permission_mode_reads_payload() {
        let payload = r#"{"session_id":"s","cwd":"/p","model":"claude-sonnet-4","permission_mode":"bypassPermissions"}"#;
        assert_eq!(
            extract_permission_mode(payload),
            Some("bypassPermissions".to_string())
        );
    }

    #[test]
    fn extract_permission_mode_skips_empty_payload() {
        let payload = r#"{"session_id":"s","cwd":"/p"}"#;
        assert!(extract_permission_mode(payload).is_none());
    }

    #[test]
    fn generate_title_format() {
        let title = generate_title("claude_code", Some("/Users/x/project"), None);
        assert_eq!(title, "Claude Code · project");
    }

    #[test]
    fn conversation_db_id_is_stable() {
        let id1 = conversation_db_id("claude_code", "sess-1");
        let id2 = conversation_db_id("claude_code", "sess-1");
        assert_eq!(id1, id2);
    }

    #[test]
    fn conversation_db_id_isolated_by_agent() {
        // Two agents using the same session_id must not collide
        let id_claude = conversation_db_id("claude_code", "shared-sess");
        let id_kimi = conversation_db_id("kimi_code", "shared-sess");
        assert_ne!(
            id_claude, id_kimi,
            "same session_id across agents must not collide"
        );
    }

    #[test]
    fn conversation_db_id_isolated_by_conversation_id() {
        let id_a = conversation_db_id("claude_code", "sess-a");
        let id_b = conversation_db_id("claude_code", "sess-b");
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn session_start_metadata_extracts_fields() {
        let payload = r#"{"hook_event_name":"SessionStart","session_id":"sess-1","cwd":"/tmp/proj","source":"startup","transcript_path":"/tmp/transcript.jsonl"}"#;
        let m = extract_session_start_metadata(payload);
        assert_eq!(m.session_id, Some("sess-1".to_string()));
        assert_eq!(m.project_path, Some("/tmp/proj".to_string()));
        assert_eq!(m.transcript_path, Some("/tmp/transcript.jsonl".to_string()));
        assert!(m.title.is_none());
        assert!(m.title_source.is_none());
    }

    #[test]
    fn prompt_metadata_extracts_title() {
        let payload = r#"{"hook_event_name":"UserPromptSubmit","session_id":"sess-1","cwd":"/tmp/proj","prompt":"fix the login bug"}"#;
        let m = extract_prompt_metadata(payload);
        assert_eq!(m.session_id, Some("sess-1".to_string()));
        assert_eq!(m.title, Some("fix the login bug".to_string()));
        assert_eq!(m.title_source, Some("first_prompt".to_string()));
    }

    #[test]
    fn prompt_metadata_truncates_long_prompt() {
        let long = "x".repeat(200);
        let payload = format!(r#"{{"session_id":"s","cwd":"/p","prompt":"{}"}}"#, long);
        let m = extract_prompt_metadata(&payload);
        let title = m.title.unwrap();
        assert!(title.chars().count() <= 81); // 80 chars + ellipsis char
        assert!(title.ends_with('…'));
    }

    #[test]
    fn prompt_metadata_empty_prompt() {
        let payload = r#"{"session_id":"s","cwd":"/p","prompt":""}"#;
        let m = extract_prompt_metadata(payload);
        assert!(m.title.is_none());
        assert!(m.title_source.is_none());
    }
}
