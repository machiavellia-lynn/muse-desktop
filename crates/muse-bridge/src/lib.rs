//! Muse Bridge — server HTTP lokal yang tersandbox ke SATU folder.
//!
//! Aturan yang dikunci (sama di versi Python, Node, dan Rust ini):
//! - Semua route kecuali `/health` wajib header `X-Bridge-Token`
//!   (dibandingkan constant-time).
//! - Path di luar workspace ditolak, termasuk `../` dan symlink keluar
//!   (semua path di-canonicalize dulu sebelum dicek).
//! - Exec MATI kecuali `allow_exec` hidup, dan setiap perintah tetap
//!   harus lolos callback `approve` (di app: dialog di laptop pengguna).
//! - Bind `0.0.0.0` ditolak mentah-mentah.
//!
//! Modul ini murni std + serde_json: tidak ada framework web, tidak ada
//! koneksi keluar, supaya seluruh permukaan serangannya bisa dibaca.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub const MAX_READ: u64 = 1_000_000;
pub const MAX_WRITE: usize = 2_000_000;
pub const EXEC_TIMEOUT: Duration = Duration::from_secs(60);
pub const VERSION: &str = "0.3.0";

/// Callback persetujuan exec: (command, workspace) -> boleh jalan?
pub type ApproveFn = Arc<dyn Fn(&str, &Path) -> bool + Send + Sync>;
/// Callback log: dipanggil untuk setiap peristiwa (app menampilkannya
/// di panel + menulis ke file log).
pub type LogFn = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone)]
pub struct BridgeState {
    pub workspace: Arc<RwLock<PathBuf>>,
    pub token: String,
    pub allow_exec: Arc<AtomicBool>,
    pub approve: ApproveFn,
    pub log: LogFn,
}

impl BridgeState {
    pub fn new(
        workspace: PathBuf,
        token: String,
        allow_exec: bool,
        approve: ApproveFn,
        log: LogFn,
    ) -> std::io::Result<Self> {
        let ws = workspace.canonicalize()?;
        Ok(Self {
            workspace: Arc::new(RwLock::new(ws)),
            token,
            allow_exec: Arc::new(AtomicBool::new(allow_exec)),
            approve,
            log,
        })
    }

    pub fn workspace(&self) -> PathBuf {
        self.workspace.read().unwrap().clone()
    }

    pub fn set_workspace(&self, dir: &Path) -> std::io::Result<()> {
        let ws = dir.canonicalize()?;
        *self.workspace.write().unwrap() = ws;
        Ok(())
    }
}

/// Perbandingan token constant-time (panjang beda = langsung false,
/// isi dibandingkan tanpa early-exit).
pub fn token_eq(got: &str, want: &str) -> bool {
    let (a, b) = (got.as_bytes(), want.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Resolve `rel` di dalam workspace. None = di luar / ilegal.
/// Target yang belum ada dicek lewat parent-nya yang sudah ada,
/// supaya symlink di tengah path tetap tertangkap.
pub fn safe_path(workspace: &Path, rel: &str) -> Option<PathBuf> {
    // 1) Normalisasi komponen secara manual: ".." yang keluar dari
    //    workspace langsung ditolak, "." diabaikan.
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for comp in Path::new(rel).components() {
        use std::path::Component::*;
        match comp {
            Normal(c) => parts.push(c.to_os_string()),
            CurDir => {}
            ParentDir => {
                if parts.pop().is_none() {
                    return None;
                }
            }
            RootDir | Prefix(_) => return None,
        }
    }
    let mut candidate = workspace.to_path_buf();
    for p in &parts {
        candidate.push(p);
    }
    // 2) Canonicalize prefix terpanjang yang sudah ada (menangkap
    //    symlink di tengah path), lalu tempel sisa komponennya.
    let mut existing = candidate.clone();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        rest.push(existing.file_name()?.to_os_string());
        existing = existing.parent()?.to_path_buf();
    }
    let mut real = existing.canonicalize().ok()?;
    for p in rest.iter().rev() {
        real.push(p);
    }
    if real == workspace || real.starts_with(workspace) {
        Some(real)
    } else {
        None
    }
}

/// Tebak IP Tailscale mesin ini (100.64.0.0/10) tanpa crate tambahan:
/// "sambungkan" soket UDP ke 100.100.100.100 (Quad100, selalu ada di
/// tailnet) lalu baca alamat lokal yang dipilih OS. Tidak ada paket
/// yang benar-benar dikirim oleh connect() UDP.
pub fn detect_tailscale_ip() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("100.100.100.100:80").ok()?;
    let ip = sock.local_addr().ok()?.ip().to_string();
    let parts: Vec<u32> = ip.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == 4 && parts[0] == 100 && (64..=127).contains(&parts[1]) {
        Some(ip)
    } else {
        None
    }
}

