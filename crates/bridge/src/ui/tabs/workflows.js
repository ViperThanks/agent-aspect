/* workflows.js — Workflows tab: chain execution engine UI */

const WFS = {
  list: [],
  selected: null,
  pollTimer: null,
  createOpen: false,
  editing: false,
  tplName: '',
  tplDesc: '',
  steps: [{ provider: 'claude_code', fallback_provider: '', retry_budget: 0, project_path: '', prompt: '', context_strategy: 'none' }]
};

const WF_TEMPLATES = [
  {
    id: 'code-review',
    name: '代码审查链',
    description: '先运行测试，再审查代码变更',
    steps: [
      { provider: 'claude_code', project_path: '', prompt: '运行 cargo test 并报告测试结果', context_strategy: 'none' },
      { provider: 'claude_code', project_path: '', prompt: '根据测试结果，审查最近的代码变更，检查潜在问题', context_strategy: 'last_50_lines' }
    ]
  },
  {
    id: 'test-deploy',
    name: '测试 + 构建验证',
    description: '冒烟测试后验证构建',
    steps: [
      { provider: 'claude_code', project_path: '', prompt: '运行 scripts/smoke_test.sh 并报告结果', context_strategy: 'none' },
      { provider: 'claude_code', project_path: '', prompt: '运行 cargo build --release 并报告构建结果', context_strategy: 'last_50_lines' }
    ]
  },
  {
    id: 'refactor-chain',
    name: '重构链',
    description: '分析 → 重构 → 测试验证',
    steps: [
      { provider: 'claude_code', project_path: '', prompt: '分析当前代码结构，识别可以重构的模块', context_strategy: 'none' },
      { provider: 'claude_code', project_path: '', prompt: '根据分析结果执行重构，保持功能不变', context_strategy: 'last_100_lines' },
      { provider: 'claude_code', project_path: '', prompt: '运行测试验证重构没有破坏现有功能', context_strategy: 'last_50_lines' }
    ]
  }
];

window.WFS = WFS;

/* ---------- Layout ---------- */
function ensureWorkflowLayout() {
  const view = document.getElementById('workflows-view');
  if (!view || document.getElementById('wf-layout')) return;

  view.innerHTML =
    '<div id="wf-layout" class="wf-layout">' +
      '<div class="wf-header">' +
        '<h2>工作流</h2>' +
        '<button class="btn btn-primary btn-sm" onclick="toggleWfCreate()">新建工作流</button>' +
      '</div>' +
      '<div id="wf-create-form" class="wf-create-form hidden"></div>' +
      '<div class="wf-body">' +
        '<div id="wf-list-panel" class="wf-list-panel"></div>' +
        '<div id="wf-detail-panel" class="wf-detail-panel"></div>' +
      '</div>' +
    '</div>';

  renderWfCreateForm();
}

