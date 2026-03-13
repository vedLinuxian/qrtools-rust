/* ===================================================================
   QRTools app.js  v2.0
   Depends on: jsQR (loaded from CDN inline fallback below)
   =================================================================== */
'use strict';

/* ── jsQR CDN fallback ─────────────────────────────────────────── */
(function(){
  const s = document.createElement('script');
  s.src = 'https://cdn.jsdelivr.net/npm/jsqr@1.4.0/dist/jsQR.min.js';
  document.head.appendChild(s);
})();

/* ── Utilities ─────────────────────────────────────────────────── */
const $ = id => document.getElementById(id);
const toast = (msg, type='info', ms=3500) => {
  const c = $('toasts');
  const el = document.createElement('div');
  el.className = `toast toast--${type}`;
  el.textContent = msg;
  c.appendChild(el);
  setTimeout(() => {
    el.classList.add('fade-out');
    setTimeout(() => el.remove(), 260);
  }, ms);
};
const loading = show => { $('overlay').style.display = show ? 'flex' : 'none'; };
const api = async (path, opts={}) => {
  const r = await fetch(path, opts);
  if (!r.ok) {
    let err = `HTTP ${r.status}`;
    try { const j = await r.json(); err = j.error || j.message || err; } catch{}
    throw new Error(err);
  }
  return r;
};
const apiJSON = async (path, opts={}) => (await api(path, opts)).json();
const fmtTs = ts => {
  if (!ts) return '';
  const d = new Date(ts);
  return isNaN(d) ? ts : d.toLocaleString();
};
const downloadBlob = (blob, filename) => {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url; a.download = filename; a.click();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
};
const encodeFileName = (fmt) => `qr_${Date.now()}.${fmt}`;

/* ── Theme ─────────────────────────────────────────────────────── */
const THEME_KEY = 'qrt_theme';
(function initTheme() {
  const saved = localStorage.getItem(THEME_KEY) || 'dark';
  document.documentElement.setAttribute('data-theme', saved);
})();
$('themeBtn').addEventListener('click', () => {
  const cur = document.documentElement.getAttribute('data-theme');
  const nxt = cur === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', nxt);
  localStorage.setItem(THEME_KEY, nxt);
});

/* ── Sidebar / tab navigation ──────────────────────────────────── */
const TITLES = { encode:'Encode', decode:'Decode', batch:'Batch Encode', payload:'Payload Builder', history:'History' };
document.querySelectorAll('.nav-item').forEach(btn => {
  btn.addEventListener('click', () => {
    const tab = btn.dataset.tab;
    document.querySelectorAll('.nav-item').forEach(b => b.classList.toggle('active', b === btn));
    document.querySelectorAll('.tab-panel').forEach(p => p.classList.toggle('active', p.id === `tab-${tab}`));
    $('pageTitle').textContent = TITLES[tab] || tab;
    if (tab === 'history') loadHistory();
  });
});
$('sidebarToggle').addEventListener('click', () => document.querySelector('.sidebar').classList.toggle('collapsed'));

/* ── Keyboard shortcuts ────────────────────────────────────────── */
document.addEventListener('keydown', e => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      const active = document.querySelector('.tab-panel.active')?.id;
      if (active === 'tab-encode') $('encodeBtn').click();
      else if (active === 'tab-decode') $('decodeBtn').click();
      else if (active === 'tab-batch') $('batchEncodeBtn').click();
    }
    return;
  }
  switch(e.key) {
    case 'e': case 'E': document.querySelector('[data-tab="encode"]').click(); break;
    case 'd': case 'D': document.querySelector('[data-tab="decode"]').click(); break;
    case 'b': case 'B': document.querySelector('[data-tab="batch"]').click(); break;
    case 'p': case 'P': document.querySelector('[data-tab="payload"]').click(); break;
    case 'h': case 'H': document.querySelector('[data-tab="history"]').click(); break;
    case 't': case 'T': $('themeBtn').click(); break;
  }
});

