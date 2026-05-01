// render.js — 共享 Markdown 渲染层
//
// 职责：封装 marked.parse，统一 markdown 渲染行为。
// bridge 和 relay 都使用同一份渲染逻辑，不允许各自实现。
//
// 依赖：marked.min.js（在注入顺序中排在本文件之前）
// 依赖：view_model.js 中的 escHtml（用于 marked 不可用时的 fallback）

// Node.js 环境：加载依赖（浏览器环境由注入顺序保证）
if (typeof module !== 'undefined' && module.exports) {
  var vm = require('./view_model.js');
  var escHtml = vm.escHtml;
  var jsStr = vm.jsStr;
}

// 统一 marked 配置（bridge + relay 必须一致）
if (typeof marked !== 'undefined') {
  marked.setOptions({ gfm: true, breaks: true });
}

// ============================================================
// Markdown 渲染
// ============================================================

/**
 * 将 markdown 文本渲染为 HTML。
 * 使用 marked.parse(GFM + breaks)，并在代码块旁添加 Copy 按钮。
 * 如果 marked 未加载，fallback 到 escHtml（纯文本转义）。
 *
 * @param {string} text - markdown 源文本
 * @returns {string} HTML 字符串
 */
function renderMd(text) {
  if (!text) return '';
  if (typeof marked === 'undefined') return typeof escHtml === 'function' ? escHtml(text) : String(text);
  var html = marked.parse(text);
  // 为代码块添加复制按钮
  html = html.replace(/<pre><code([^>]*)>/g, function (m, attrs) {
    return '<div class="md-code-wrap"><button class="code-copy-btn" onclick="copyCodeBlock(this)">Copy</button><pre><code' + attrs + '>';
  });
  html = html.replace(/<\/code><\/pre>/g, '</code></pre></div>');
  return html;
}

// ============================================================
// 代码块复制
// ============================================================

/**
 * 复制代码块内容到剪贴板。
 * 由 renderMd 注入的 Copy 按钮调用。
 *
 * @param {HTMLElement} btn - 触发按钮
 */
function copyCodeBlock(btn) {
  var pre = btn.parentElement.querySelector('code');
  if (pre) {
    navigator.clipboard.writeText(pre.textContent).then(function () {
      btn.textContent = '已复制';
      setTimeout(function () { btn.textContent = 'Copy'; }, 1500);
    }).catch(function () {});
  }
}

// ============================================================
// 通用 UI 组件（bridge + relay 共用）
// ============================================================

/**
 * 复制文本到剪贴板并显示 toast。
 *
 * @param {string} text
 */
function copyText(text) {
  if (!text) return;
  navigator.clipboard.writeText(text).then(function () {
    if (typeof toast === 'function') toast('已复制');
  }).catch(function () {});
}

/**
 * 生成复制按钮 HTML。
 *
 * @param {string} text - 要复制的文本
 * @param {string} [label] - 可选标签
 * @returns {string}
 */
function copyButton(text, label) {
  var lbl = label !== undefined ? label : '';
  return '<button class="icon-btn" onclick="event.stopPropagation();copyText(\'' + (typeof jsStr === 'function' ? jsStr(text) : text) + '\')" title="复制' + (lbl ? ' ' + (typeof escHtml === 'function' ? escHtml(lbl) : lbl) : '') + '">' + copyIcon() + '</button>';
}

/**
 * 复制图标 SVG。
 *
 * @returns {string}
 */
function copyIcon() {
  return '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>';
}

// ============================================================
// 导出
// ============================================================
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    renderMd: renderMd,
    copyCodeBlock: copyCodeBlock,
    copyText: copyText,
    copyButton: copyButton,
    copyIcon: copyIcon,
  };
}