/* ---------- Create Form ---------- */
function renderWfCreateForm() {
  const el = document.getElementById('wf-create-form');
  if (!el) return;

  let stepsHtml = '';
  WFS.steps.forEach((s, i) => {
    stepsHtml +=
      '<div class="wf-step-editor">' +
        '<span class="wf-step-editor-num">' + (i + 1) + '</span>' +
        '<div class="wf-step-editor-body">' +
          '<div class="wf-step-editor-row">' +
            '<select class="select wf-step-provider" data-idx="' + i + '">' +
              '<option value="claude_code"' + (s.provider === 'claude_code' ? ' selected' : '') + '>Claude Code</option>' +
              '<option value="kimi_code"' + (s.provider === 'kimi_code' ? ' selected' : '') + '>Kimi Code</option>' +
              '<option value="codex_cli"' + (s.provider === 'codex_cli' ? ' selected' : '') + '>Codex CLI</option>' +
            '</select>' +
            '<select class="select wf-step-fallback" data-idx="' + i + '" style="width:150px">' +
              '<option value=""' + (!s.fallback_provider ? ' selected' : '') + '>无 fallback</option>' +
              '<option value="claude_code"' + (s.fallback_provider === 'claude_code' ? ' selected' : '') + '>fallback Claude</option>' +
              '<option value="kimi_code"' + (s.fallback_provider === 'kimi_code' ? ' selected' : '') + '>fallback Kimi</option>' +
              '<option value="codex_cli"' + (s.fallback_provider === 'codex_cli' ? ' selected' : '') + '>fallback Codex</option>' +
            '</select>' +
            '<input class="input wf-step-retry" data-idx="' + i + '" type="number" min="0" max="5" placeholder="retry" value="' + esc(s.retry_budget || 0) + '" style="width:72px">' +
            '<select class="select wf-step-ctx" data-idx="' + i + '" style="width:140px">' +
              '<option value="none"' + (s.context_strategy === 'none' ? ' selected' : '') + '>无上下文</option>' +
              '<option value="last_50_lines"' + (s.context_strategy === 'last_50_lines' ? ' selected' : '') + '>最后 50 行</option>' +
              '<option value="last_100_lines"' + (s.context_strategy === 'last_100_lines' ? ' selected' : '') + '>最后 100 行</option>' +
              '<option value="full_log"' + (s.context_strategy === 'full_log' ? ' selected' : '') + '>完整日志</option>' +
            '</select>' +
          '</div>' +
          '<input class="input wf-step-project" data-idx="' + i + '" placeholder="项目路径（可选）" value="' + esc(s.project_path) + '">' +
          '<textarea class="textarea wf-step-prompt" data-idx="' + i + '" placeholder="步骤提示词..." style="min-height:60px">' + esc(s.prompt) + '</textarea>' +
        '</div>' +
        (WFS.steps.length > 1 ? '<button class="icon-btn" onclick="removeWfStep(' + i + ')" title="删除" style="margin-top:4px"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></button>' : '') +
      '</div>';
  });

  el.innerHTML =
    '<div style="display:flex;flex-direction:column;gap:10px">' +
      '<div style="display:flex;gap:8px;align-items:center">' +
        '<input class="input" id="wf-name" placeholder="工作流名称" style="flex:1" value="' + esc(WFS.tplName) + '">' +
        '<select class="select" id="wf-template" onchange="applyWfTemplate()" style="width:140px">' +
          '<option value="">从模板创建...</option>' +
          WF_TEMPLATES.map(t => '<option value="' + t.id + '">' + esc(t.name) + '</option>').join('') +
        '</select>' +
      '</div>' +
      '<input class="input" id="wf-desc" placeholder="描述（可选）" value="' + esc(WFS.tplDesc) + '">' +
      '<div style="font-size:.8rem;color:var(--dim)">步骤（按顺序执行）</div>' +
      '<div id="wf-steps-list">' + stepsHtml + '</div>' +
      '<div style="display:flex;gap:8px">' +
        '<button class="btn btn-sm" onclick="addWfStep()">添加步骤</button>' +
        '<button class="btn btn-primary btn-sm" onclick="submitCreateWf()" style="margin-left:auto">创建</button>' +
        '<button class="btn btn-sm" onclick="toggleWfCreate()">取消</button>' +
      '</div>' +
    '</div>';
}

function toggleWfCreate() {
  WFS.createOpen = !WFS.createOpen;
  const el = document.getElementById('wf-create-form');
  if (el) el.classList.toggle('hidden', !WFS.createOpen);
  if (WFS.createOpen) {
    WFS.tplName = '';
    WFS.tplDesc = '';
    renderWfCreateForm();
  }
}

function applyWfTemplate() {
  const sel = document.getElementById('wf-template');
  if (!sel || !sel.value) return;
  const tpl = WF_TEMPLATES.find(t => t.id === sel.value);
  if (!tpl) return;

  WFS.tplName = tpl.name;
  WFS.tplDesc = tpl.description;
  WFS.steps = tpl.steps.map(s => ({...s}));
  renderWfCreateForm();
}

