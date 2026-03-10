/* ============================================================
   QR Tools Dashboard — Frontend Application
   Production-grade ES2022 vanilla JS (no framework required)
   ============================================================ */

'use strict';

// ── State ────────────────────────────────────────────────────
const State = {
  currentTab: 'encode',
  encodeResult: null,
  decodeFile: null,
  batchResults: [],
};

// ── DOM Helpers ──────────────────────────────────────────────
const $ = (sel, ctx = document) => ctx.querySelector(sel);
const $$ = (sel, ctx = document) => [...ctx.querySelectorAll(sel)];

// ── Toast ─────────────────────────────────────────────────────
function toast(message, type = 'info', duration = 3500) {
  const container = $('#toastContainer');
  const el = document.createElement('div');
  el.className = `toast toast--${type}`;
  const icons = {
    success: '✓',
    error: '✕',
    info: 'ℹ',
  };
  el.textContent = `${icons[type] ?? ''} ${message}`;
  container.appendChild(el);
  setTimeout(() => {
    el.style.animation = 'slideUp .2s ease reverse';
    setTimeout(() => el.remove(), 200);
  }, duration);
}

// ── Loading ───────────────────────────────────────────────────
function setLoading(on) {
  $('#loadingOverlay').style.display = on ? 'flex' : 'none';
}

// ── API ───────────────────────────────────────────────────────
const API_BASE = '/api';

async function apiFetch(path, options = {}) {
  const res = await fetch(`${API_BASE}${path}`, options);
  const json = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg = json?.error?.message ?? `HTTP ${res.status}`;
    throw new Error(msg);
  }
  return json;
}

// ── Health check ──────────────────────────────────────────────
async function checkHealth() {
  const dot = $('#statusDot');
  const label = $('#statusLabel');
  try {
    const data = await apiFetch('/health/ready');
    dot.className = 'status-dot ok';
    label.textContent = 'Online';
    $('#statOps').textContent = data.ops_total ?? 0;
    $('#statHistory').textContent = data.history_entries ?? 0;
  } catch {
    dot.className = 'status-dot error';
    label.textContent = 'Offline';
  }
}

// ── Tab Navigation ────────────────────────────────────────────
function switchTab(name) {
  State.currentTab = name;
  $$('.nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.tab === name);
  });
  $$('.tab-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `tab-${name}`);
  });
  const titles = {
    encode: ['Encode QR Code', 'Convert text or URLs into QR codes'],
    decode: ['Decode QR Code', 'Extract data from QR code images'],
    batch:  ['Batch Encode', 'Generate multiple QR codes at once'],
    history:['Operation History', 'Recent encode / decode operations'],
  };
  const [title, sub] = titles[name] ?? ['QR Tools', ''];
  $('#pageTitle').textContent = title;
  $('#pageSubtitle').textContent = sub;

  if (name === 'history') loadHistory();
}

// ── Encode ────────────────────────────────────────────────────
async function handleEncode() {
  const data      = $('#encodeData').value.trim();
  const format    = $('#encodeFormat').value;
  const ecLevel   = $('#encodeEcLevel').value;
  const moduleSize= parseInt($('#moduleSize').value, 10) || 10;
  const quietZone = parseInt($('#quietZone').value, 10) || 4;
  const fg        = $('#fgColorText').value.trim() || '#000000';
  const bg        = $('#bgColorText').value.trim() || '#ffffff';

  if (!data) {
    toast('Please enter text or a URL to encode.', 'error');
    $('#encodeData').focus();
    return;
  }

  setLoading(true);
  try {
    const result = await apiFetch('/encode', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        data,
        format,
        ec_level: ecLevel,
        module_size: moduleSize,
        quiet_zone: quietZone,
        foreground: fg,
        background: bg,
      }),
    });

    State.encodeResult = result;
    renderEncodeResult(result);
    toast('QR code generated!', 'success');
    await checkHealth();
  } catch (err) {
    toast(err.message, 'error');
  } finally {
    setLoading(false);
  }
}

