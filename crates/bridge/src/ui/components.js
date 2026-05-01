// ===== Bridge-Specific Components =====
// renderMd, copyCodeBlock, copyIcon, copyButton, copyText — 来自 shared_ui/render.js

function renderSkeleton(count,into){
  const container=typeof into==='string'?document.getElementById(into):into;
  if(!container)return;
  let html='';
  for(let i=0;i<count;i++)html+=SKELETON_HTML;
  container.innerHTML=html;
}

function emptyState(title,subtitle){
  return '<div class="empty-state"><div class="empty-state-title">'+esc(title)+'</div><div class="empty-state-sub">'+esc(subtitle)+'</div></div>';
}

function badge(type,text){
  return '<span class="badge badge-'+esc(type)+'">'+esc(text)+'</span>';
}

// Section label
function sectionLabel(text){
  return '<div class="section-label">'+esc(text)+'</div>';
}

// Card wrapper
function card(html,opts){
  opts=opts||{};
  let cls='card';
  if(opts.clickable)cls+=' card-clickable';
  if(opts.selected)cls+=' selected';
  return '<div class="'+cls+'"'+(opts.onclick?' onclick="'+opts.onclick+'"':'')+(opts.id?' id="'+opts.id+'"':'')+'>'+html+'</div>';
}

// Compact event meta line
function eventMeta(parts){
  return '<div class="card-meta">'+parts.filter(Boolean).map(p=>esc(p)).join(' <span style="color:var(--dimmer)">·</span> ')+'</div>';
}