function addWfStep() {
  syncWfStepsFromDom();
  WFS.steps.push({ provider: 'claude_code', fallback_provider: '', retry_budget: 0, project_path: '', prompt: '', context_strategy: 'none' });
  renderWfCreateForm();
}

function removeWfStep(idx) {
  syncWfStepsFromDom();
  WFS.steps.splice(idx, 1);
  renderWfCreateForm();
}

function syncWfStepsFromDom() {
  document.querySelectorAll('.wf-step-provider').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].provider = el.value;
  });
  document.querySelectorAll('.wf-step-ctx').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].context_strategy = el.value;
  });
  document.querySelectorAll('.wf-step-fallback').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].fallback_provider = el.value;
  });
  document.querySelectorAll('.wf-step-retry').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].retry_budget = Math.max(0, Math.min(5, parseInt(el.value || '0', 10) || 0));
  });
  document.querySelectorAll('.wf-step-project').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].project_path = el.value;
  });
  document.querySelectorAll('.wf-step-prompt').forEach(el => {
    const i = parseInt(el.dataset.idx);
    if (WFS.steps[i]) WFS.steps[i].prompt = el.value;
  });
}

function submitCreateWf() {
  syncWfStepsFromDom();
  const name = (document.getElementById('wf-name') || {}).value || '';
  const desc = (document.getElementById('wf-desc') || {}).value || '';
  if (!name.trim()) { toast('请输入名称'); return; }
  if (WFS.steps.length === 0) { toast('至少需要一个步骤'); return; }
  for (const s of WFS.steps) {
    if (!s.prompt.trim()) { toast('步骤提示词不能为空'); return; }
  }

  const advanceMode = (document.getElementById('wf-advance-mode') || {}).value || 'auto';
  api('/workflows', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: name.trim(), description: desc.trim(), advance_mode: advanceMode, steps: WFS.steps })
  }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.id) {
      toast('工作流已创建');
      WFS.createOpen = false;
      const el = document.getElementById('wf-create-form');
      if (el) el.classList.add('hidden');
      WFS.tplName = '';
      WFS.tplDesc = '';
      WFS.steps = [{ provider: 'claude_code', fallback_provider: '', retry_budget: 0, project_path: '', prompt: '', context_strategy: 'none' }];
      loadWorkflowList();
      selectWorkflow(data.id);
    } else {
      toast('创建失败');
    }
  });
}

/* ---------- List ---------- */
function loadWorkflowList() {
  return api('/workflows?limit=50').then(data => {
    if (data.error) return;
    WFS.list = data.workflows || [];
    renderWfList();
  });
}

function renderWfList() {
  const el = document.getElementById('wf-list-panel');
  if (!el) return;

  if (WFS.list.length === 0) {
    el.innerHTML = '<div class="wf-empty">暂无工作流</div>';
    return;
  }

  el.innerHTML = WFS.list.map(wf => {
    const selected = WFS.selected && WFS.selected.id === wf.id;
    const badge = wfStatusBadge(wf.status);
    const counts = wf.step_counts || {};
    return '<div class="wf-card' + (selected ? ' wf-card-selected' : '') + '" onclick="selectWorkflow(\'' + jsStr(wf.id) + '\')">' +
      '<div class="wf-card-title">' + esc(wf.name) + badge + '</div>' +
      (wf.description ? '<div class="wf-card-desc">' + esc(wf.description) + '</div>' : '') +
      '<div class="wf-card-meta">' +
        (counts.total || 0) + ' 步 · ' +
        '<span style="color:var(--green)">' + (counts.succeeded || 0) + ' 完成</span>' +
        (counts.failed ? ' · <span style="color:var(--red)">' + counts.failed + ' 失败</span>' : '') +
      '</div>' +
    '</div>';
  }).join('');
}

/* ---------- Detail ---------- */
function selectWorkflow(id) {
  api('/workflows/' + id).then(data => {
    if (data.error) { toast(data.error); return; }
    WFS.selected = data;
    WFS.editing = false;
    renderWfList();
    renderWfDetail();
  });
}

