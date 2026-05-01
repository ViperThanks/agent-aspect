//! 标题导入 — 从 provider transcript 文件中提取真实会话标题。
//!
//! 支持 Claude Code（ai-title / first user prompt）、Codex CLI（thread_name / first user message）、
//! Kimi Code（custom_title）。
//!
//! 标题优先级：provider > first_prompt > fallback，与 conversation.rs 中 TitleSource 一致。

use crate::conversation::TitleSource;

/// Try to extract a real title for a conversation from provider transcript files.
///
/// Returns `(title, source)` where source is `"provider"`, `"first_prompt"`, or `"fallback"`.
/// Returns `None` if no better title can be found.
pub fn import_title_for(
    agent: &str,
    conversation_id: &str,
    project_path: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<(String, String)> {
    let (title, source) = match agent {
        "claude_code" => extract_claude_title(conversation_id, project_path, transcript_path)?,
        "codex_cli" => extract_codex_title(transcript_path)?,
        "kimi_code" => extract_kimi_title(conversation_id)?,
        _ => return None,
    };
    Some((title, source.as_str().to_string()))
}

// ---------------------------------------------------------------------------
// Claude Code
// ---------------------------------------------------------------------------

fn extract_claude_title(
    session_id: &str,
    project_path: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<(String, TitleSource)> {
    let file_path = resolve_claude_title_path(session_id, project_path, transcript_path)?;
    let content = std::fs::read_to_string(&file_path).ok()?;

    // Priority 1: ai-title
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("ai-title") {
                if let Some(title) = v.get("aiTitle").and_then(|t| t.as_str()) {
                    if !title.is_empty() {
                        return Some((title.to_string(), TitleSource::Provider));
                    }
                }
            }
        }
    }

    // Priority 2: first user prompt
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("user") {
                if let Some(text) = extract_user_message_text(&v) {
                    if !text.is_empty() {
                        return Some((
                            crate::utils::truncate_str(&text, crate::constants::TITLE_MAX_LEN),
                            TitleSource::FirstPrompt,
                        ));
                    }
                }
            }
        }
    }

    None
}

/// 解析 Claude Code transcript 路径。
///
/// 新导入会话会持久化 transcript_path，因此优先使用它；老数据继续兼容
/// project_path → `~/.claude/projects/{encoded}` 的历史解析方式。
fn resolve_claude_title_path(
    session_id: &str,
    project_path: Option<&str>,
    transcript_path: Option<&str>,
) -> Option<std::path::PathBuf> {
    if let Some(path) = transcript_path {
        let path = std::path::PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    let pp = project_path?;
    let dir = crate::utils::claude_project_dir(pp)?;
    let path = dir.join(format!("{session_id}.jsonl"));
    if path.exists() { Some(path) } else { None }
}

/// Extract the text content from a user message JSON value.
/// `message.content` can be a plain string or an array of content blocks.
fn extract_user_message_text(v: &serde_json::Value) -> Option<String> {
    let content = v.get("message")?.get("content")?;

    // Plain string
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }

    // Array of content blocks — find first text block
    if let Some(arr) = content.as_array() {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    return Some(text.to_string());
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Codex CLI
// ---------------------------------------------------------------------------

fn extract_codex_title(transcript_path: Option<&str>) -> Option<(String, TitleSource)> {
    let path = transcript_path?;
    let content = std::fs::read_to_string(path).ok()?;

    // Priority 1: thread_name from event_msg with payload.type == "thread_name_updated"
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
                if let Some(payload) = v.get("payload") {
                    if payload.get("type").and_then(|t| t.as_str()) == Some("thread_name_updated") {
                        if let Some(name) = payload.get("thread_name").and_then(|n| n.as_str()) {
                            if !name.is_empty() {
                                return Some((name.to_string(), TitleSource::Provider));
                            }
                        }
                    }
                }
            }
        }
    }

    // Priority 2: first user message from event_msg with payload.type == "user_message"
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("event_msg") {
                if let Some(payload) = v.get("payload") {
                    if payload.get("type").and_then(|t| t.as_str()) == Some("user_message") {
                        if let Some(text) = payload.get("message").and_then(|m| m.as_str()) {
                            if !text.is_empty() {
                                return Some((
                                    crate::utils::truncate_str(
                                        text,
                                        crate::constants::TITLE_MAX_LEN,
                                    ),
                                    TitleSource::FirstPrompt,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Kimi Code
// ---------------------------------------------------------------------------

fn extract_kimi_title(session_id: &str) -> Option<(String, TitleSource)> {
    let home = std::env::var("HOME").unwrap_or_default();
    let sessions_dir = format!("{home}/.kimi/sessions");
    let entries = std::fs::read_dir(&sessions_dir).ok()?;

    for entry in entries.flatten() {
        let state_path = entry.path().join(session_id).join("state.json");
        if state_path.exists() {
            let content = std::fs::read_to_string(&state_path).ok()?;
            let v: serde_json::Value = serde_json::from_str(&content).ok()?;
            if let Some(title) = v.get("custom_title").and_then(|t| t.as_str()) {
                if !title.is_empty() {
                    return Some((title.to_string(), TitleSource::Provider));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_title_unknown_agent() {
        assert_eq!(import_title_for("unknown", "sid", None, None), None);
    }

    #[test]
    fn codex_title_from_thread_name() {
        let jsonl = r#"{"timestamp":"2026-04-26T02:45:02.771Z","type":"session_meta","payload":{"id":"019dc7ac-a8fa-72e2-b1b1-0696f5026871","cwd":"/Users/test/proj"}}
{"timestamp":"2026-04-26T02:45:05.000Z","type":"event_msg","payload":{"type":"user_message","message":"fix the login bug"}}
{"timestamp":"2026-04-26T02:45:10.000Z","type":"event_msg","payload":{"type":"thread_name_updated","thread_id":"tid-1","thread_name":"Fix login authentication"}}
{"timestamp":"2026-04-26T02:45:15.000Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":\"cat src/login.rs\"}","call_id":"c1"}}
"#;
        let dir = std::env::temp_dir().join(format!("codex-title-thread-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jsonl");
        std::fs::write(&path, jsonl).unwrap();
        let result = extract_codex_title(Some(path.to_str().unwrap()));
        assert_eq!(
            result,
            Some((
                "Fix login authentication".to_string(),
                TitleSource::Provider
            ))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn codex_title_from_first_user_message() {
        let jsonl = r#"{"timestamp":"2026-04-26T02:45:02.771Z","type":"session_meta","payload":{"id":"sess-1","cwd":"/Users/test/proj"}}
{"timestamp":"2026-04-26T02:45:05.000Z","type":"event_msg","payload":{"type":"user_message","message":"echo hello"}}
{"timestamp":"2026-04-26T02:45:10.000Z","type":"response_item","payload":{"type":"function_call","name":"shell","arguments":"{\"command\":\"echo hello\"}","call_id":"c1"}}
"#;
        let dir = std::env::temp_dir().join(format!("codex-title-msg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jsonl");
        std::fs::write(&path, jsonl).unwrap();
        let result = extract_codex_title(Some(path.to_str().unwrap()));
        assert_eq!(
            result,
            Some(("echo hello".to_string(), TitleSource::FirstPrompt))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn codex_title_no_transcript_path() {
        assert_eq!(extract_codex_title(None), None);
    }

    #[test]
    fn codex_title_empty_file() {
        let dir = std::env::temp_dir().join(format!("codex-title-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jsonl");
        std::fs::write(&path, "").unwrap();
        let result = extract_codex_title(Some(path.to_str().unwrap()));
        assert_eq!(result, None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
