// job_body_test.js — 共享 job_body 模块测试
//
// 验证 buildNewJobBody / buildContinueJobBody 的不变量。

const { buildNewJobBody, buildContinueJobBody } = require('../job_body.js');

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

function assertThrows(fn, label) {
  try { fn(); failed++; console.error('  FAIL: ' + label); }
  catch (_) { passed++; }
}

// ---- buildNewJobBody ----

console.log('buildNewJobBody');

(function test_no_conversation_id() {
  const body = buildNewJobBody('claude_code', '/tmp/proj', 'hello');
  assert(!('conversation_id' in body), 'new body must NOT contain conversation_id');
  assertEqual(body.kind, 'agent_prompt', 'kind is agent_prompt');
  assertEqual(body.provider, 'claude_code', 'provider set');
  assertEqual(body.prompt, 'hello', 'prompt set');
  assertEqual(body.project_path, '/tmp/proj', 'project_path set');
})();

(function test_empty_project_excluded() {
  const body = buildNewJobBody('kimi_code', '', 'test');
  assert(!('project_path' in body), 'empty project_path excluded');
})();

(function test_null_project_excluded() {
  const body = buildNewJobBody('codex_cli', null, 'fix');
  assert(!('project_path' in body), 'null project_path excluded');
})();

(function test_codex_provider() {
  const body = buildNewJobBody('codex_cli', '/tmp', 'fix');
  assertEqual(body.provider, 'codex_cli', 'codex provider');
  assert(!('conversation_id' in body), 'no conversation_id');
})();

// ---- buildContinueJobBody ----

console.log('buildContinueJobBody');

(function test_has_conversation_id() {
  const body = buildContinueJobBody('claude_code', '/tmp', 'sess-123', 'go');
  assertEqual(body.conversation_id, 'sess-123', 'conversation_id present');
  assertEqual(body.kind, 'agent_prompt', 'kind');
  assertEqual(body.provider, 'claude_code', 'provider');
})();

(function test_null_project_undefined() {
  const body = buildContinueJobBody('claude_code', null, 'sess-1', 'hi');
  assertEqual(body.project_path, undefined, 'null → undefined');
  assertEqual(body.conversation_id, 'sess-1', 'conversation_id set');
})();

(function test_missing_conversation_id_throws() {
  assertThrows(() => buildContinueJobBody('codex_cli', '/tmp', '', 'x'), 'empty → throw');
  assertThrows(() => buildContinueJobBody('codex_cli', '/tmp', null, 'x'), 'null → throw');
  assertThrows(() => buildContinueJobBody('codex_cli', '/tmp', undefined, 'x'), 'undefined → throw');
})();

// ---- Summary ----

console.log('\n' + passed + ' passed, ' + failed + ' failed');
if (failed > 0) process.exit(1);
