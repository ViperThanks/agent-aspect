// css_token_integrity_test.js — 验证端侧 CSS 不覆盖 design_tokens 变量
//
// 读取 design_tokens.css 中定义的 :root 变量名，
// 检查 bridge styles.css 和 relay style.css 不会在 :root 中重新定义这些变量。

const fs = require('fs');
const path = require('path');

let passed = 0;
let failed = 0;

function assert(condition, label) {
  if (condition) { passed++; }
  else { failed++; console.error('  FAIL: ' + label); }
}

const tokensPath = path.join(__dirname, '..', 'design_tokens.css');
const bridgeCssPath = path.join(__dirname, '..', '..', 'bridge', 'src', 'ui', 'styles.css');
const relayCssPath = path.join(__dirname, '..', '..', 'relay', 'src', 'ui', 'style.css');

// 从 design_tokens.css 提取所有 :root 变量名
function extractTokenNames(css) {
  const names = new Set();
  const rootBlock = css.match(/:root\s*\{([^}]+)\}/);
  if (!rootBlock) return names;
  const varMatches = rootBlock[1].matchAll(/--[\w-]+/g);
  for (const m of varMatches) {
    names.add(m[0]);
  }
  return names;
}

// 从端侧 CSS 提取 :root 中的变量名（排除 fallback defaults）
function extractRootVarNames(css) {
  const names = new Set();
  const rootBlocks = css.matchAll(/:root\s*\{([^}]+)\}/g);
  for (const block of rootBlocks) {
    const varMatches = block[1].matchAll(/(--[\w-]+)\s*:/g);
    for (const m of varMatches) {
      names.add(m[1]);
    }
  }
  return names;
}

console.log('CSS token integrity');

const tokensCss = fs.readFileSync(tokensPath, 'utf8');
const bridgeCss = fs.readFileSync(bridgeCssPath, 'utf8');
const relayCss = fs.readFileSync(relayCssPath, 'utf8');

const tokenNames = extractTokenNames(tokensCss);

// bridge :root 不重定义 token 变量（允许 layout 变量如 --sidebar-w）
(function test_bridge_no_token_override() {
  const bridgeRootVars = extractRootVarNames(bridgeCss);
  const overrides = [...bridgeRootVars].filter(v => tokenNames.has(v));
  assert(overrides.length === 0, 'bridge :root 不覆盖 design token 变量 (found: ' + overrides.join(', ') + ')');
})();

// relay :root 不重定义 token 变量
(function test_relay_no_token_override() {
  const relayRootVars = extractRootVarNames(relayCss);
  const overrides = [...relayRootVars].filter(v => tokenNames.has(v));
  assert(overrides.length === 0, 'relay :root 不覆盖 design token 变量 (found: ' + overrides.join(', ') + ')');
})();

// light mode token 定义完整：必须覆盖关键变量
(function test_light_mode_completeness() {
  const criticalTokens = ['--bg', '--surface', '--text', '--accent', '--red', '--green', '--yellow', '--border'];
  const lightBlock = tokensCss.match(/\[data-theme="light"\]\s*\{([^}]+)\}/);
  assert(lightBlock !== null, 'design_tokens.css 包含 light mode block');
  if (lightBlock) {
    const lightVars = lightBlock[1];
    for (const t of criticalTokens) {
      assert(lightVars.indexOf(t + ':') >= 0 || lightVars.indexOf(t + ':') >= 0,
        'light mode 定义 ' + t);
    }
  }
})();

// 两端都引用了 design_tokens.css
(function test_design_tokens_injection() {
  const bridgeUiRs = fs.readFileSync(path.join(__dirname, '..', '..', 'bridge', 'src', 'ui.rs'), 'utf8');
  const relayMobileUiRs = fs.readFileSync(path.join(__dirname, '..', '..', 'relay', 'src', 'mobile_ui.rs'), 'utf8');
  assert(bridgeUiRs.indexOf('design_tokens.css') >= 0, 'bridge ui.rs 注入 design_tokens.css');
  assert(relayMobileUiRs.indexOf('design_tokens.css') >= 0, 'relay mobile_ui.rs 注入 design_tokens.css');
})();

// 端侧 CSS 不包含硬编码颜色（抽查关键选择器）
(function test_no_hardcoded_colors_in_shared_selectors() {
  const bridgeHardcoded = bridgeCss.match(/\.badge-deny\s*\{[^}]*#[0-9a-fA-F]{3,8}/);
  assert(!bridgeHardcoded, 'bridge .badge-deny 使用 token');
  const relayHardcoded = relayCss.match(/\.badge-red\s*\{[^}]*#[0-9a-fA-F]{3,8}/);
  assert(!relayHardcoded, 'relay .badge-red 使用 token');
})();

console.log('\n' + passed + ' passed, ' + failed + ' failed');
if (failed > 0) process.exit(1);