function renderEncodeResult(result) {
  const preview = $('#encodePreview');
  const actions = $('#encodeActions');
  const meta    = $('#encodeMeta');

  if (result.mime_type === 'image/svg+xml') {
    preview.innerHTML = result.data;
    const svg = preview.querySelector('svg');
    if (svg) { svg.style.maxWidth = '100%'; svg.style.maxHeight = '400px'; }
  } else {
    const img = document.createElement('img');
    img.src = `data:${result.mime_type};base64,${result.data}`;
    img.alt = 'Generated QR code';
    img.style.maxWidth = '100%';
    img.style.maxHeight = '400px';
    preview.innerHTML = '';
    preview.appendChild(img);
  }

  actions.style.display = 'flex';

  const sizeStr = result.width && result.height
    ? `${result.width} × ${result.height} px`
    : '—';
  meta.innerHTML = `
    <span><b>ID:</b> ${result.id.slice(0,8)}…</span>
    <span><b>Format:</b> ${result.mime_type}</span>
    <span><b>Size:</b> ${sizeStr}</span>
    <span><b>At:</b> ${new Date(result.timestamp).toLocaleTimeString()}</span>
  `;
}

function handleDownload() {
  const result = State.encodeResult;
  if (!result) return;

  const ext  = result.mime_type === 'image/svg+xml' ? 'svg' : 'png';
  const text = result.mime_type === 'image/svg+xml'
    ? result.data
    : null;
  const link = document.createElement('a');
  link.download = `qrcode-${result.id.slice(0,8)}.${ext}`;

  if (ext === 'svg') {
    const blob = new Blob([text], { type: 'image/svg+xml' });
    link.href = URL.createObjectURL(blob);
  } else {
    link.href = `data:image/png;base64,${result.data}`;
  }
  link.click();
  URL.revokeObjectURL(link.href);
  toast('Download started!', 'success');
}

function handleCopyBase64() {
  const result = State.encodeResult;
  if (!result) return;
  navigator.clipboard.writeText(result.data)
    .then(() => toast('Copied to clipboard!', 'success'))
    .catch(() => toast('Clipboard not available.', 'error'));
}

// ── Decode ────────────────────────────────────────────────────
async function handleDecode() {
  const base64Input = $('#decodeBase64').value.trim();
  const fileData    = State.decodeFile;

  if (!fileData && !base64Input) {
    toast('Please upload an image or paste base64 data.', 'error');
    return;
  }

  setLoading(true);
  try {
    let result;

    if (fileData) {
      // Multipart upload
      const form = new FormData();
      form.append('file', fileData);
      const res = await fetch(`${API_BASE}/decode/upload`, { method: 'POST', body: form });
      const json = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(json?.error?.message ?? `HTTP ${res.status}`);
      result = json;
    } else {
      // Strip data URI prefix if present
      const raw = base64Input.replace(/^data:[^;]+;base64,/, '');
      result = await apiFetch('/decode/json', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ image_b64: raw }),
      });
    }

    renderDecodeResult(result);
    toast('QR code decoded!', 'success');
    await checkHealth();
  } catch (err) {
    toast(err.message, 'error');
    $('#decodeResult').innerHTML = `<div class="qr-placeholder" style="color:var(--error)">
      <p>${escapeHtml(err.message)}</p></div>`;
  } finally {
    setLoading(false);
  }
}

function renderDecodeResult(result) {
  const container = $('#decodeResult');
  const isUrl = /^https?:\/\//i.test(result.text);
  const textEl = isUrl
    ? `<a href="${escapeHtml(result.text)}" target="_blank" rel="noopener noreferrer"
         style="color:var(--accent)">${escapeHtml(result.text)}</a>`
    : escapeHtml(result.text);

  container.innerHTML = `
    <div class="decode-success" style="width:100%">
      <div class="decode-text-box">${textEl}</div>
      <div class="decode-meta">
        <span>Format: ${escapeHtml(result.format)}</span>
        <span>ID: ${escapeHtml(result.id.slice(0,8))}…</span>
        <span>At: ${new Date(result.timestamp).toLocaleTimeString()}</span>
      </div>
      <div class="decode-actions">
        <button class="btn btn--secondary btn--sm" onclick="copyText(${JSON.stringify(result.text)})">
          Copy text
        </button>
        ${isUrl ? `<a class="btn btn--secondary btn--sm" href="${escapeHtml(result.text)}"
            target="_blank" rel="noopener noreferrer">Open URL</a>` : ''}
      </div>
    </div>
  `;
}