/* ── WebSocket live stats ──────────────────────────────────────── */
let wsRetry = 0;
function connectWS() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const ws = new WebSocket(`${proto}://${location.host}/ws/stats`);
  ws.onopen = () => {
    wsRetry = 0;
    $('wsDot').className = 'ws-dot live';
    $('wsTxt').textContent = 'WS live';
  };
  ws.onmessage = e => {
    try {
      const d = JSON.parse(e.data);
      if (typeof d.ops_total === 'number')  $('chipOps').textContent  = `${d.ops_total} ops`;
      if (typeof d.hist_count === 'number') $('chipHist').textContent = `${d.hist_count} entries`;
    } catch {}
  };
  ws.onclose = () => {
    $('wsDot').className = 'ws-dot';
    $('wsTxt').textContent = 'WS off';
    const delay = Math.min(30000, 1000 * 2**wsRetry++);
    setTimeout(connectWS, delay);
  };
  ws.onerror = () => ws.close();
}
connectWS();

/* ── Health check ──────────────────────────────────────────────── */
async function checkHealth() {
  try {
    const j = await apiJSON('/api/health');
    $('statusDot').className = 'status-dot ok';
    $('statusTxt').textContent = j.status || 'ok';
  } catch {
    $('statusDot').className = 'status-dot err';
    $('statusTxt').textContent = 'error';
  }
}
checkHealth();
setInterval(checkHealth, 60_000);

/* ═══════════════════════════════════════════════════════════════
   ENCODE
   ═══════════════════════════════════════════════════════════════ */
const encData = $('encodeData');
encData.addEventListener('input', () => {
  $('encodeCounter').textContent = `${encData.value.length} / 7089`;
});