function renderWfDetail() {
  const el = document.getElementById('wf-detail-panel');
  if (!el || !WFS.selected) {
    if (el) el.innerHTML = '<div class="wf-empty">选择一个工作流</div>';
    return;
  }

  const wf = WFS.selected;
  const badge = wfStatusBadge(wf.status);
  const canRun = wf.status === 'draft';
  const canRetry = wf.status === 'failed' || wf.status === 'cancelled' || wf.status === 'paused';
  const canCancel = wf.status === 'running';
  const canEdit = wf.status !== 'running' && wf.status !== 'paused';
  const canAdvance = wf.status === 'paused' && wf.advance_mode === 'manual';

  // 编辑表单（toggle）
  let editHtml = '';
  if (WFS.editing) {
    editHtml =
      '<div style="margin-bottom:16px;padding:12px;background:var(--surface);border-radius:8px">' +
        '<input class="input" id="wf-edit-name" value="' + esc(wf.name) + '" style="margin-bottom:8px">' +
        '<input class="input" id="wf-edit-desc" value="' + esc(wf.description) + '" placeholder="描述（可选）" style="margin-bottom:8px">' +
        '<select class="select" id="wf-edit-advance-mode" style="margin-bottom:8px;width:160px">' +
          '<option value="auto"' + (wf.advance_mode === 'auto' ? ' selected' : '') + '>自动推进</option>' +
          '<option value="manual"' + (wf.advance_mode === 'manual' ? ' selected' : '') + '>手动推进</option>' +
        '</select>' +
        '<div style="display:flex;gap:8px">' +
          '<button class="btn btn-primary btn-sm" onclick="saveWfEdit(\'' + jsStr(wf.id) + '\')">保存</button>' +
          '<button class="btn btn-sm" onclick="cancelWfEdit()">取消</button>' +
        '</div>' +
      '</div>';
  }

  // 步骤列表（可拖拽）
  let stepsHtml = (wf.steps || []).map((s, i) => {
    const stepBadge = stepStatusBadge(s.status);
    const numClass = 'wf-step-number ' + (s.status || 'pending');
    const connClass = 'wf-step-connector ' + (s.status === 'succeeded' ? 'done' : '');
    const draggable = canEdit ? ' draggable="true" data-step-id="' + esc(s.id) + '" data-step-idx="' + i + '"' : '';
    const providerLabel = AGENTS[s.provider] || s.provider || 'unknown';
    const attemptText = 'attempt ' + (s.attempt || 1) + '/' + (s.max_attempts || 1);
    const retryText = (s.retry_budget || 0) > 0 ? ' · retry ' + s.retry_budget : '';
    const fallbackText = s.fallback_provider ? ' · fallback ' + (AGENTS[s.fallback_provider] || s.fallback_provider) : '';
    const deadlineText = s.hard_deadline_at ? ' · deadline ' + formatTime(s.hard_deadline_at) : '';
    const ctxText = ((s.input_context_bytes || 0) || (s.output_context_bytes || 0))
      ? ' · ctx ' + formatWfBytes(s.input_context_bytes || 0) + '/' + formatWfBytes(s.output_context_bytes || 0)
      : '';
    const failureText = s.failure_class ? ' · ' + esc(s.failure_class) : '';
    const attemptHistory = renderWfAttempts(s.attempts || []);
    return '<div class="wf-step-item"' + draggable + '>' +
      '<div class="wf-step-timeline">' +
        (canEdit ? '<div style="cursor:grab;color:var(--dim);font-size:.7rem;margin-bottom:2px">⋮⋮</div>' : '') +
        '<div class="' + numClass + '" style="background:' + stepColor(s.status) + '">' + (i + 1) + '</div>' +
        (i < wf.steps.length - 1 ? '<div class="' + connClass + '"></div>' : '') +
      '</div>' +
      '<div class="wf-step-content">' +
        '<div class="wf-step-head">' +
          '<span class="wf-step-provider">' + esc(providerLabel) + '</span>' +
          stepBadge +
          (s.context_strategy !== 'none' ? '<span class="wf-step-ctx">' + esc(s.context_strategy) + '</span>' : '') +
        '</div>' +
        (s.project_path ? '<div class="wf-step-path">' + esc(s.project_path) + '</div>' : '') +
        '<div class="wf-step-prompt">' + esc(s.prompt) + '</div>' +
        '<div class="wf-step-job">' + esc(attemptText + retryText + fallbackText + deadlineText) + ctxText + failureText + '</div>' +
        attemptHistory +
        (s.job_id ? '<div class="wf-step-job">Job: ' + esc(s.job_id.substring(0, 8)) + '… <button class="btn btn-sm" style="font-size:.68rem;padding:1px 6px" onclick="toggleStepLogs(\'' + jsStr(s.id) + '\',\'' + jsStr(wf.id) + '\')">日志</button></div>' : '') +
        '<div id="step-logs-' + esc(s.id) + '"></div>' +
      '</div>' +
    '</div>';
  }).join('');

  el.innerHTML =
    '<div class="wf-detail-header">' +
      '<h3>' + esc(wf.name) + '</h3>' +
      badge +
      '<div class="wf-detail-actions">' +
        (canRun ? '<button class="btn btn-primary btn-sm" onclick="runWorkflow(\'' + jsStr(wf.id) + '\')">执行</button>' : '') +
        (canRetry ? '<button class="btn btn-primary btn-sm" onclick="runWorkflow(\'' + jsStr(wf.id) + '\')">重试</button>' : '') +
        (canAdvance ? '<button class="btn btn-primary btn-sm" onclick="advanceWorkflow(\'' + jsStr(wf.id) + '\')">下一步</button>' : '') +
        (canCancel ? '<button class="btn btn-sm" style="color:var(--red)" onclick="cancelWorkflow(\'' + jsStr(wf.id) + '\')">取消</button>' : '') +
        (canEdit ? '<button class="btn btn-sm" onclick="startWfEdit()">编辑</button>' : '') +
        (canEdit ? '<button class="btn btn-sm" style="color:var(--red)" onclick="deleteWorkflow(\'' + jsStr(wf.id) + '\')">删除</button>' : '') +
      '</div>' +
    '</div>' +
    editHtml +
    (wf.description && !WFS.editing ? '<p style="color:var(--dim);margin:0 0 16px;font-size:.85rem">' + esc(wf.description) + '</p>' : '') +
    '<div style="font-size:.78rem;color:var(--dim);margin-bottom:16px">创建于 ' + formatTime(wf.created_at) + ' · 模式: ' + (wf.advance_mode === 'manual' ? '<span style="color:var(--yellow)">手动推进</span>' : '自动推进') + '</div>' +
    '<div style="font-size:.85rem;font-weight:500;margin-bottom:12px">步骤</div>' +
    '<div class="wf-step-list">' + stepsHtml + '</div>';

  // 绑定拖拽事件
  if (canEdit) initStepDragDrop();
}