window.copyText = function(text) {
  navigator.clipboard.writeText(text)
    .then(() => toast('Copied!', 'success'))
    .catch(() => toast('Clipboard not available.', 'error'));
};

// ── Drop Zone ─────────────────────────────────────────────────
function initDropZone() {
  const zone     = $('#dropZone');
  const input    = $('#fileInput');
  const preview  = $('#dropPreview');
  const content  = $('#dropContent');

  zone.addEventListener('click', () => input.click());
  zone.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') input.click(); });

  zone.addEventListener('dragover', e => { e.preventDefault(); zone.classList.add('drag-over'); });
  zone.addEventListener('dragleave', () => zone.classList.remove('drag-over'));
  zone.addEventListener('drop', e => {
    e.preventDefault();
    zone.classList.remove('drag-over');
    const file = e.dataTransfer?.files?.[0];
    if (file) setDecodeFile(file);
  });

  input.addEventListener('change', () => {
    const file = input.files?.[0];
    if (file) setDecodeFile(file);
  });

  function setDecodeFile(file) {
    if (!file.type.startsWith('image/')) {
      toast('Please select an image file.', 'error');
      return;
    }
    State.decodeFile = file;
    const reader = new FileReader();
    reader.onload = e => {
      preview.src = e.target.result;
      preview.style.display = 'block';
      content.style.display = 'none';
    };
    reader.readAsDataURL(file);
    toast(`File loaded: ${file.name}`, 'info');
  }
}

// ── Batch ─────────────────────────────────────────────────────
async function handleBatch() {
  const lines = $('#batchData').value
    .split('\n')
    .map(l => l.trim())
    .filter(l => l.length > 0);

  if (lines.length === 0) {
    toast('Please enter at least one item.', 'error');
    return;
  }
  if (lines.length > 50) {
    toast('Maximum 50 items per batch.', 'error');
    return;
  }

  const format   = $('#batchFormat').value;
  const ecLevel  = $('#batchEcLevel').value;
  const container = $('#batchResults');
  container.innerHTML = '';

  setLoading(true);
  const results = [];

  for (const line of lines) {
    try {
      const result = await apiFetch('/encode', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ data: line, format, ec_level: ecLevel }),
      });
      results.push({ line, result, error: null });
    } catch (err) {
      results.push({ line, result: null, error: err.message });
    }
  }

  State.batchResults = results;
  setLoading(false);

  for (const { line, result, error } of results) {
    const item = document.createElement('div');
    item.className = 'batch-item';

    if (error) {
      item.innerHTML = `
        <div style="color:var(--error);font-size:.78rem;">Error: ${escapeHtml(error)}</div>
        <div class="batch-item-label">${escapeHtml(truncate(line, 30))}</div>
      `;
    } else {
      const src = result.mime_type === 'image/svg+xml'
        ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(result.data)}`
        : `data:image/png;base64,${result.data}`;
      item.innerHTML = `
        <img src="${src}" alt="QR ${escapeHtml(line)}" loading="lazy" />
        <div class="batch-item-label" title="${escapeHtml(line)}">${escapeHtml(truncate(line, 28))}</div>
        <div class="batch-item-actions">
          <a class="btn btn--secondary btn--sm" href="${src}" download="qr-${result.id.slice(0,6)}.${format}">↓</a>
        </div>
      `;
    }
    container.appendChild(item);
  }

  const ok = results.filter(r => !r.error).length;
  toast(`Batch complete: ${ok}/${lines.length} generated.`, ok === lines.length ? 'success' : 'info');
  await checkHealth();
}

// ── History ───────────────────────────────────────────────────
async function loadHistory() {
  const list = $('#historyList');
  list.innerHTML = '<div style="color:var(--text-muted);font-size:.85rem;padding:1rem;">Loading…</div>';
  try {
    const data = await apiFetch('/history');
    renderHistory(data.entries ?? []);
    $('#statHistory').textContent = data.total ?? 0;
  } catch {
    list.innerHTML = '<div style="color:var(--error);font-size:.85rem;padding:1rem;">Failed to load history.</div>';
  }
}

function renderHistory(entries) {
  const list = $('#historyList');
  if (!entries.length) {
    list.innerHTML = `<div class="qr-placeholder">
      <svg class="placeholder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
      <p>No operations yet</p>
    </div>`;
    return;
  }

  list.innerHTML = entries.map(entry => {
    const thumb = entry.preview_png_b64
      ? `<img src="data:image/png;base64,${entry.preview_png_b64}" alt="" />`
      : `<div class="history-thumb-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="24" height="24"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg></div>`;

    const kindClass = entry.kind === 'encode' ? 'history-kind--encode' : 'history-kind--decode';
    const kindLabel = entry.kind ?? '';
    const time = entry.timestamp ? new Date(entry.timestamp).toLocaleString() : '';

    return `
      <div class="history-item">
        <div class="history-thumb">${thumb}</div>
        <div class="history-body">
          <div class="history-summary">${escapeHtml(entry.summary ?? '')}</div>
          <div class="history-detail">${escapeHtml(time)}</div>
        </div>
        <span class="history-kind ${kindClass}">${escapeHtml(kindLabel)}</span>
      </div>
    `;
  }).join('');
}

