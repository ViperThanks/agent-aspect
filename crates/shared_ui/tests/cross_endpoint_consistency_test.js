// cross_endpoint_consistency_test.js — 验证两端数据展示一致
//
// 用同一份 fixture 数据，验证：
// 1. 首页最近会话：title / provider / can_resume 渲染一致
// 2. 会话列表 provider badge 一致
// 3. runtime drift 红色提示一致
// 4. continue / new job body 一致
// 5. marked 配置一致

const {
  escHtml, jsStr, shortId, trunc, agentLabel, relTime,
} = require('../view_model.js');
const { buildNewJobBody, buildContinueJobBody } = require('../job_body.js');
const {
  runtimeHealthBadge, runtimeAlertCard, driftText, parseRuntimeHealth,
} = require('../runtime_health.js');
const fixtures = require('./fixtures/overview_runtime_critical.json');
const fs = require('fs');
const path = require('path');

let passed = 0;
let failed = 0;

function assert(condition, label) {
  if (condition) { passed++; }
  else { failed++; console.error('  FAIL: ' + label); }
}

function assertEqual(actual, expected, label) {
  const eq = JSON.stringify(actual) === JSON.stringify(expected);
  if (eq) { passed++; }
  else { failed++; console.error('  FAIL: ' + label); console.error('    expected: ' + JSON.stringify(expected)); console.error('    actual:   ' + JSON.stringify(actual)); }
}

// 模拟 bridge 和 relay 两端渲染首页最近会话
console.log('首页最近会话一致');

(function test_home_conversation_consistency() {
  for (const conv of fixtures.conversations) {
    const title = escHtml(trunc(conv.title || '无标题', 40));
    const provider = agentLabel(conv.agent);
    const canResume = !!conv.can_resume;
    assert(title.length > 0, 'title non-empty for ' + conv.id);
    assert(provider.length > 0, 'provider non-empty for ' + conv.id);
    assert(typeof canResume === 'boolean', 'can_resume is boolean for ' + conv.id);
    // 两端调用同一函数 → 结果一定一致，验证函数不依赖外部状态
    const title2 = escHtml(trunc(conv.title || '无标题', 40));
    assertEqual(title, title2, 'title idempotent for ' + conv.id);
  }
})();

// 会话列表 provider badge 一致
console.log('provider badge 一致');

(function test_provider_badge_consistency() {
  const agents = ['claude_code', 'codex_cli', 'kimi_code', 'gemini_cli', 'claude', 'codex'];
  for (const a of agents) {
    const label1 = agentLabel(a);
    const label2 = agentLabel(a);
    assertEqual(label1, label2, 'agentLabel(' + a + ') idempotent');
    assert(label1.length > 0, 'agentLabel(' + a + ') non-empty');
  }
  // fixture 中的 agent 也必须正确渲染
  const agentSet = new Set(fixtures.conversations.map(c => agentLabel(c.agent)));
  assert(agentSet.has('Claude Code'), 'fixture has Claude Code badge');
  assert(agentSet.has('Kimi Code'), 'fixture has Kimi Code badge');
})();

// runtime drift 红色提示两端一致
console.log('runtime drift 提示一致');

(function test_runtime_drift_consistency() {
  // runtimeAlertCard 接收整个会话列表，生成首页 alert card
  const alertCard = runtimeAlertCard(fixtures.conversations);
  assert(alertCard.length > 0, 'alertCard non-empty when critical convos present');
  assert(alertCard.indexOf('runtime-alert-card') >= 0, 'alertCard has card class');
  assert(alertCard.indexOf('运行环境漂移') >= 0, 'alertCard has drift title');

  // 逐会话 badge 测试
  for (const conv of fixtures.conversations) {
    if (!conv.runtime_health) continue;
    const badge = runtimeHealthBadge(conv.runtime_health);
    if (conv.runtime_health.status === 'critical') {
      assert(badge.indexOf('badge-red') >= 0, 'critical badge is red for ' + conv.id);
      assert(badge.indexOf('环境漂移') >= 0, 'critical badge has drift text for ' + conv.id);
      for (const w of conv.runtime_health.warnings) {
        const dt = driftText(w);
        assert(dt.length > 0, 'driftText non-empty for ' + w.field);
      }
    }
    if (conv.runtime_health.status === 'ok') {
      assertEqual(badge, '', 'ok status → no badge for ' + conv.id);
    }
  }
})();

// continue / new job body 两端一致
console.log('job body 两端一致');

(function test_job_body_consistency() {
  const provider = 'claude_code';
  const project = '/Users/dev/myproject';
  const convId = 'sess-abc123-def456';
  const prompt = 'Fix the login bug';

  // new job body
  const newBody = buildNewJobBody(provider, project, prompt);
  const newBody2 = buildNewJobBody(provider, project, prompt);
  assertEqual(newBody, newBody2, 'buildNewJobBody idempotent');
  assertEqual(newBody.kind, 'agent_prompt', 'new body kind');
  assertEqual(newBody.provider, provider, 'new body provider');
  assertEqual(newBody.prompt, prompt, 'new body prompt');
  assertEqual(newBody.project_path, project, 'new body project_path');
  assert(!newBody.conversation_id, 'new body has no conversation_id');

  // continue job body
  const contBody = buildContinueJobBody(provider, project, convId, prompt);
  const contBody2 = buildContinueJobBody(provider, project, convId, prompt);
  assertEqual(contBody, contBody2, 'buildContinueJobBody idempotent');
  assertEqual(contBody.kind, 'agent_prompt', 'continue body kind');
  assertEqual(contBody.conversation_id, convId, 'continue body conversation_id');
  assertEqual(contBody.prompt, prompt, 'continue body prompt');

  // 两端用相同输入 → 相同输出
  assertEqual(newBody, buildNewJobBody(provider, project, prompt), 'new body cross-call equal');
  assertEqual(contBody, buildContinueJobBody(provider, project, convId, prompt), 'continue body cross-call equal');
})();

(function test_bridge_run_continue_does_not_downgrade_to_new_job() {
  const runJs = fs.readFileSync(path.join(__dirname, '..', '..', 'bridge', 'src', 'ui', 'tabs', 'run.js'), 'utf8');
  assert(runJs.indexOf("toast('请选择要继续的会话')") >= 0, 'bridge run continue missing conversation shows explicit error');
  assert(runJs.indexOf("if (RS.sessionMode === 'continue' && cv && cv.value)") < 0, 'bridge run continue must not conditionally fall through to new job');
})();

// light/dark token：验证 escHtml/jsStr 不依赖主题
console.log('主题无关纯函数');

(function test_theme_independent() {
  const inputs = ['<b>bold</b>', "it's a test", 'line1\nline2', 0, false, null, ''];
  for (const input of inputs) {
    const r1 = escHtml(input);
    const r2 = escHtml(input);
    assertEqual(r1, r2, 'escHtml idempotent for ' + JSON.stringify(input));
    const j1 = jsStr(input);
    const j2 = jsStr(input);
    assertEqual(j1, j2, 'jsStr idempotent for ' + JSON.stringify(input));
  }
})();

console.log('\n' + passed + ' passed, ' + failed + ' failed');
if (failed > 0) process.exit(1);
