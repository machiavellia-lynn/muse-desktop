# Setup — connect your Muse to your laptop

You only do this once. Five short parts.

## 1. Install Tailscale on your PC

Install [Tailscale](https://tailscale.com/download) on your Windows PC
and sign in (a free account is fine). Leave it running — it creates the
private, encrypted network between your PC and your Muse, so the
bridge never has to be exposed to the internet.

Find your PC’s Tailscale address: click the Tailscale tray icon, or run
`tailscale ip -4` in a terminal. It looks like `100.x.x.x`. Muse
Desktop automatically binds its bridge to this address (port `8765`)
when Tailscale is up, and falls back to `127.0.0.1` (PC-only) when it
isn’t.

## 2. Let your Muse join your tailnet

Your Muse runs on its own cloud computer. It needs to join the *same*
Tailscale network:

1. In your Muse app, look for the **Tailscale** connector
   (Settings → Connectors), or simply tell your Muse:
   *“Please join my Tailscale network.”*
2. Your Muse will give you an approval link. Open it, sign in to
   Tailscale, and approve the new device (it appears as `muse`).
3. That’s it — your Muse can now reach other devices on your tailnet,
   outbound only.

## 3. Run Muse Desktop and sign in

1. Start **Muse Desktop**.
2. In the main window, sign in to **muse.ai** with your own Muse
   account — exactly like in a browser. The app itself never handles
   your password.
3. The **Control Panel** window shows:
   - the workspace folder (default `C:\Users\<you>\MuseWorkspace`) —
     change it if you like, this folder is the *only* place your Muse
     can touch;
   - the bridge address (e.g. `http://100.x.x.x:8765`);
   - your **pairing token** and its fingerprint.

## 4. Pair your Muse

1. Download [`client/bridge_client.py`](client/bridge_client.py) from
   this repo and give the file to your Muse (attach it in chat, or
   ask your Muse to fetch it from the repo).
2. Open [`client/PROMPT-FOR-YOUR-MUSE.md`](client/PROMPT-FOR-YOUR-MUSE.md),
   fill in your bridge address and token, and paste the prompt to your
   Muse.
3. Your Muse will test the connection (`/health`, then a folder
   listing).

## 5. Try it

Ask your Muse things like:

- “List the files in my laptop workspace.”
- “Read `notes.txt` from my laptop workspace.”
- “Save that summary as `summary.md` in my laptop workspace.”

Terminal commands stay off until you enable them in the Control Panel,
and even then every command asks you first, on your screen.

## Revoking access

- **Immediately:** quit Muse Desktop from its tray icon. The bridge
  stops; nothing can connect.
- **Permanently rotate the token:** delete `token.txt` in the app’s
  data folder (shown in the Control Panel log on startup) and restart
  the app — a fresh token is generated and the old one stops working.
- **Remove Muse from your network:** in the Tailscale admin console,
  remove the `muse` device.