async function clearHistory() {
  try {
    await apiFetch('/history', { method: 'DELETE' });
    toast('History cleared.', 'success');
    renderHistory([]);
    $('#statHistory').textContent = 0;
  } catch (err) {
    toast(err.message, 'error');
  }
}

// ── Helpers ───────────────────────────────────────────────────
function escapeHtml(str = '') {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function truncate(str, max) {
  return str.length > max ? str.slice(0, max) + '…' : str;
}

// ── Theme ─────────────────────────────────────────────────────
function initTheme() {
  const saved = localStorage.getItem('qrtools-theme') ?? 'dark';
  document.documentElement.dataset.theme = saved;
  $('#themeToggle').addEventListener('click', () => {
    const current = document.documentElement.dataset.theme;
    const next = current === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    localStorage.setItem('qrtools-theme', next);
  });
}

// ── Color sync ────────────────────────────────────────────────
function initColorSync(pickerId, textId) {
  const picker = $(`#${pickerId}`);
  const text   = $(`#${textId}`);
  picker.addEventListener('input', () => { text.value = picker.value; });
  text.addEventListener('input', () => {
    const v = text.value.trim();
    if (/^#[0-9a-fA-F]{6}$/.test(v)) picker.value = v;
  });
}

// ── Char counter ──────────────────────────────────────────────
function initCharCounter() {
  const ta = $('#encodeData');
  const counter = $('#charCount');
  ta.addEventListener('input', () => { counter.textContent = ta.value.length; });
}

// ── Boot ──────────────────────────────────────────────────────
function init() {
  initTheme();
  initDropZone();
  initCharCounter();
  initColorSync('fgColorPicker', 'fgColorText');
  initColorSync('bgColorPicker', 'bgColorText');

  // Tab nav
  $$('.nav-item').forEach(btn => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab));
  });

  // Encode
  $('#encodeBtn').addEventListener('click', handleEncode);
  $('#downloadBtn').addEventListener('click', handleDownload);
  $('#copyBtn').addEventListener('click', handleCopyBase64);

  // Encode on Ctrl+Enter
  $('#encodeData').addEventListener('keydown', e => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) handleEncode();
  });

  // Decode
  $('#decodeBtn').addEventListener('click', handleDecode);

  // Batch
  $('#batchBtn').addEventListener('click', handleBatch);

  // History
  $('#clearHistoryBtn').addEventListener('click', clearHistory);

  // Health poll
  checkHealth();
  setInterval(checkHealth, 30_000);
}

document.addEventListener('DOMContentLoaded', init);
