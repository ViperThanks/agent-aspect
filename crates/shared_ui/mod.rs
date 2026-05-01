//! 共享前端 UI 模块 — 编译时验证。
//!
//! 职责：通过 include_str! 确保所有共享 JS 文件存在且 UTF-8 合法。
//! 不产生 HTML，仅供 bridge/relay 引用时做编译期校验。

/// marked.min.js — vendored marked v15
pub const MARKED_JS: &str = include_str!("marked.min.js");

/// view_model.js — escHtml, jsStr, shortId, trunc, formatTime, relTime, agentLabel, toast, cleanAgentLogChunk
pub const VIEW_MODEL_JS: &str = include_str!("view_model.js");

/// render.js — renderMd, copyCodeBlock, copyText, copyButton
pub const RENDER_JS: &str = include_str!("render.js");

/// api_client.js — api(), apiJson(), apiPost()
pub const API_CLIENT_JS: &str = include_str!("api_client.js");

/// job_body.js — buildNewJobBody, buildContinueJobBody
pub const JOB_BODY_JS: &str = include_str!("job_body.js");

/// runtime_health.js — runtimeHealthBadge, runtimeAlertCard, runtimeHealthBanner, driftText
pub const RUNTIME_HEALTH_JS: &str = include_str!("runtime_health.js");

/// activity_segment.js — buildSegments, buildTurnGroups, renderSegmentCard, renderTurnBanner
pub const ACTIVITY_SEGMENT_JS: &str = include_str!("activity_segment.js");
