// Muse Desktop — proses utama (Rust/Tauri).
//
// Dua jendela, dua dunia yang sengaja dipisah:
// - "muse": memuat https://muse.ai RESMI (Meta). Tidak ada preload,
//   tidak ada capability IPC (lihat capabilities/panel.json) — aplikasi
//   ini tidak bisa membaca, menyuntik, atau mencuri apa pun dari
//   halaman itu. Login diketik langsung ke Meta, seperti di browser.
// - "panel": halaman lokal Panel Kontrol, satu-satunya yang boleh
//   bicara ke Rust (status bridge, ganti folder, token, toggle exec,
//   tombol setuju/tolak perintah terminal).
//
// Bridge HTTP jalan di thread sendiri (crate muse-bridge), bind
// ke IP Tailscale atau 127.0.0.1, tidak pernah 0.0.0.0, dan tidak
// pernah membuat koneksi keluar.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use muse_bridge::{BridgeServer, BridgeState};
use rand::Rng;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Listener, Manager};

const PORT: u16 = 8765;

#[derive(Clone, Serialize)]
struct Status {
    workspace: String,
    address: String,
    token: String,
    token_fingerprint: String,
    exec_enabled: bool,
    easy_url: Option<String>,
    easy_on: bool,
}

struct AppState {
    bridge: BridgeState,
    address: String,
    pending: Mutex<HashMap<u64, std::sync::mpsc::Sender<bool>>>,
    counter: Mutex<u64>,
    log_lines: Mutex<Vec<String>>,
    easy_child: Mutex<Option<std::process::Child>>,
    easy_url: Mutex<Option<String>>,
}

fn fingerprint(token: &str) -> String {
    // FNV-1a sederhana hanya untuk sidik jari tampilan (bukan keamanan).
    let mut h: u64 = 0xcbf29ce484222325;
    for b in token.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", (h >> 32) as u32)
}

fn gen_token() -> String {
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            CHARS[rng.gen_range(0..CHARS.len())] as char
        })
        .collect()
}

fn push_log(app: &tauri::AppHandle, lines: &Mutex<Vec<String>>, msg: &str) {
    let line = format!(
        "[{}] {}",
        chrono_like_now(),
        msg
    );
    if let Ok(mut v) = lines.lock() {
        v.push(line.clone());
        if v.len() > 200 {
            let excess = v.len() - 200;
            v.drain(0..excess);
        }
    }
    let _ = app.emit_to("panel", "bridge-log", line);
}