/* ---------- Actions ---------- */
function runWorkflow(id) {
  api('/workflows/' + id + '/run', { method: 'POST' }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.status === 'running') {
      toast('工作流已开始执行');
      selectWorkflow(id);
      startWfPolling();
    } else {
      toast(data.error || '执行失败');
    }
  });
}

function advanceWorkflow(id) {
  api('/workflows/' + id + '/next-step', { method: 'POST' }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.status === 'signal_queued') {
      toast('已发送推进信号');
      selectWorkflow(id);
    } else {
      toast(data.error || '推进失败');
    }
  });
}

function cancelWorkflow(id) {
  api('/workflows/' + id + '/cancel', { method: 'POST' }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.status === 'cancelled') {
      toast('工作流已取消');
      selectWorkflow(id);
      stopWfPolling();
    } else {
      toast(data.error || '取消失败');
    }
  });
}

/* ---------- Edit/Delete ---------- */
function startWfEdit() {
  WFS.editing = true;
  renderWfDetail();
}

function cancelWfEdit() {
  WFS.editing = false;
  renderWfDetail();
}

function saveWfEdit(id) {
  const name = (document.getElementById('wf-edit-name') || {}).value || '';
  const desc = (document.getElementById('wf-edit-desc') || {}).value || '';
  const advanceMode = (document.getElementById('wf-edit-advance-mode') || {}).value || '';
  if (!name.trim()) { toast('名称不能为空'); return; }

  const payload = { name: name.trim(), description: desc.trim() };
  if (advanceMode) payload.advance_mode = advanceMode;

  api('/workflows/' + id, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload)
  }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.status === 'updated') {
      toast('已保存');
      WFS.editing = false;
      loadWorkflowList();
      selectWorkflow(id);
    } else {
      toast(data.error || '保存失败');
    }
  });
}