// ---------------- HTTP minimal ----------------

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    token: String,
    body: Vec<u8>,
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if buf.len() > 65536 {
            return None;
        }
    }
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let first = lines.next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let raw_path = parts.next()?.to_string();

    let mut content_length = 0usize;
    let mut token = String::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            if k == "content-length" {
                content_length = v.parse().unwrap_or(0);
            } else if k == "x-bridge-token" {
                token = v.to_string();
            }
        }
    }
    if content_length > MAX_WRITE + 65536 {
        return None;
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);

    let (path, query) = match raw_path.split_once('?') {
        Some((p, q)) => {
            let pairs = q
                .split('&')
                .filter_map(|kv| {
                    let (k, v) = kv.split_once('=')?;
                    Some((url_decode(k), url_decode(v)))
                })
                .collect();
            (url_decode(p), pairs)
        }
        None => (url_decode(&raw_path), Vec::new()),
    };
    Some(Request { method, path, query, token, body })
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn respond(stream: &mut TcpStream, code: u32, obj: serde_json::Value) {
    let body = serde_json::to_vec(&obj).unwrap_or_default();
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

fn err(msg: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": msg })
}

fn query_get<'a>(req: &'a Request, key: &str) -> &'a str {
    req.query
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or(".")
}

// ---------------- handler ----------------

fn run_exec(state: &BridgeState, command: &str) -> (u32, serde_json::Value) {
    let ws = state.workspace();
    if !(state.approve)(command, &ws) {
        (state.log)(&format!("EXEC DITOLAK USER: {command}"));
        return (403, err("ditolak di laptop"));
    }
    (state.log)(&format!("EXEC DISETUJUI: {command}"));
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd.exe");
        c.args(["/d", "/s", "/c", command]);
        c
    } else {
        let mut c = std::process::Command::new("/bin/sh");
        c.args(["-c", command]);
        c
    };
    cmd.current_dir(&ws)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (500, err(&format!("gagal menjalankan: {e}"))),
    };
    // Baca stdout/stderr di thread terpisah supaya pipe tidak buntu.
    let mut out_pipe = child.stdout.take().unwrap();
    let mut err_pipe = child.stderr.take().unwrap();
    let (txo, rxo) = std::sync::mpsc::channel::<Vec<u8>>();
    let (txe, rxe) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = out_pipe.read_to_end(&mut v);
        let _ = txo.send(v);
    });
    std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = err_pipe.read_to_end(&mut v);
        let _ = txe.send(v);
    });
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if started.elapsed() > EXEC_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break None,
        }
    };
    match status {
        Some(s) => {
            let code = s.code().unwrap_or(-1);
            (state.log)(&format!("EXEC SELESAI (exit {code}): {command}"));
            let stdout = String::from_utf8_lossy(&rxo.recv().unwrap_or_default()).into_owned();
            let stderr = String::from_utf8_lossy(&rxe.recv().unwrap_or_default()).into_owned();
            let tail = |s: &str, n: usize| -> String {
                let rev: String = s.chars().rev().take(n).collect();
                rev.chars().rev().collect()
            };
            (
                200,
                serde_json::json!({
                    "ok": true,
                    "exit_code": code,
                    "stdout": tail(&stdout, 20000),
                    "stderr": tail(&stderr, 5000),
                }),
            )
        }
        None => {
            (state.log)(&format!("EXEC TIMEOUT: {command}"));
            (408, err("timeout 60s"))
        }
    }
}

