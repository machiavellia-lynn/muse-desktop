<div align="center">

# Muse Desktop

### Your Muse, now with hands on your own Windows PC — safely.

A lightweight, open-source desktop companion that lets **your own Muse
agent** work in **one folder you choose** on your Windows PC.

[![Build](https://github.com/machiavellia-lynn/muse-desktop/actions/workflows/build-windows.yml/badge.svg)](https://github.com/machiavellia-lynn/muse-desktop/actions/workflows/build-windows.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)
![Built with Tauri + Rust](https://img.shields.io/badge/built%20with-Tauri%20%2B%20Rust-orange)

**English** · [Bahasa Indonesia](README.id.md)

</div>

> [!WARNING]
> **This is an unofficial, community project.** It is not made by Meta
> and is not affiliated with Meta. “Muse” is a trademark of Meta.
> This app only opens Meta’s official [muse.ai](https://muse.ai) in a
> window and adds a small, sandboxed local bridge on your own machine.
> The full source code is right here — read it before you trust it.

---

## ⬇️ Download

### [⬇️ Download the latest version](https://github.com/machiavellia-lynn/muse-desktop/releases/latest)

Download the .exe files from the official Release page — they are
listed under **Assets**:

| File | What it is |
|---|---|
| `Muse.Desktop_0.3.0_x64-setup.exe` | Installer (recommended) — [direct download](https://github.com/machiavellia-lynn/muse-desktop/releases/download/v0.3.0/Muse.Desktop_0.3.0_x64-setup.exe) |
| `muse-desktop.exe` | Portable — no install, just run it — [direct download](https://github.com/machiavellia-lynn/muse-desktop/releases/download/v0.3.0/muse-desktop.exe)|

> Windows may show a SmartScreen warning because the app is not
> code-signed yet. Click **More info → Run anyway** — or build it
> yourself from this source (instructions below) so you don’t have to
> trust anyone’s binary.

**You need:**
- ✅ A Windows 10/11 PC
- ✅ A Muse account (you sign in with *your own*)
- ✅ Nothing else — pick **Easy Mode** (built in, no account) or the
  more private **Tailscale** route (free account) in step 3
- ⏱️ About 10 minutes, once

---

## 🚀 Install in 4 steps

**1. Install the app**
Run the installer (or the portable `.exe`). Two windows open: the main
**Muse** window and a small **Control Panel**.

**2. Sign in**
In the main window, sign in to muse.ai with your own Muse account —
exactly like in your browser. The app never sees your password.

**3. Choose a connection mode**

*Easy Mode (recommended for most people)* — in the Control Panel,
click **Aktifkan Mode Mudah / Enable Easy Mode**. After ±30 seconds
the panel shows a temporary address like
`https://random-words.trycloudflare.com`. No account, no extra
install. The address changes every time the app restarts; treat it
like a password.

*Private Mode (Tailscale)* — install
[Tailscale](https://tailscale.com/download), sign in, leave it
running, and ask your Muse to join your network (walkthrough:
**[SETUP.md](SETUP.md)**). More private: the bridge is never exposed
to the public internet at all.

**4. Pair them**
Give your Muse two things from the Control Panel: the **address**
(the Easy Mode address, or `http://100.x.x.x:8765` on Tailscale) and
the **pairing token** — together with the ready-made prompt in
[client/PROMPT-FOR-YOUR-MUSE.md](client/PROMPT-FOR-YOUR-MUSE.md).

Done. Try asking your Muse:

> “List the files in my laptop workspace.”

---

## ✨ What your Muse can do with it

- 📁 List, read and create files — **only** inside the one workspace
  folder you picked
- 📝 Save summaries, drafts, code and reports straight onto your PC
- 🖥️ Run terminal commands — **only** if you enable it, and **only**
  after you click *Allow* on a dialog on your own screen, every time

## 🔒 What it can never do

- ❌ Touch anything outside your chosen folder (path escapes and
  symlink tricks are rejected in code, covered by tests)
- ❌ Run a command silently — there is no remote-exec-without-approval
  path, by design
- ❌ See your Muse password or account — sign-in happens on Meta’s
  real page; the app injects nothing into it and that window is denied
  all app permissions
- ❌ Phone home — there is no telemetry, no update server, no
  third-party cloud in the loop at all

Every access is logged live in the Control Panel. Closing the app from
the tray kills the bridge completely.

---

## 🧩 How it works

```
┌─ Your Windows PC ─────────────────────────────┐
│  Muse Desktop (one small Rust process)        │
│  ├─ Window 1 · muse.ai (official, by Meta)    │
│  │             no app permissions, no scripts  │
│  ├─ Window 2 · Control Panel (local page)     │
│  │             folder · token · log · approve  │
│  ├─ Tray icon · open / panel / quit           │
│  └─ Bridge · HTTP on your Tailscale IP only   │
└──────────────────┬────────────────────────────┘
                   │  Tailscale — encrypted,
                   │  your private network
┌──────────────────▼────────────────────────────┐
│  Your Muse · its own cloud computer          │
│  talks to the bridge with your pairing token  │
└───────────────────────────────────────────────┘
```

The bridge core is ~700 lines of Rust (`crates/muse-bridge`,
std + serde_json only) with unit tests for the sandbox, the token
check, and the approval flow: `cargo test -p muse-bridge`.

Full threat model: **[SECURITY.md](SECURITY.md)**.

---

## ❓ FAQ

**Is this the official Muse app for Windows?**
No. Meta does not offer a Windows app — Muse is on web, iOS, Android
and Mac. This is an independent open-source project (MIT) that opens
the official website in a window and adds the local bridge.

**Is it safe?**
Judge for yourself: the source is small and fully here; the terminal
is off by default and needs your click per command; the bridge only
listens on your private Tailscale network and demands a token only
you can see. The honest list of limitations is in
[SECURITY.md](SECURITY.md).

**Why Tailscale?**
Your Muse lives on its own cloud computer — it needs *some* private
road to your PC. Tailscale is free for personal use, encrypted, and
works without opening any port on your router. The bridge refuses to
listen on anything else.

**Which folder should I pick as the workspace?**
A dedicated folder (the default is `Documents`-style:
`C:\Users\<you>\MuseWorkspace`). Don’t point it at your whole drive,
Documents, or anything with secrets you wouldn’t hand your Muse
directly.

**Can I revoke access?**
Instantly: quit the app from the tray. Permanently: delete `token.txt`
in the app data folder (a new token is generated on next start) and/or
remove the `muse` device from your Tailscale admin page.

**macOS / Linux?**
Not yet. The bridge core is platform-neutral Rust; a Tauri build for
other systems is welcome as a contribution.

---

## 🛠️ Build from source

**The easy way (no tooling on your PC):** fork this repo → the
**Build Muse Desktop (Windows)** GitHub Actions workflow builds the
installer and portable exe for you, running the bridge tests first.

**Locally on Windows:**

```powershell
# needs: Rust (rustup) + Node.js 22
npm install
python scripts\make_icon.py   # pip install pillow first
npx tauri icon src-tauri\icons\icon.png
npx tauri build
```

## 📂 What’s in this repo

| Path | Contents |
|---|---|
| `crates/muse-bridge/` | The bridge core (pure Rust) + all tests |
| `src-tauri/` | Tauri shell — windows, tray, approval dialogs, permissions |
| `src/` | The Control Panel (plain HTML/CSS/JS) |
| `client/` | Tiny Python client + the pairing prompt for your Muse |
| `SETUP.md` | Step-by-step connection guide |
| `SECURITY.md` | Threat model & limitations, honestly |

## 📜 License

[MIT](LICENSE) — use it, fork it, share it. Keep the copyright notice
and the unofficial disclaimer: **this is not Meta’s product.**

<div align="center">

Made with Rust, Tauri, and a healthy distrust of closed-source binaries.

</div>
