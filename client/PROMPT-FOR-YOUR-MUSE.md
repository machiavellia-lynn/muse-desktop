# Pairing prompt — paste this to your Muse

Fill in the two values from the Muse Desktop Control Panel, attach
`bridge_client.py` (from this `client/` folder) to the same message,
and send it to your Muse:

---

I installed **Muse Desktop (unofficial)** on my Windows PC. It runs a
small local bridge that lets you work in ONE folder on my laptop.

- Bridge address: `http://100.x.x.x:8765` ← replace with yours
- Pairing token: `<paste token here>`

I attached `bridge_client.py`. Please use it like this:

```
BRIDGE_URL=http://100.x.x.x:8765 BRIDGE_TOKEN=<my token> \
  python3 bridge_client.py list .
```

Notes for you:

- If your environment reaches my Tailscale network through a proxy,
  set `BRIDGE_PROXY` accordingly (the Tailscale connector docs
  describe the tunnel proxy port). A `/health` call needs no token —
  test that first, then test `list .` with the token.
- The bridge only allows that one workspace folder. `list`, `read`
  and `write` work inside it; paths outside it will be rejected.
- `exec` runs a terminal command on my laptop, but **only** after I
  approve it in a dialog on my screen, and only if I enabled the
  terminal in the app. A rejection is normal — never retry a rejected
  command in a loop.
- Treat the token like a password: do not print it in your replies,
  do not save it into long-term memory, and do not send it anywhere
  except the bridge itself. Keep it only in the working file/env you
  use to call the client during our work together.

Please confirm the connection by listing my workspace folder.

---
