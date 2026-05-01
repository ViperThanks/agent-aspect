//! 通用工具函数 — 字符串截断、项目目录解析等。
//!
//! `truncate_str` 按 Unicode 字符计数（非字节），用于标题和工具输入预览。
//! `claude_project_dir` 将项目路径映射到 `~/.claude/projects/` 目录。

/// 按 Unicode 字符数截断，超出时追加省略号（U+2026）。
///
/// 与 transcript.rs 内的 `truncate_str` 不同：本函数按字符计、用单字符省略号；
/// 那个按字节计、用三字符 `...`，专为 JSON 工具输入设计。
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let end = s
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    format!("{}…", &s[..end])
}

/// 将项目路径映射为 `~/.claude/projects/{encoded}/` 目录。
///
/// 编码规则：`/` 替换为 `-`。目录不存在时返回 None。
pub fn claude_project_dir(project_path: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let encoded = project_path.replace('/', "-");
    let dir = std::path::PathBuf::from(home)
        .join(".claude/projects")
        .join(&encoded);
    if dir.exists() { Some(dir) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short() {
        assert_eq!(truncate_str("hello", 80), "hello");
    }

    #[test]
    fn truncate_long() {
        let long: String = "a".repeat(100);
        let result = truncate_str(&long, 80);
        assert!(result.chars().count() == 81); // 80 chars + ellipsis
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_unicode() {
        let input: String = "你".repeat(100);
        let result = truncate_str(&input, 10);
        assert!(result.ends_with('…'));
        let without_suffix: String = result.chars().take(10).collect();
        assert_eq!(without_suffix.chars().count(), 10);
    }

    #[test]
    fn truncate_exact_length() {
        let s = "abcde";
        assert_eq!(truncate_str(s, 5), "abcde");
    }
}