fn handle(state: &BridgeState, req: &Request, stream: &mut TcpStream) {
    // /health satu-satunya tanpa token — hanya membuktikan bridge hidup.
    if req.method == "GET" && req.path == "/health" {
        return respond(
            stream,
            200,
            serde_json::json!({ "ok": true, "app": "muse-bridge", "version": VERSION }),
        );
    }
    if !token_eq(&req.token, &state.token) {
        (state.log)(&format!("TOLAK {} {} — token salah/tidak ada", req.method, req.path));
        return respond(stream, 401, err("unauthorized"));
    }
    let ws = state.workspace();

    if req.method == "GET" && req.path == "/list" {
        let rel = query_get(req, "path");
        let target = match safe_path(&ws, rel) {
            Some(p) => p,
            None => return respond(stream, 403, err("path di luar workspace")),
        };
        let meta = match std::fs::metadata(&target) {
            Ok(m) if m.is_dir() => m,
            _ => return respond(stream, 404, err("bukan folder / tidak ada")),
        };
        let _ = meta;
        let mut entries: Vec<serde_json::Value> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&target) {
            for e in rd.flatten() {
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = if is_dir {
                    serde_json::Value::Null
                } else {
                    e.metadata().map(|m| m.len().into()).unwrap_or(serde_json::Value::Null)
                };
                entries.push(serde_json::json!({
                    "name": e.file_name().to_string_lossy(),
                    "type": if is_dir { "dir" } else { "file" },
                    "size": size,
                }));
            }
        }
        entries.sort_by(|a, b| {
            let (ta, tb) = (a["type"].as_str().unwrap_or(""), b["type"].as_str().unwrap_or(""));
            tb.cmp(ta).then_with(|| {
                a["name"].as_str().unwrap_or("").to_lowercase()
                    .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
            })
        });
        (state.log)(&format!("LIST {rel} -> {} entri", entries.len()));
        return respond(stream, 200, serde_json::json!({ "ok": true, "path": rel, "entries": entries }));
    }

    if req.method == "GET" && req.path == "/read" {
        let rel = query_get(req, "path");
        let target = match safe_path(&ws, rel) {
            Some(p) => p,
            None => return respond(stream, 403, err("path di luar workspace")),
        };
        let meta = match std::fs::metadata(&target) {
            Ok(m) if m.is_file() => m,
            _ => return respond(stream, 404, err("file tidak ada")),
        };
        if meta.len() > MAX_READ {
            return respond(stream, 413, err("file > 1 MB"));
        }
        let data = std::fs::read(&target).unwrap_or_default();
        if data.contains(&0) {
            return respond(stream, 415, err("file biner — hanya teks"));
        }
        (state.log)(&format!("READ {rel} ({} byte)", meta.len()));
        return respond(
            stream,
            200,
            serde_json::json!({ "ok": true, "path": rel,
                "content": String::from_utf8_lossy(&data) }),
        );
    }

    if req.method == "POST" && req.path == "/write" {
        let data: serde_json::Value = serde_json::from_slice(&req.body).unwrap_or_default();
        let rel = data["path"].as_str().unwrap_or("");
        let content = data["content"].as_str();
        if rel.is_empty() || content.is_none() {
            return respond(stream, 400, err("butuh path + content (teks)"));
        }
        let content = content.unwrap();
        if content.len() > MAX_WRITE {
            return respond(stream, 413, err("konten terlalu besar"));
        }
        let target = match safe_path(&ws, rel) {
            Some(p) => p,
            None => return respond(stream, 403, err("path di luar workspace")),
        };
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&target, content) {
            Ok(()) => {
                (state.log)(&format!("WRITE {rel} ({} karakter)", content.chars().count()));
                let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
                return respond(stream, 200, serde_json::json!({ "ok": true, "path": rel, "bytes": size }));
            }
            Err(e) => return respond(stream, 500, err(&format!("gagal tulis: {e}"))),
        }
    }

    if req.method == "POST" && req.path == "/exec" {
        if !state.allow_exec.load(Ordering::SeqCst) {
            (state.log)("TOLAK EXEC — terminal mati");
            return respond(stream, 403, err("exec mati. Hidupkan dari Panel Kontrol Muse Desktop."));
        }
        let data: serde_json::Value = serde_json::from_slice(&req.body).unwrap_or_default();
        let command = data["command"].as_str().unwrap_or("");
        if command.trim().is_empty() {
            return respond(stream, 400, err("butuh command (string)"));
        }
        let (code, obj) = run_exec(state, command);
        return respond(stream, code, obj);
    }

    respond(stream, 404, err("route tidak dikenal"))
}

// ---------------- server ----------------

