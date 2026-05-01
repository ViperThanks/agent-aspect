//! 决策类型 — `Action` 枚举和 `Decision` 记录。
//!
//! `Action` 是规则引擎的输出：allow / deny / ask / notify / log。
//! `Decision` 将 action 关联到具体 event_id，持久化到 audit store。

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
    Ask,
    Notify,
    Log,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Ask => "ask",
            Self::Notify => "notify",
            Self::Log => "log",
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub event_id: String,
    pub action: Action,
    pub rule_id: Option<String>,
    pub note: String,
}