function deleteWorkflow(id) {
  if (!confirm('确定删除此工作流？此操作不可恢复。')) return;

  api('/workflows/' + id, { method: 'DELETE' }).then(data => {
    if (data.error) { toast(data.error); return; }
    if (data.status === 'deleted') {
      toast('已删除');
      WFS.selected = null;
      WFS.editing = false;
      loadWorkflowList();
      const detail = document.getElementById('wf-detail-panel');
      if (detail) detail.innerHTML = '<div class="wf-empty">选择一个工作流</div>';
    } else {
      toast(data.error || '删除失败');
    }
  });
}

/* ---------- Step Drag & Drop ---------- */
function initStepDragDrop() {
  const container = document.getElementById('wf-steps-list');
  if (!container) return;

  let dragId = null;

  container.querySelectorAll('.wf-step-item[draggable]').forEach(item => {
    item.addEventListener('dragstart', e => {
      dragId = item.dataset.stepId;
      item.style.opacity = '0.4';
      e.dataTransfer.effectAllowed = 'move';
    });

    item.addEventListener('dragend', () => {
      dragId = null;
      item.style.opacity = '1';
      container.querySelectorAll('.wf-step-item').forEach(el => el.style.borderTop = '');
    });

    item.addEventListener('dragover', e => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      item.style.borderTop = '2px solid var(--blue)';
    });

    item.addEventListener('dragleave', () => {
      item.style.borderTop = '';
    });

    item.addEventListener('drop', e => {
      e.preventDefault();
      item.style.borderTop = '';
      if (!dragId || dragId === item.dataset.stepId) return;

      const steps = WFS.selected.steps;
      const fromIdx = steps.findIndex(s => s.id === dragId);
      let toIdx = parseInt(item.dataset.stepIdx);
      if (fromIdx < 0 || isNaN(toIdx) || fromIdx === toIdx) return;

      const [moved] = steps.splice(fromIdx, 1);
      if (fromIdx < toIdx) toIdx--;
      steps.splice(toIdx, 0, moved);

      const stepOrders = steps.map((s, i) => ({ id: s.id, step_order: i }));

      api('/workflows/' + WFS.selected.id + '/steps/reorder', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ steps: stepOrders })
      }).then(data => {
        if (data.error) { toast(data.error); return; }
        if (data.status === 'reordered') {
          toast('步骤已重排');
          selectWorkflow(WFS.selected.id);
        } else {
          toast(data.error || '重排失败');
        }
      });
    });
  });
}

/* ---------- Polling ---------- */
function startWfPolling() {
  stopWfPolling();
  WFS.pollTimer = setInterval(() => {
    loadWorkflowList();
    if (WFS.selected) selectWorkflow(WFS.selected.id);
  }, 3000);
}

function stopWfPolling() {
  if (WFS.pollTimer) { clearInterval(WFS.pollTimer); WFS.pollTimer = null; }
}

/* ---------- Step Logs ---------- */
const WFS_LOGS = {}  // step_id -> { open, loading }