pub struct BridgeServer {
    stop: Arc<AtomicBool>,
    pub addr: std::net::SocketAddr,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl BridgeServer {
    pub fn start(state: BridgeState, bind: &str, port: u16) -> std::io::Result<Self> {
        if bind == "0.0.0.0" || bind == "::" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "bind 0.0.0.0 ditolak: bridge tidak boleh membuka ke semua jaringan",
            ));
        }
        let listener = TcpListener::bind((bind, port))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let handle = std::thread::spawn(move || {
            while !stop_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let st = state.clone();
                        std::thread::spawn(move || {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(90)));
                            if let Some(req) = read_request(&mut stream) {
                                handle(&st, &req, &mut stream);
                            }
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self { stop, addr, handle: Some(handle) })
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

// ---------------- tests ----------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    fn tmp_ws() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("fb-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    fn state(ws: &Path, allow_exec: bool, approve_ok: bool) -> BridgeState {
        BridgeState::new(
            ws.to_path_buf(),
            "test-token-1234567890".to_string(),
            allow_exec,
            Arc::new(move |_, _| approve_ok),
            Arc::new(|_| {}),
        )
        .unwrap()
    }

    fn raw(addr: std::net::SocketAddr, request: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_all(request.as_bytes()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    }

    fn get(addr: std::net::SocketAddr, path: &str, token: Option<&str>) -> String {
        let t = token.map(|t| format!("X-Bridge-Token: {t}\r\n")).unwrap_or_default();
        raw(addr, &format!("GET {path} HTTP/1.1\r\nHost: x\r\n{t}Connection: close\r\n\r\n"))
    }

    fn post(addr: std::net::SocketAddr, path: &str, body: &str, token: Option<&str>) -> String {
        let t = token.map(|t| format!("X-Bridge-Token: {t}\r\n")).unwrap_or_default();
        raw(addr, &format!(
            "POST {path} HTTP/1.1\r\nHost: x\r\n{t}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    }

    const TOK: &str = "test-token-1234567890";

    #[test]
    fn token_compare() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "abcd"));
    }

    #[test]
    fn safe_path_blocks_escape() {
        let ws = tmp_ws();
        assert!(safe_path(&ws, "a/b.txt").is_some());
        assert!(safe_path(&ws, "../secret").is_none());
        assert!(safe_path(&ws, "..").is_none());
    }

    #[cfg(unix)]
    #[test]
    fn safe_path_blocks_symlink() {
        let ws = tmp_ws();
        std::os::unix::fs::symlink("/etc/passwd", ws.join("link-keluar")).unwrap();
        assert!(safe_path(&ws, "link-keluar").is_none());
    }

    #[test]
    fn http_flow() {
        let ws = tmp_ws();
        std::fs::write(ws.join("catatan.txt"), "halo dari workspace\n").unwrap();
        let server = BridgeServer::start(state(&ws, true, true), "127.0.0.1", 0).unwrap();
        let a = server.addr;

        let r = get(a, "/health", None);
        assert!(r.contains("200") && r.contains("muse-bridge"), "{r}");

        let r = get(a, "/list?path=.", None);
        assert!(r.contains("401"), "{r}");

        let r = get(a, "/list?path=.", Some(TOK));
        assert!(r.contains("catatan.txt"), "{r}");

        let r = get(a, "/read?path=catatan.txt", Some(TOK));
        assert!(r.contains("halo"), "{r}");

        let r = get(a, "/read?path=..%2Fsecret", Some(TOK));
        assert!(r.contains("403"), "{r}");

        let r = post(a, "/write", r#"{"path":"baru/sub.txt","content":"isi baru"}"#, Some(TOK));
        assert!(r.contains("200"), "{r}");
        assert_eq!(std::fs::read_to_string(ws.join("baru/sub.txt")).unwrap(), "isi baru");

        let r = post(a, "/write", r#"{"path":"../evil.txt","content":"x"}"#, Some(TOK));
        assert!(r.contains("403"), "{r}");

        let r = post(a, "/exec", r#"{"command":"echo perintah-jalan"}"#, Some(TOK));
        assert!(r.contains("perintah-jalan"), "{r}");

        server.stop();
    }

    #[test]
    fn exec_denied_and_disabled() {
        let ws = tmp_ws();
        let server = BridgeServer::start(state(&ws, true, false), "127.0.0.1", 0).unwrap();
        let r = post(server.addr, "/exec", r#"{"command":"echo x"}"#, Some(TOK));
        assert!(r.contains("403"), "{r}");
        server.stop();

        let server = BridgeServer::start(state(&ws, false, true), "127.0.0.1", 0).unwrap();
        let r = post(server.addr, "/exec", r#"{"command":"echo x"}"#, Some(TOK));
        assert!(r.contains("403"), "{r}");
        server.stop();
    }

    #[test]
    fn bind_all_rejected() {
        let ws = tmp_ws();
        assert!(BridgeServer::start(state(&ws, false, true), "0.0.0.0", 0).is_err());
    }
}