fn chrono_like_now() -> String {
    // Tanpa crate tanggal: pakai detik sejak epoch -> HH:MM:SS UTC kasar.
    // (Log lokal untuk dibaca pemilik laptop; zona tidak kritis.)
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!(
        "{:02}:{:02}:{:02}",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}

#[tauri::command]
fn get_status(state: tauri::State<'_, Arc<AppState>>) -> Status {
    Status {
        workspace: state.bridge.workspace().to_string_lossy().into_owned(),
        address: format!("http://{}", state.address),
        token_fingerprint: fingerprint(&state.bridge.token),
        token: state.bridge.token.clone(),
        exec_enabled: state.bridge.allow_exec.load(Ordering::SeqCst),
        easy_url: state.easy_url.lock().unwrap().clone(),
        easy_on: state.easy_child.lock().unwrap().is_some(),
    }
}

#[tauri::command]
fn get_log(state: tauri::State<'_, Arc<AppState>>) -> Vec<String> {
    state.log_lines.lock().unwrap().clone()
}

#[tauri::command]
fn set_exec_enabled(state: tauri::State<'_, Arc<AppState>>, enabled: bool) {
    state.bridge.allow_exec.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
fn set_workspace(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    dir: String,
) -> Result<(), String> {
    state
        .bridge
        .set_workspace(&PathBuf::from(&dir))
        .map_err(|e| e.to_string())?;
    save_config(&app, &dir);
    Ok(())
}

// ---------- Mode Mudah (Cloudflare Tunnel, tanpa Tailscale) ----------
// cloudflared quick tunnel ke bridge di 127.0.0.1 (bridge selalu juga
// mendengarkan di loopback). URL acak trycloudflare = alamat publik
// sementara; token pairing tetap penjaganya. cloudflared diunduh
// sekali ke folder data aplikasi (bin/cloudflared.exe), tidak dibundel.

fn no_window(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
}

fn cloudflared_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("bin")
        .join("cloudflared.exe")
}

fn ensure_cloudflared(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let p = cloudflared_path(app);
    if p.exists() {
        return Ok(p);
    }
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    // URL unduhan resmi cloudflared (rakit dari potongan string).
    let url = concat!(
        "https://github.com/",
        "cloudflare/cloudflared/releases/",
        "latest/download/cloudflared-windows-amd64.exe"
    );
    let ps = format!(
        "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
        url,
        p.to_string_lossy()
    );
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
    no_window(&mut cmd);
    let st = cmd.status().map_err(|e| e.to_string())?;
    if st.success() && p.exists() {
        Ok(p)
    } else {
        Err("gagal mengunduh cloudflared — cek koneksi internet lalu coba lagi".into())
    }
}

#[tauri::command]
fn easy_mode_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if let Some(u) = state.easy_url.lock().unwrap().clone() {
        return Ok(u);
    }
    if state.easy_child.lock().unwrap().is_some() {
        return Err(
            "Mode Mudah sedang disiapkan — tunggu sebentar, alamatnya muncul di panel".into(),
        );
    }
    let exe = ensure_cloudflared(&app)?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.args([
        "tunnel",
        "--no-autoupdate",
        "--url",
        &format!("http://127.0.0.1:{PORT}"),
    ])
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());
    no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stderr = child.stderr.take().ok_or("cloudflared tanpa stderr")?;
    *state.easy_child.lock().unwrap() = Some(child);
    // Thread pembaca: saring URL trycloudflare dari output cloudflared.
    let app2 = app.clone();
    let state2 = state.inner().clone();
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if let Some(i) = line.find("https://") {
                let url = line[i..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(|c| c == '|' || c == '+')
                    .to_string();
                if url.contains("trycloudflare.com") {
                    *state2.easy_url.lock().unwrap() = Some(url.clone());
                    push_log(&app2, &state2.log_lines, &format!("MODE MUDAH AKTIF: {url}"));
                    let _ = app2.emit_to("panel", "easy-url", url);
                }
            }
        }
    });
    Ok("menyiapkan".into())
}

#[tauri::command]
fn easy_mode_stop(app: tauri::AppHandle, state: tauri::State<'_, Arc<AppState>>) {
    if let Some(mut ch) = state.easy_child.lock().unwrap().take() {
        let _ = ch.kill();
        let _ = ch.wait();
    }
    *state.easy_url.lock().unwrap() = None;
    let _ = app.emit_to("panel", "easy-url", String::new());
}

#[tauri::command]
fn exec_decision(state: tauri::State<'_, Arc<AppState>>, id: u64, approved: bool) {
    if let Some(tx) = state.pending.lock().unwrap().remove(&id) {
        let _ = tx.send(approved);
    }
}

fn config_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("app data dir")
        .join("config.json")
}

fn save_config(app: &tauri::AppHandle, workspace: &str) {
    let cfg = serde_json::json!({ "workspace": workspace });
    let _ = std::fs::write(config_path(app), cfg.to_string());
}

fn load_workspace(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(text) = std::fs::read_to_string(config_path(app)) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(w) = v["workspace"].as_str() {
                let p = PathBuf::from(w);
                if p.is_dir() {
                    return p;
                }
            }
        }
    }
    let default = app
        .path()
        .home_dir()
        .expect("home dir")
        .join("MuseWorkspace");
    let _ = std::fs::create_dir_all(&default);
    save_config(app, &default.to_string_lossy());
    default
}

