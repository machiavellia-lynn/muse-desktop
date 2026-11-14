// Panel Kontrol — satu-satunya halaman yang boleh memakai IPC Tauri.
// Memakai global Tauri (withGlobalTauri) supaya tidak perlu bundler.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

const $ = (id) => document.getElementById(id);
let pendingExecId = null;

async function refresh() {
  const s = await invoke('get_status');
  $('addr').textContent = 'bridge aktif di ' + s.address;
  $('workspace').textContent = s.workspace;
  $('token').textContent = s.token;
  $('fp').textContent = s.token_fingerprint;
  $('exec-toggle').checked = s.exec_enabled;
  $('easy-url').textContent = s.easy_url
    ? s.easy_url
    : (s.easy_on ? 'Menyiapkan Mode Mudah…' : 'Mode Mudah mati');
  const lines = await invoke('get_log');
  if (lines.length) $('log').textContent = lines.join('\n');
}

$('btn-easy-on').addEventListener('click', async () => {
  $('easy-url').textContent = 'Menyiapkan Mode Mudah… (unduhan pertama ±1 menit, sesudahnya ±30 detik)';
  try {
    await invoke('easy_mode_start');
  } catch (e) {
    $('easy-url').textContent = 'Gagal: ' + e;
  }
});

$('btn-easy-off').addEventListener('click', async () => {
  await invoke('easy_mode_stop');
  $('easy-url').textContent = 'Mode Mudah mati';
});

$('btn-copy-easy').addEventListener('click', async () => {
  const t = $('easy-url').textContent;
  if (t.startsWith('http')) {
    await navigator.clipboard.writeText(t);
    $('btn-copy-easy').textContent = 'Tersalin ✓';
    setTimeout(() => ($('btn-copy-easy').textContent = 'Salin alamat'), 1500);
  }
});

listen('easy-url', (e) => {
  $('easy-url').textContent = e.payload ? e.payload : 'Mode Mudah mati';
});

$('btn-copy').addEventListener('click', async () => {
  await navigator.clipboard.writeText($('token').textContent);
  $('btn-copy').textContent = 'Tersalin ✓';
  setTimeout(() => ($('btn-copy').textContent = 'Salin token'), 1500);
});

$('btn-folder').addEventListener('click', async () => {
  const dir = await open({ directory: true, multiple: false });
  if (dir) {
    await invoke('set_workspace', { dir });
    await refresh();
  }
});

$('exec-toggle').addEventListener('change', async (e) => {
  await invoke('set_exec_enabled', { enabled: e.target.checked });
});

$('btn-allow').addEventListener('click', async () => {
  if (pendingExecId !== null) await invoke('exec_decision', { id: pendingExecId, approved: true });
  pendingExecId = null;
  $('exec-card').classList.add('hidden');
});

$('btn-deny').addEventListener('click', async () => {
  if (pendingExecId !== null) await invoke('exec_decision', { id: pendingExecId, approved: false });
  pendingExecId = null;
  $('exec-card').classList.add('hidden');
});

listen('exec-request', (e) => {
  pendingExecId = e.payload.id;
  $('exec-cmd').textContent = e.payload.command;
  $('exec-card').classList.remove('hidden');
});

listen('bridge-log', (e) => {
  const el = $('log');
  el.textContent += (el.textContent ? '\n' : '') + e.payload;
  el.scrollTop = el.scrollHeight;
});

refresh();