/* Color pickers sync */
[['encodeFgPicker','encodeFgText'], ['encodeBgPicker','encodeBgText'], ['encodeGradPicker','encodeGradText']].forEach(([pid, tid]) => {
  const picker = $(pid), text = $(tid);
  picker.addEventListener('input', () => { text.value = picker.value; });
  text.addEventListener('change', () => {
    if (/^#[0-9a-fA-F]{6}$/.test(text.value)) picker.value = text.value;
  });
});

/* JPEG quality slider */
$('encodeFormat').addEventListener('change', () => {
  $('jpegQualityRow').style.display = $('encodeFormat').value === 'jpeg' ? 'block' : 'none';
});
$('encodeJpegQ').addEventListener('input', () => {
  $('encodeJpegQVal').textContent = $('encodeJpegQ').value;
});

/* Logo ratio slider */
$('encodeLogoRatio').addEventListener('input', () => {
  $('encodeLogoRatioVal').textContent = $('encodeLogoRatio').value;
});

/* Logo file pick */
$('encodeLogo').addEventListener('change', function() {
  $('encodeLogoName').textContent = this.files[0]?.name || 'No file chosen';
});
$('encodeClearLogo').addEventListener('click', () => {
  $('encodeLogo').value = '';
  $('encodeLogoName').textContent = 'No file chosen';
});

/* Build encode request body */
async function buildEncodeBody() {
  const body = {
    data:           encData.value.trim(),
    format:         $('encodeFormat').value,
    ec_level:       $('encodeEC').value,
    module_size:    parseInt($('encodeModSize').value) || 10,
    quiet_zone:     parseInt($('encodeQZ').value) || 4,
    foreground:     $('encodeFgText').value || '#000000',
    background:     $('encodeBgText').value || '#ffffff',
    module_style:   $('encodeStyle').value,
  };
  const grad = $('encodeGradText').value.trim();
  if (grad) body.gradient_color = grad;
  const badge = $('encodeBadge').value.trim();
  if (badge) body.badge_label = badge;
  if ($('encodeFormat').value === 'jpeg') body.jpeg_quality = parseInt($('encodeJpegQ').value);
  const logoFile = $('encodeLogo').files[0];
  if (logoFile) {
    const b64 = await readFileAsBase64(logoFile);
    body.logo_base64 = b64.split(',')[1] || b64;
    body.logo_ratio = parseInt($('encodeLogoRatio').value) / 100;
  }
  return body;
}

function readFileAsBase64(file) {
  return new Promise((res,rej) => {
    const fr = new FileReader();
    fr.onload = () => res(fr.result);
    fr.onerror = rej;
    fr.readAsDataURL(file);
  });
}

$('encodeBtn').addEventListener('click', async () => {
  const data = encData.value.trim();
  if (!data) { toast('Enter some data first', 'error'); return; }
  loading(true);
  try {
    const body = await buildEncodeBody();
    const json = await apiJSON('/api/encode', {
      method: 'POST',
      headers: {'Content-Type':'application/json'},
      body: JSON.stringify(body)
    });
    renderEncodeResult(json, body.format);
  } catch(e) { toast(e.message, 'error'); }
  finally { loading(false); }
});

let lastEncodeResult = null;
function renderEncodeResult(json, fmt) {
  lastEncodeResult = json;
  const card = $('encodeResultCard');
  const preview = $('encodePreview');
  card.style.display = '';
  if (fmt === 'svg') {
    preview.innerHTML = atob(json.data);  // SVG string
  } else {
    const img = new Image();
    img.src = `data:image/${fmt === 'jpeg' ? 'jpeg' : fmt === 'webp' ? 'webp' : 'png'};base64,${json.data}`;
    img.alt = 'QR Code';
    preview.innerHTML = '';
    preview.appendChild(img);
  }
  $('encodeMeta').innerHTML = `Format: <strong>${fmt.toUpperCase()}</strong> &nbsp;|&nbsp; Size: <strong>${json.width}×${json.height} px</strong>`;
  $('encodeCopyURL').style.display = isURL(encData.value.trim()) ? '' : 'none';
}

$('encodeDownload').addEventListener('click', () => {
  if (!lastEncodeResult) return;
  const fmt = $('encodeFormat').value;
  if (fmt === 'svg') {
    const blob = new Blob([atob(lastEncodeResult.data)], {type:'image/svg+xml'});
    downloadBlob(blob, encodeFileName('svg'));
  } else {
    const mime = fmt === 'jpeg' ? 'image/jpeg' : fmt === 'webp' ? 'image/webp' : 'image/png';
    const byteStr = atob(lastEncodeResult.data);
    const buf = new Uint8Array(byteStr.length);
    for (let i=0;i<byteStr.length;i++) buf[i] = byteStr.charCodeAt(i);
    downloadBlob(new Blob([buf], {type: mime}), encodeFileName(fmt));
  }
});
$('encodeCopyB64').addEventListener('click', () => {
  if (!lastEncodeResult) return;
  navigator.clipboard.writeText(lastEncodeResult.data).then(() => toast('Copied!', 'success'));
});
$('encodeCopyURL').addEventListener('click', () => {
  const u = encData.value.trim();
  if (isURL(u)) window.open(u,'_blank');
});

/* ═══════════════════════════════════════════════════════════════
   DECODE
   ═══════════════════════════════════════════════════════════════ */
const dropZone = $('dropZone');

dropZone.addEventListener('dragover', e => { e.preventDefault(); dropZone.classList.add('drag-over'); });
dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
dropZone.addEventListener('drop', e => {
  e.preventDefault(); dropZone.classList.remove('drag-over');
  const file = e.dataTransfer.files[0];
  if (file) handleDecodeFile(file);
});
dropZone.addEventListener('click', () => $('decodeFile').click());
dropZone.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') $('decodeFile').click(); });
$('decodeFile').addEventListener('change', function() { if(this.files[0]) handleDecodeFile(this.files[0]); });

async function handleDecodeFile(file) {
  const b64 = await readFileAsBase64(file);
  $('decodeBase64').value = b64;
  $('decodeURL').value = '';
}

$('decodeBtn').addEventListener('click', async () => {
  loading(true);
  try {
    let json;
    const url = $('decodeURL').value.trim();
    const b64 = $('decodeBase64').value.trim();
    if (url) {
      json = await apiJSON('/api/decode/url', {
        method: 'POST',
        headers: {'Content-Type':'application/json'},
        body: JSON.stringify({url})
      });
    } else if (b64) {
      const raw = b64.includes(',') ? b64.split(',')[1] : b64;
      json = await apiJSON('/api/decode/json', {
        method: 'POST',
        headers: {'Content-Type':'application/json'},
        body: JSON.stringify({image_b64: raw})
      });
    } else {
      toast('Provide an image, base64, or URL', 'error');
      return;
    }
    showDecodeResult(json);
  } catch(e) { toast(e.message, 'error'); }
  finally { loading(false); }
});

function showDecodeResult(json) {
  const card = $('decodeResultCard');
  card.style.display = '';
  $('decodeText').textContent = json.data || json.text || JSON.stringify(json);
  $('decodeFormat').textContent = json.format ? `Format: ${json.format}` : '';
  const isUrl = isURL(json.data || json.text || '');
  $('decodeOpenURL').style.display = isUrl ? '' : 'none';
}

$('decodeCopy').addEventListener('click', () => {
  navigator.clipboard.writeText($('decodeText').textContent).then(() => toast('Copied!', 'success'));
});
$('decodeOpenURL').addEventListener('click', () => {
  window.open($('decodeText').textContent.trim(), '_blank');
});
$('decodeEncodeBack').addEventListener('click', () => {
  encData.value = $('decodeText').textContent.trim();
  document.querySelector('[data-tab="encode"]').click();
});

/* Camera decode */
let cameraStream = null;
let cameraTimer = null;
$('cameraBtn').addEventListener('click', async () => {
  if (cameraStream) return;
  try {
    cameraStream = await navigator.mediaDevices.getUserMedia({video:{facingMode:'environment'}});
    const video = $('cameraVideo');
    video.srcObject = cameraStream;
    video.style.display = '';
    $('cameraStopBtn').style.display = '';
    $('cameraBtn').textContent = '📷 Camera active…';
    cameraTimer = setInterval(() => scanFrame(), 300);
  } catch(e) { toast('Camera not available: ' + e.message, 'error'); }
});
$('cameraStopBtn').addEventListener('click', stopCamera);
function stopCamera() {
  if (cameraStream) { cameraStream.getTracks().forEach(t => t.stop()); cameraStream = null; }
  clearInterval(cameraTimer); cameraTimer = null;
  $('cameraVideo').style.display = 'none';
  $('cameraStopBtn').style.display = 'none';
  $('cameraBtn').textContent = '📷 Scan with Camera';
}
function scanFrame() {
  const video = $('cameraVideo');
  const canvas = $('cameraCanvas');
  if (video.readyState < 2) return;
  canvas.width  = video.videoWidth;
  canvas.height = video.videoHeight;
  const ctx = canvas.getContext('2d');
  ctx.drawImage(video, 0, 0);
  if (typeof jsQR === 'undefined') return;
  const img = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const code = jsQR(img.data, img.width, img.height);
  if (code) {
    stopCamera();
    showDecodeResult({data: code.data, format: 'QR_CODE (camera)'});
    toast('QR found!', 'success');
  }
}

/* ═══════════════════════════════════════════════════════════════
   BATCH
   ═══════════════════════════════════════════════════════════════ */
$('batchEncodeBtn').addEventListener('click', () => runBatch(false));
$('batchExportBtn').addEventListener('click', () => runBatch(true));

async function runBatch(zip) {
  const lines = $('batchData').value.split('\n').map(l => l.trim()).filter(Boolean);
  if (!lines.length) { toast('Enter at least one item', 'error'); return; }
  if (lines.length > 200) { toast('Max 200 items', 'error'); return; }

  const defaults = {
    format:        $('batchFormat').value,
    ec_level:      $('batchEC').value,
    module_style:  $('batchStyle').value,
  };
  const items = lines.map(l => {
    const parts = l.split(',');
    const item = {data: parts[0].trim()};
    if (parts[1]) item.badge_label = parts[1].trim();
    return item;
  });
  const payload = {defaults, items};

  loading(true);
  const bar = $('batchProgress');
  const fill = $('batchProgressFill');
  bar.style.display = '';
  fill.style.width = '30%';

  try {
    if (zip) {
      const resp = await api('/api/batch/export', {
        method: 'POST',
        headers: {'Content-Type':'application/json'},
        body: JSON.stringify(payload)
      });
      fill.style.width = '90%';
      const blob = await resp.blob();
      downloadBlob(blob, `qrcodes_${Date.now()}.zip`);
      fill.style.width = '100%';
      toast(`ZIP with ${items.length} QR codes downloaded`, 'success');
    } else {
      const json = await apiJSON('/api/batch/encode', {
        method: 'POST',
        headers: {'Content-Type':'application/json'},
        body: JSON.stringify(payload)
      });
      fill.style.width = '100%';
      renderBatchResult(json.results, defaults.format);
      toast(`${json.results.filter(r=>!r.error).length}/${json.results.length} generated`, 'success');
    }
  } catch(e) { toast(e.message, 'error'); }
  finally { loading(false); setTimeout(() => { bar.style.display='none'; fill.style.width='0'; }, 1500); }
}

function renderBatchResult(results, fmt) {
  const container = $('batchResults');
  container.innerHTML = '';
  results.forEach((r, i) => {
    const div = document.createElement('div');
    div.className = `batch-item${r.error ? ' error' : ''}`;
    if (r.error) {
      div.innerHTML = `<span class="batch-item__err">${r.error}</span><span class="batch-item__label" title="${r.data||''}">${(r.data||'item '+i).substring(0,20)}</span>`;
    } else {
      const mime = fmt === 'jpeg' ? 'image/jpeg' : fmt === 'webp' ? 'image/webp' : fmt === 'svg' ? 'image/svg+xml' : 'image/png';
      const src = `data:${mime};base64,${r.image}`;
      div.innerHTML = `<img src="${src}" alt="QR"/><span class="batch-item__label" title="${r.data||''}">${(r.data||'').substring(0,20)}</span><a class="batch-item__dl">⬇ Save</a>`;
      div.querySelector('a').addEventListener('click', () => {
        if (fmt === 'svg') {
          const blob = new Blob([atob(r.image_b64)], {type:'image/svg+xml'});
          downloadBlob(blob, `qr_${i}.svg`);
        } else {
          const byteStr = atob(r.image);
          const buf = new Uint8Array(byteStr.length);
          for (let k=0;k<byteStr.length;k++) buf[k]=byteStr.charCodeAt(k);
          downloadBlob(new Blob([buf],{type:mime}), `qr_${i}.${fmt}`);
        }
      });
    }
    container.appendChild(div);
  });
}

/* ═══════════════════════════════════════════════════════════════
   PAYLOAD BUILDER
   ═══════════════════════════════════════════════════════════════ */
const PAYLOAD_FORMS = {
  wifi: `
    <div class="field"><label>SSID (Network name)</label><input id="pf_ssid" placeholder="MyWifi"/></div>
    <div class="field"><label>Password</label><input id="pf_pass" type="password" placeholder="password"/></div>
    <div class="row-2">
      <div class="field"><label>Security</label>
        <select id="pf_sec"><option value="WPA">WPA/WPA2</option><option value="WEP">WEP</option><option value="nopass">None</option></select>
      </div>
      <div class="field"><label>Hidden network?</label>
        <select id="pf_hidden"><option value="false">No</option><option value="true">Yes</option></select>
      </div>
    </div>`,
  vcard: `
    <div class="row-2">
      <div class="field"><label>First name</label><input id="pf_vfirst" placeholder="Jane"/></div>
      <div class="field"><label>Last name</label><input id="pf_vlast" placeholder="Doe"/></div>
    </div>
    <div class="field"><label>Organisation</label><input id="pf_vorg"/></div>
    <div class="field"><label>Phone</label><input id="pf_vphone" placeholder="+1 555 0100"/></div>
    <div class="field"><label>Email</label><input id="pf_vemail" type="email"/></div>
    <div class="field"><label>Website</label><input id="pf_vurl" type="url"/></div>
    <div class="field"><label>Address</label><input id="pf_vaddr" placeholder="123 Main St, City, Country"/></div>
    <div class="field"><label>Note / Bio</label><textarea id="pf_vnote" rows="2"></textarea></div>`,
  sms: `
    <div class="field"><label>Phone number</label><input id="pf_sph" placeholder="+15550100"/></div>
    <div class="field"><label>Message (optional)</label><textarea id="pf_sbody" rows="3"></textarea></div>`,
  email: `
    <div class="field"><label>To</label><input id="pf_eto" type="email" placeholder="foo@example.com"/></div>
    <div class="field"><label>CC (optional)</label><input id="pf_ecc"/></div>
    <div class="field"><label>Subject (optional)</label><input id="pf_esubj"/></div>
    <div class="field"><label>Body (optional)</label><textarea id="pf_ebody" rows="3"></textarea></div>`,
  geo: `
    <div class="row-2">
      <div class="field"><label>Latitude</label><input id="pf_glat" placeholder="48.8566" type="number" step="any"/></div>
      <div class="field"><label>Longitude</label><input id="pf_glon" placeholder="2.3522" type="number" step="any"/></div>
    </div>
    <div class="field"><label>Altitude (optional)</label><input id="pf_galt" type="number" step="any"/></div>`,
  phone: `<div class="field"><label>Phone number</label><input id="pf_ph" placeholder="+15550100"/></div>`,
  url: `<div class="field"><label>URL</label><input id="pf_url" type="url" placeholder="https://"/></div>`,
  text: `<div class="field"><label>Text</label><textarea id="pf_text" rows="4"></textarea></div>`,
  cal_event: `
    <div class="field"><label>Summary / Title</label><input id="pf_csumm" placeholder="Meeting"/></div>
    <div class="row-2">
      <div class="field"><label>Start (local datetime)</label><input id="pf_cstart" type="datetime-local"/></div>
      <div class="field"><label>End   (local datetime)</label><input id="pf_cend"   type="datetime-local"/></div>
    </div>
    <div class="field"><label>Location (optional)</label><input id="pf_cloc"/></div>
    <div class="field"><label>Description (optional)</label><textarea id="pf_cdesc" rows="2"></textarea></div>`,
  bitcoin: `
    <div class="field"><label>Bitcoin address</label><input id="pf_btaddr" placeholder="bc1q…"/></div>
    <div class="row-2">
      <div class="field"><label>Amount (BTC)</label><input id="pf_btamt" type="number" step="any" placeholder="0.001"/></div>
      <div class="field"><label>Label (optional)</label><input id="pf_btlabel"/></div>
    </div>
    <div class="field"><label>Message (optional)</label><input id="pf_btmsg"/></div>`,
};

const payloadTypeEl = $('payloadType');
payloadTypeEl.addEventListener('change', renderPayloadForm);

function renderPayloadForm() {
  const t = payloadTypeEl.value;
  $('payloadForm').innerHTML = PAYLOAD_FORMS[t] || '';
  $('payloadPreviewBox').style.display = 'none';
}
renderPayloadForm();

function gatherPayloadRequest() {
  const t = payloadTypeEl.value;
  const g = id => $(id)?.value?.trim() || '';
  const payloads = {
    wifi:      { type:'wifi',      ssid:g('pf_ssid'), password:g('pf_pass'), auth:g('pf_sec'), hidden: g('pf_hidden')==='true' },
    vcard:     { type:'vcard',     first_name:g('pf_vfirst'), last_name:g('pf_vlast'), org:g('pf_vorg'), phone:g('pf_vphone'), email:g('pf_vemail'), url:g('pf_vurl'), address:g('pf_vaddr'), note:g('pf_vnote') },
    sms:       { type:'sms',       phone:g('pf_sph'), message:g('pf_sbody') },
    email:     { type:'email',     to:g('pf_eto'), cc:g('pf_ecc'), subject:g('pf_esubj'), body:g('pf_ebody') },
    geo:       { type:'geo',       latitude:parseFloat(g('pf_glat'))||0, longitude:parseFloat(g('pf_glon'))||0, altitude:g('pf_galt')?parseFloat(g('pf_galt')):undefined },
    phone:     { type:'phone',     phone:g('pf_ph') },
    url:       { type:'url',       url:g('pf_url') },
    text:      { type:'text',      text:g('pf_text') },
    cal_event: { type:'cal_event', summary:g('pf_csumm'), dtstart:g('pf_cstart'), dtend:g('pf_cend'), location:g('pf_cloc'), description:g('pf_cdesc') },
    bitcoin:   { type:'bitcoin',   address:g('pf_btaddr'), amount:g('pf_btamt')?parseFloat(g('pf_btamt')):undefined, label:g('pf_btlabel'), message:g('pf_btmsg') },
  };
  return payloads[t];
}

$('payloadPreviewBtn').addEventListener('click', async () => {
  const req = gatherPayloadRequest();
  if (!req) { toast('Fill in the form', 'error'); return; }
  loading(true);
  try {
    const json = await apiJSON('/api/payload/build', {
      method:'POST', headers:{'Content-Type':'application/json'},
      body: JSON.stringify(req)
    });
    $('payloadPreviewText').value = json.payload;
    $('payloadPreviewBox').style.display = '';
  } catch(e) { toast(e.message,'error'); }
  finally { loading(false); }
});

let lastPayloadResult = null;
$('payloadEncodeBtn').addEventListener('click', async () => {
  const req = gatherPayloadRequest();
  if (!req) { toast('Fill in the form', 'error'); return; }
  loading(true);
  try {
    const json = await apiJSON('/api/payload/encode', {
      method:'POST', headers:{'Content-Type':'application/json'},
      body: JSON.stringify({payload: req, options:{format:'png', ec_level:'H', module_size:10, quiet_zone:4, foreground:'#000000', background:'#ffffff'}})
    });
    lastPayloadResult = json;
    const card = $('payloadResultCard');
    card.style.display = '';
    const img = new Image();
    img.src = `data:image/png;base64,${json.data}`;
    img.alt = 'QR';
    $('payloadPreview').innerHTML = '';
    $('payloadPreview').appendChild(img);
    $('payloadMeta').innerHTML = `${json.width}×${json.height} px`;
  } catch(e) { toast(e.message,'error'); }
  finally { loading(false); }
});

$('payloadDownload').addEventListener('click', () => {
  if (!lastPayloadResult) return;
  const byteStr = atob(lastPayloadResult.data);
  const buf = new Uint8Array(byteStr.length);
  for(let i=0;i<byteStr.length;i++) buf[i]=byteStr.charCodeAt(i);
  downloadBlob(new Blob([buf],{type:'image/png'}), `payload_qr_${Date.now()}.png`);
});
$('payloadCopy').addEventListener('click', () => {
  if (!lastPayloadResult) return;
  navigator.clipboard.writeText(lastPayloadResult.data).then(()=>toast('Copied b64','success'));
});

/* ═══════════════════════════════════════════════════════════════
   HISTORY
   ═══════════════════════════════════════════════════════════════ */
let histPage = 0;
const HIST_LIMIT = 30;

async function loadHistory() {
  const search = $('histSearch').value.trim();
  const offset = histPage * HIST_LIMIT;
  let url = `/api/history?limit=${HIST_LIMIT}&offset=${offset}`;
  try {
    const json = await apiJSON(url);
    renderHistory(json.entries || json, json.total);
  } catch(e) { toast(e.message,'error'); }
}

function renderHistory(entries, total) {
  const list = $('historyList');
  const search = $('histSearch').value.trim().toLowerCase();
  list.innerHTML = '';
  const filtered = search ? entries.filter(e => (e.data||'').toLowerCase().includes(search)) : entries;
  if (!filtered.length) {
    list.innerHTML = '<p style="color:var(--text-muted);text-align:center;padding:20px">No entries</p>';
    return;
  }
  filtered.forEach(entry => {
    const el = document.createElement('div');
    el.className = 'hist-item';
    const thumb = entry.thumbnail_b64
      ? `<img class="hist-thumb" src="data:image/png;base64,${entry.thumbnail_b64}" alt="thumb"/>`
      : `<div class="hist-thumb-none">▦</div>`;
    el.innerHTML = `
      ${thumb}
      <div class="hist-content">
        <div class="hist-content__data" title="${entry.data||''}">${entry.data || '—'}</div>
        <div class="hist-content__meta">${entry.kind||''} · ${fmtTs(entry.created_at)}</div>
      </div>
      <div class="hist-actions">
        <button class="btn btn--ghost btn--sm hist-reenc">↩</button>
        <button class="btn btn--danger btn--sm hist-del">🗑</button>
      </div>`;
    el.querySelector('.hist-del').addEventListener('click', async () => {
      try {
        await api(`/api/history/${entry.id}`, {method:'DELETE'});
        el.remove();
        toast('Deleted','success');
      } catch(e) { toast(e.message,'error'); }
    });
    el.querySelector('.hist-reenc').addEventListener('click', () => {
      encData.value = entry.data || '';
      document.querySelector('[data-tab="encode"]').click();
    });
    list.appendChild(el);
  });

  // Pagination
  const pg = $('histPagination');
  pg.innerHTML = '';
  if (typeof total === 'number' && total > HIST_LIMIT) {
    const pages = Math.ceil(total / HIST_LIMIT);
    for (let i=0; i<pages; i++) {
      const btn = document.createElement('button');
      btn.className = 'page-btn' + (i === histPage ? ' active' : '');
      btn.textContent = i+1;
      btn.addEventListener('click', () => { histPage = i; loadHistory(); });
      pg.appendChild(btn);
    }
  }
}

$('histRefresh').addEventListener('click', loadHistory);
$('histSearch').addEventListener('input', loadHistory);
$('histClear').addEventListener('click', async () => {
  if (!confirm('Clear entire history? This cannot be undone.')) return;
  // Delete all one by one or via bulk — fallback: just reload
  try {
    const json = await apiJSON(`/api/history?limit=200&offset=0`);
    const entries = json.entries || json;
    await Promise.all(entries.map(e => api(`/api/history/${e.id}`,{method:'DELETE'}).catch(()=>{})));
    histPage = 0;
    await loadHistory();
    toast('History cleared','success');
  } catch(e) { toast(e.message,'error'); }
});

/* ── Helpers ───────────────────────────────────────────────────── */
function isURL(s) {
  try { const u = new URL(s); return u.protocol === 'http:' || u.protocol === 'https:'; } catch { return false; }
}