fn load_token(app: &tauri::AppHandle) -> String {
    let path = app
        .path()
        .app_data_dir()
        .expect("app data dir")
        .join("token.txt");
    if let Ok(t) = std::fs::read_to_string(&path) {
        let t = t.trim().to_string();
        if t.len() >= 32 {
            return t;
        }
    }
    let t = gen_token();
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    let _ = std::fs::write(&path, &t);
    t
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_log,
            set_exec_enabled,
            set_workspace,
            exec_decision,
            easy_mode_start,
            easy_mode_stop
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let workspace = load_workspace(&handle);
            let token = load_token(&handle);
            let bind = muse_bridge::detect_tailscale_ip()
                .unwrap_or_else(|| "127.0.0.1".to_string());

            let log_lines = Mutex::new(Vec::<String>::new());
            let app_state_holder: Arc<Mutex<Option<Arc<AppState>>>> =
                Arc::new(Mutex::new(None));

            // Callback persetujuan exec: lempar event ke panel, tunggu
            // jawaban manusia (maks 60 detik, lewat = TOLAK).
            let approve: muse_bridge::ApproveFn = {
                let handle = handle.clone();
                let holder = app_state_holder.clone();
                Arc::new(move |command: &str, _ws: &std::path::Path| {
                    let state = holder.lock().unwrap().clone();
                    let Some(state) = state else { return false };
                    let (tx, rx) = std::sync::mpsc::channel::<bool>();
                    let id = {
                        let mut c = state.counter.lock().unwrap();
                        *c += 1;
                        *c
                    };
                    state.pending.lock().unwrap().insert(id, tx);
                    if let Some(win) = handle.get_webview_window("panel") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                    let _ = handle.emit_to(
                        "panel",
                        "exec-request",
                        serde_json::json!({ "id": id, "command": command }),
                    );
                    rx.recv_timeout(std::time::Duration::from_secs(60))
                        .unwrap_or(false)
                })
            };

            let log_fn: muse_bridge::LogFn = {
                let handle = handle.clone();
                Arc::new(move |msg: &str| {
                    // log_lines diakses lewat state; sebelum state siap,
                    // tulis ke stderr saja.
                    eprintln!("[bridge] {msg}");
                    let _ = handle.emit_to("panel", "bridge-log", msg.to_string());
                })
            };

            let bridge = BridgeState::new(workspace, token, false, approve, log_fn)
                .expect("workspace valid");
            let server = BridgeServer::start(bridge.clone(), &bind, PORT)
                .expect("bridge gagal bind");
            let address = server.addr.to_string();
            // Server sengaja tidak disimpan handle stop-nya: bridge hidup
            // selama aplikasi hidup; menutup aplikasi = bridge mati.
            std::mem::forget(server);

            // Bridge SELALU juga mendengarkan di 127.0.0.1 (kalau bind
            // utama bukan loopback): hanya bisa dijangkau dari laptop
            // ini sendiri, dan jadi target Mode Mudah (cloudflared).
            if bind != "127.0.0.1" {
                if let Ok(local) = BridgeServer::start(bridge.clone(), "127.0.0.1", PORT) {
                    std::mem::forget(local);
                }
            }

            let state = Arc::new(AppState {
                bridge,
                address,
                pending: Mutex::new(HashMap::new()),
                counter: Mutex::new(0),
                log_lines,
                easy_child: Mutex::new(None),
                easy_url: Mutex::new(None),
            });
            *app_state_holder.lock().unwrap() = Some(state.clone());
            app.manage(state);

            // Tray: ikon + menu sederhana.
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;
            let open_i = MenuItem::with_id(app, "open", "Buka Muse", true, None::<&str>)?;
            let panel_i =
                MenuItem::with_id(app, "panel", "Panel Kontrol", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Keluar (bridge mati)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &panel_i, &quit_i])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Muse Desktop — bridge aktif")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("muse") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "panel" => {
                        if let Some(w) = app.get_webview_window("panel") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        // Matikan cloudflared Mode Mudah dulu supaya
                        // tidak jadi proses yatim.
                        if let Some(st) = app.try_state::<Arc<AppState>>() {
                            if let Some(mut ch) = st.easy_child.lock().unwrap().take() {
                                let _ = ch.kill();
                                let _ = ch.wait();
                            }
                        }
                        app.exit(0)
                    }
                    _ => {}
                })
                .build(app)?;

            // Tutup jendela muse = sembunyikan (app tetap di tray),
            // bukan keluar. Berlaku juga untuk panel.
            for label in ["muse", "panel"] {
                if let Some(w) = app.get_webview_window(label) {
                    let w2 = w.clone();
                    w.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            let _ = w2.hide();
                        }
                    });
                }
            }
            let _ = app.listen("noop", |_| {});
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Muse Desktop gagal jalan");
}

// Penanda agar AtomicBool terpakai eksplisit (toggle exec dari tray nanti).
#[allow(dead_code)]
fn _unused(_: &AtomicBool) {}
