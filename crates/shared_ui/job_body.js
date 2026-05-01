// job_body.js — job request body 的唯一构造源
//
// 职责：定义「新建会话」和「继续会话」两个不可混淆的业务原语。
// 所有 UI 入口（Run tab 提交、会话详情继续、会话详情新建跳转后提交）
// 都必须调用此模块的函数，不允许自行拼装 job body。
//
// 不变量：
// - 新建任务 (buildNewJobBody) 返回的 body 绝不包含 conversation_id
// - 继续任务 (buildContinueJobBody) 缺少 conversation_id 时必须 throw，
//   不允许静默降级为新建
// - UI 状态（can_resume / disabled 等）只能影响按钮展示，
//   不能改变 body 的语义
//
// 环境兼容：
// - 浏览器：被 mobile_ui.rs include_str! 后注入 <script>，函数挂全局
// - Node.js：通过 module.exports 导出，供 app_test.js 直接测生产代码

/**
 * 构造「新建会话」的 POST /api/jobs body。
 *
 * @param {string} provider  - 代理标识 (claude_code | kimi_code | codex_cli)
 * @param {string} projectPath - 项目目录，可为空
 * @param {string} prompt    - 用户提示词，不可为空
 * @returns {object} 不含 conversation_id 的 job body
 *
 * 不变量：返回对象永远不含 conversation_id 字段。
 * 如果后端收到 conversation_id，那一定不是从这条路进去的。
 */
function buildNewJobBody(provider, projectPath, prompt) {
  var body = { kind: 'agent_prompt', prompt: prompt, provider: provider };
  if (projectPath) body.project_path = projectPath;
  return body;
}

/**
 * 构造「继续会话」的 POST /api/jobs body。
 *
 * @param {string} provider       - 代理标识
 * @param {string} projectPath    - 项目目录，可为空
 * @param {string} conversationId - 要继续的会话 ID，不可为空
 * @param {string} prompt         - 用户提示词
 * @returns {object} 包含 conversation_id 的 job body
 * @throws {Error} conversationId 为空时抛错，防止静默降级为新建
 *
 * 不变量：返回对象永远包含 conversation_id。
 * 调用方不需要也不允许按 provider 或 can_resume 判断是否传入；
 * 后端收到后由 provider command builder 决定如何使用。
 */
function buildContinueJobBody(provider, projectPath, conversationId, prompt) {
  if (!conversationId) {
    throw new Error('buildContinueJobBody: conversation_id is required — use buildNewJobBody for new conversations');
  }
  return {
    kind: 'agent_prompt',
    provider: provider,
    project_path: projectPath || undefined,
    conversation_id: conversationId,
    prompt: prompt,
  };
}

// Node.js 环境导出（测试用）；浏览器环境忽略
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { buildNewJobBody: buildNewJobBody, buildContinueJobBody: buildContinueJobBody };
}
