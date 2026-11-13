# Security model

## What this app is trusted with

Muse Desktop deliberately holds very little power:

- It can read/write **text files inside one folder you choose**.
- It can run terminal commands **only** if you enabled the terminal in
  the Control Panel, and **only** with your manual, per-command
  approval in a dialog on your own screen.
- It displays the official muse.ai website in a window. That window is
  given **no** app permissions (see `src-tauri/capabilities/` — only
  the `panel` window has any), and the app injects no scripts into it.

## What protects you

1. **Network position.** The bridge binds only to your Tailscale IP
   (or localhost) and refuses `0.0.0.0` in code. Nothing is exposed to
   your LAN or the internet by the app itself.
2. **Pairing token.** Every bridge request requires the token
   (compared in constant time). The token is generated on your machine
   and stored only there (app data folder) and wherever *you* paste it.
3. **Sandbox.** Paths are canonicalized and checked against the
   workspace root; `../` and symlink escapes are rejected. This is
   covered by unit tests in `crates/muse-bridge`.
4. **No outbound traffic.** The bridge only answers requests. There is
   no telemetry, no update server, no analytics.
5. **Auditability.** ~700 lines of Rust for the bridge core, plain
   HTML/JS for the panel. Every access is logged in the Control Panel.

## What the token does NOT protect

- Anyone who has your token **and** network access to the bridge
  (i.e. a device on your tailnet) can use the bridge as your Muse
  would. Treat the token like a password: never post it publicly,
  rotate it if it leaks (delete `token.txt`, restart the app).
- Terminal commands you approve run with **your** Windows account’s
  rights, inside your whole account — the workspace sandbox applies to
  the bridge’s file API, not to a shell you approved. Read every
  approval dialog. When in doubt, click **Deny**.
- Tailscale’s own security is part of the chain: keep your tailnet
  limited to devices you trust.

## Known limitations (v0.2)

- Text files only; 1 MB read / 2 MB write caps.
- The token is stored in plaintext in the app data folder (same
  trade-off most local tools make; OS-level disk encryption such as
  BitLocker is recommended). Hardening this is on the roadmap.
- The installer is not code-signed yet (unsigned builds may trigger a
  Windows SmartScreen prompt). Verify downloads against the checksums
  published with each release, or build from source via the included
  GitHub Actions workflow.

## Reporting a vulnerability

Open a GitHub issue with the label `security`, or contact the
maintainers privately through GitHub if the issue is sensitive.
Please do not include anyone’s pairing token in a report.
