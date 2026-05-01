//! 规则引擎 — 对工具使用事件进行安全评估。
//!
//! 规则按严重级别组织：Observer < Autonomous < Guard < Paranoid。
//! 每条规则检查 UnifiedEvent 并返回 Action (allow / ask / deny)。
//! Mode 控制哪些规则集生效。
//!
//! 规则评估语义：
//! - Observer：全量评估但全部 allow（仅记录 "would be X"）
//! - Autonomous+：硬红线（force push / rm -rf / sudo）强制 deny
//! - Guard+：默认把关（main push / bulk delete / package install）需确认
//! - Paranoid：未分类操作一律 ask

use crate::decision::{Action, Decision};
use crate::event::UnifiedEvent;
use serde::{Deserialize, Serialize};
use shlex::split as shell_split;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Observer,
    Autonomous,
    Guard,
    Paranoid,
}

impl Mode {
    pub fn level(self) -> u8 {
        match self {
            Self::Observer => 0,
            Self::Autonomous => 1,
            Self::Guard => 2,
            Self::Paranoid => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    Default,
    Learned,
    User,
    Community,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub description: &'static str,
    pub source: RuleSource,
    pub default_action: Action,
    pub matcher: fn(&UnifiedEvent) -> bool,
    /// 规则在此模式及以上生效。Observer 特殊：所有规则都"评估"但不执行。
    pub min_mode: Mode,
}

/// 规则引擎 — 持有规则列表和当前模式，对事件进行评估。
pub struct RuleEngine {
    rules: Vec<Rule>,
    mode: Mode,
}

impl RuleEngine {
    /// 使用内置默认规则集构造引擎。
    pub fn with_defaults(mode: Mode) -> Self {
        Self {
            rules: default_rules(),
            mode,
        }
    }

    /// 评估事件：遍历激活规则，返回优先级最高的 action 对应的 Decision。
    pub fn evaluate(&self, event: &UnifiedEvent) -> Decision {
        // Observer 模式：全量评估但全部 allow，note 中标注 "would be X"
        if self.mode == Mode::Observer {
            return self.evaluate_observer(event);
        }

        // 非 Observer: 只激活 min_mode <= current mode 的规则
        let active: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|r| r.min_mode.level() <= self.mode.level())
            .collect();

        let matched: Vec<&Rule> = active
            .iter()
            .copied()
            .filter(|r| (r.matcher)(event))
            .collect();

        if matched.is_empty() {
            // Paranoid: 未分类操作 → ask
            if self.mode == Mode::Paranoid {
                return Decision {
                    event_id: event.id.clone(),
                    action: Action::Ask,
                    rule_id: Some("D999".to_string()),
                    note: "paranoid mode: unclassified operation requires confirmation".to_string(),
                };
            }
            return Decision {
                event_id: event.id.clone(),
                action: Action::Allow,
                rule_id: None,
                note: "no rule matched".to_string(),
            };
        }

        let primary = matched
            .iter()
            .max_by_key(|r| action_priority(r.default_action))
            .unwrap();

        let note = matched
            .iter()
            .map(|r| format!("{}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("; ");

        Decision {
            event_id: event.id.clone(),
            action: primary.default_action,
            rule_id: Some(primary.id.to_string()),
            note,
        }
    }

    /// Observer 专用评估：所有 action 强制 allow，但在 note 中标注原始建议。
    fn evaluate_observer(&self, event: &UnifiedEvent) -> Decision {
        let matched: Vec<&Rule> = self.rules.iter().filter(|r| (r.matcher)(event)).collect();

        if matched.is_empty() {
            return Decision {
                event_id: event.id.clone(),
                action: Action::Allow,
                rule_id: None,
                note: "no rule matched".to_string(),
            };
        }

        let primary = matched
            .iter()
            .max_by_key(|r| action_priority(r.default_action))
            .unwrap();

        let note = matched
            .iter()
            .map(|r| format!("{}: {}", r.id, r.description))
            .collect::<Vec<_>>()
            .join("; ");

        let observer_note = if primary.default_action != Action::Allow {
            format!("[observer] would be {}: {}", primary.default_action, note)
        } else {
            note
        };

        Decision {
            event_id: event.id.clone(),
            action: Action::Allow,
            rule_id: Some(primary.id.to_string()),
            note: observer_note,
        }
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}

/// action 优先级排序：deny > ask > notify > log > allow，用于多规则匹配时取最严。
fn action_priority(action: Action) -> u8 {
    match action {
        Action::Deny => 4,
        Action::Ask => 3,
        Action::Notify => 2,
        Action::Log => 1,
        Action::Allow => 0,
    }
}

fn default_rules() -> Vec<Rule> {
    vec![
        // ---- Autonomous+ (硬红线) ----
        Rule {
            id: "D002",
            description: "git push --force → deny",
            source: RuleSource::Default,
            default_action: Action::Deny,
            matcher: d002_force_push,
            min_mode: Mode::Autonomous,
        },
        Rule {
            id: "D003",
            description: "rm -rf → deny",
            source: RuleSource::Default,
            default_action: Action::Deny,
            matcher: d003_destructive_rm,
            min_mode: Mode::Autonomous,
        },
        Rule {
            id: "D004",
            description: "sudo / privilege escalation → deny",
            source: RuleSource::Default,
            default_action: Action::Deny,
            matcher: d004_sudo,
            min_mode: Mode::Autonomous,
        },
        Rule {
            id: "D006",
            description: "sensitive file write → deny",
            source: RuleSource::Default,
            default_action: Action::Deny,
            matcher: d006_sensitive_file,
            min_mode: Mode::Autonomous,
        },
        Rule {
            id: "D007",
            description: "secret pattern in content → deny",
            source: RuleSource::Default,
            default_action: Action::Deny,
            matcher: d007_secret_in_content,
            min_mode: Mode::Autonomous,
        },
        // ---- Guard+ (默认把关) ----
        Rule {
            id: "D001",
            description: "git push to main/master → ask",
            source: RuleSource::Default,
            default_action: Action::Ask,
            matcher: d001_main_push,
            min_mode: Mode::Guard,
        },
        Rule {
            id: "D005",
            description: "bulk delete (glob or >= 5 files) → ask",
            source: RuleSource::Default,
            default_action: Action::Ask,
            matcher: d005_bulk_delete,
            min_mode: Mode::Autonomous,
        },
        Rule {
            id: "D008",
            description: "git add all / dot → ask",
            source: RuleSource::Default,
            default_action: Action::Ask,
            matcher: d008_large_batch_add,
            min_mode: Mode::Guard,
        },
        Rule {
            id: "D010",
            description: "package install → ask",
            source: RuleSource::Default,
            default_action: Action::Ask,
            matcher: d010_package_install,
            min_mode: Mode::Guard,
        },
        Rule {
            id: "D011",
            description: "curl/wget | bash/sh/shell → ask",
            source: RuleSource::Default,
            default_action: Action::Ask,
            matcher: d011_pipe_to_shell,
            min_mode: Mode::Guard,
        },
    ]
}

// D001: git push to main/master branch → ask
fn d001_main_push(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let Some(push_index) = find_git_push(&tokens) else {
        return false;
    };
    if has_force_flag(&tokens[push_index..]) {
        return false;
    }

    let positionals = positional_tokens(&tokens[push_index..]);
    positionals
        .last()
        .map(|token| is_main_ref(token))
        .unwrap_or(false)
}

// D002: git push --force / -f → deny
fn d002_force_push(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let Some(push_index) = find_git_push(&tokens) else {
        return false;
    };

    has_force_flag(&tokens[push_index..])
}

// D003: rm -rf / rm -r → deny
fn d003_destructive_rm(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let Some(rm_index) = find_command(&tokens, "rm") else {
        return false;
    };

    has_recursive_rm_flag(&tokens[rm_index + 1..])
}

// D004: sudo / privilege escalation → deny
// 只看首个 token（可执行命令），避免 echo sudo 等误判。
fn d004_sudo(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let Some(first) = tokens.first() else {
        return false;
    };
    matches!(first.as_str(), "sudo" | "sudoedit" | "pkexec" | "doas")
}

// D006: Edit/Write to sensitive files → deny
fn d006_sensitive_file(event: &UnifiedEvent) -> bool {
    let path = match event.tool_input.file_path.as_deref() {
        Some(p) => p,
        None => return false,
    };
    let sensitive = [".env", "credentials", ".pem", ".key", "secret"];
    sensitive.iter().any(|p| path.contains(p))
}

// D007: secret pattern in content/new_string → deny
fn d007_secret_in_content(event: &UnifiedEvent) -> bool {
    let patterns = [
        "api_key",
        "apikey",
        "api_secret",
        "apisecret",
        "private_key",
        "privatekey",
        "access_token",
        "accesstoken",
        "-----begin private",
        "-----begin rsa",
    ];

    let check = |s: &str| {
        let lower = s.to_lowercase();
        patterns.iter().any(|p| lower.contains(p))
    };

    event.tool_input.new_string.as_deref().map_or(false, check)
        || event.tool_input.content.as_deref().map_or(false, check)
}

// D010: package manager install → ask
// 只看首个 token（可执行命令）+ 紧接的子命令，避免 echo npm install 误判。
// D005: rm with glob or >= 5 positional args → ask
// 只看首个可执行命令，避免 echo rm *.log 等误报。
fn d005_bulk_delete(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let Some(first) = tokens.first() else {
        return false;
    };
    if first != "rm" {
        return false;
    }

    let args = &tokens[1..];
    // glob pattern indicates bulk intent
    if args.iter().any(|t| t.contains('*') || t.contains('?')) {
        return true;
    }
    // >= 5 positional files/directories
    positional_tokens(args).len() >= 5
}

// D008: git add . / -A / --all / * → ask
// 只看首个可执行命令，避免 echo git add . 等误报。
fn d008_large_batch_add(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    if tokens.len() < 2 {
        return false;
    }
    if tokens[0] != "git" || tokens[1] != "add" {
        return false;
    }
    let args = &tokens[2..];
    args.iter()
        .any(|t| t == "." || t == "-a" || t == "--all" || t == "*")
}

// D011: curl/wget/fetch piped to shell → ask
// 要求管道左侧的第一个可执行命令是 downloader，避免 echo curl | bash 等误报。
fn d011_pipe_to_shell(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };
    let pipe_pos = tokens.iter().position(|t| t == "|");
    let Some(pipe_pos) = pipe_pos else {
        return false;
    };
    let before_pipe = &tokens[..pipe_pos];
    let first_cmd = before_pipe.iter().find(|t| !t.starts_with('-'));
    let Some(first_cmd) = first_cmd else {
        return false;
    };
    let is_downloader = matches!(first_cmd.as_str(), "curl" | "wget" | "fetch");
    if !is_downloader {
        return false;
    }
    let after_pipe = &tokens[pipe_pos + 1..];
    // Only match true shell interpreters, not script languages that are often
    // used for harmless data processing (e.g. python3 -m json.tool).
    let shells = ["bash", "sh", "zsh"];
    after_pipe.iter().any(|t| shells.contains(&t.as_str()))
}

fn d010_package_install(event: &UnifiedEvent) -> bool {
    let Some(tokens) = shell_tokens(event) else {
        return false;
    };

    let first = match tokens.first() {
        Some(t) => t.as_str(),
        None => return false,
    };

    let subcmds: &[&str] = match first {
        "npm" => &["install", "i", "add"],
        "yarn" => &["add"],
        "pnpm" => &["add", "install"],
        "pip" | "pip3" => &["install"],
        "cargo" => &["install"],
        "brew" => &["install"],
        "apt" | "apt-get" => &["install"],
        "dnf" | "yum" => &["install"],
        _ => return false,
    };

    // 首个非 flag 子命令必须是 install/add
    let first_pos = tokens.iter().skip(1).find(|t| !t.starts_with('-'));
    first_pos.map_or(false, |pos| subcmds.contains(&pos.as_str()))
}

fn shell_tokens(event: &UnifiedEvent) -> Option<Vec<String>> {
    let command = event.tool_input.command.as_deref()?;
    shell_split(command).map(|tokens| {
        tokens
            .into_iter()
            .map(|token| token.to_lowercase())
            .collect()
    })
}

fn find_git_push(tokens: &[String]) -> Option<usize> {
    tokens
        .windows(2)
        .position(|window| window[0] == "git" && window[1] == "push")
        .map(|index| index + 2)
}

fn find_command(tokens: &[String], command: &str) -> Option<usize> {
    tokens.iter().position(|token| token == command)
}

fn positional_tokens<'a>(tokens: &'a [String]) -> Vec<&'a str> {
    let mut values = Vec::new();
    let mut after_double_dash = false;

    for token in tokens {
        if after_double_dash {
            values.push(token.as_str());
            continue;
        }

        if token == "--" {
            after_double_dash = true;
            continue;
        }

        if token.starts_with('-') {
            continue;
        }

        values.push(token.as_str());
    }

    values
}

fn has_force_flag(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        token == "--force" || token == "--force-with-lease" || short_flag_contains(token, 'f')
    })
}

fn has_recursive_rm_flag(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| token == "--recursive" || short_flag_contains(token, 'r'))
}

fn short_flag_contains(token: &str, expected: char) -> bool {
    token.starts_with('-')
        && !token.starts_with("--")
        && token.chars().skip(1).any(|flag| flag == expected)
}

fn is_main_ref(token: &str) -> bool {
    token == "main" || token == "master" || token.ends_with(":main") || token.ends_with(":master")
}