function toggleStepLogs(stepId, workflowId) {
  const el = document.getElementById('step-logs-' + stepId)
  if (!el) return

  if (WFS_LOGS[stepId] && WFS_LOGS[stepId].loading) return

  if (WFS_LOGS[stepId] && WFS_LOGS[stepId].open) {
    el.innerHTML = ''
    WFS_LOGS[stepId].open = false
    return
  }

  WFS_LOGS[stepId] = { open: true, loading: true }
  el.innerHTML = '<div class="wf-empty" style="padding:12px;font-size:.75rem">加载日志...</div>'

  api('/workflows/' + workflowId + '/steps/' + stepId + '/logs?limit=500').then(data => {
    WFS_LOGS[stepId].loading = false
    if (data.error) {
      el.innerHTML = '<div style="padding:8px;color:var(--red);font-size:.75rem">' + esc(data.error) + '</div>'
      return
    }
    const logs = data.logs || []
    if (logs.length === 0) {
      el.innerHTML = '<div class="wf-empty" style="padding:12px;font-size:.75rem">暂无日志</div>'
      return
    }
    const lines = logs.map(l => {
      const ts = l.timestamp ? '<span style="color:var(--dim)">' + esc(l.timestamp.substring(11, 19)) + '</span> ' : ''
      const cls = (l.stream === 'stderr' || l.stream === 'STDERR') ? 'style="color:var(--red)"' : ''
      return '<div ' + cls + '>' + ts + esc(l.chunk) + '</div>'
    }).join('')
    el.innerHTML = '<div class="wf-step-logs">' + lines + '</div>' +
      (data.total > logs.length ? '<div style="padding:4px;font-size:.68rem;color:var(--dim)">显示 ' + logs.length + '/' + data.total + '</div>' : '')
  })
}

/* ---------- Helpers ---------- */
function wfStatusBadge(status) {
  const colors = { draft: 'var(--dim)', running: 'var(--blue)', succeeded: 'var(--green)', failed: 'var(--red)', cancelled: 'var(--yellow)' };
  const labels = { draft: '草稿', running: '运行中', succeeded: '完成', failed: '失败', cancelled: '已取消' };
  return '<span class="wf-step-badge" style="background:' + (colors[status] || 'var(--dim)') + ';color:#fff">' + (labels[status] || esc(status)) + '</span>';
}

function stepStatusBadge(status) {
  const colors = { pending: 'var(--dim)', running: 'var(--blue)', succeeded: 'var(--green)', failed: 'var(--red)', cancelled: 'var(--yellow)', skipped: 'var(--dim)' };
  const labels = { pending: '待执行', running: '执行中', succeeded: '完成', failed: '失败', cancelled: '已取消', skipped: '跳过' };
  return '<span class="wf-step-badge" style="background:' + (colors[status] || 'var(--dim)') + ';color:#fff">' + (labels[status] || esc(status)) + '</span>';
}

function stepColor(status) {
  const colors = { pending: 'var(--dim)', running: 'var(--blue)', succeeded: 'var(--green)', failed: 'var(--red)', cancelled: 'var(--yellow)', skipped: 'var(--dim)' };
  return colors[status] || 'var(--dim)';
}

function formatWfBytes(bytes) {
  const n = Number(bytes || 0);
  if (n >= 1024 * 1024) return (n / 1024 / 1024).toFixed(1) + 'MB';
  if (n >= 1024) return (n / 1024).toFixed(1) + 'KB';
  return n + 'B';
}

function renderWfAttempts(attempts) {
  if (!attempts || attempts.length <= 1) return '';
  const chips = attempts.map(a => {
    const label = '#' + a.attempt + ' ' + (a.status || 'unknown') +
      (a.job_id ? ' · ' + String(a.job_id).substring(0, 8) : '') +
      (a.failure_class ? ' · ' + a.failure_class : '');
    return '<span class="wf-step-ctx">' + esc(label) + '</span>';
  }).join('');
  return '<div style="display:flex;gap:4px;flex-wrap:wrap;margin-top:6px">' + chips + '</div>';
}

/* ---------- Tab Entry ---------- */
function loadWorkflows() {
  ensureWorkflowLayout();
  loadWorkflowList().then(() => {
    if (WFS.list.some(wf => wf.status === 'running')) {
      startWfPolling();
    }
  });
}
