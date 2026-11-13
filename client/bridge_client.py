#!/usr/bin/env python3
"""Client for a Muse agent (cloud side) to talk to the Muse Desktop bridge.

Usage:
  BRIDGE_URL=http://100.x.x.x:8765 BRIDGE_TOKEN=<token-from-control-panel> \
  BRIDGE_PROXY=<optional-proxy-url> \
    python3 bridge_client.py list .
    python3 bridge_client.py read notes.txt
    python3 bridge_client.py write notes.txt "content"
    python3 bridge_client.py exec "dir"

BRIDGE_PROXY is only needed when your agent environment reaches the
user's Tailscale network through an HTTP proxy (e.g. the Muse VM
tunnel proxy described in the Tailscale connector docs).
"""
import json
import os
import sys
import urllib.request


def call(method: str, route: str, payload: dict | None = None):
    url = os.environ["BRIDGE_URL"].rstrip("/") + route
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("X-Bridge-Token", os.environ.get("BRIDGE_TOKEN", ""))
    if data:
        req.add_header("Content-Type", "application/json")
    opener = urllib.request.build_opener()
    proxy = os.environ.get("BRIDGE_PROXY")
    if proxy:
        opener = urllib.request.build_opener(
            urllib.request.ProxyHandler({"http": proxy, "https": proxy}))
    try:
        with opener.open(req, timeout=90) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return json.loads(e.read().decode())


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    cmd = sys.argv[1]
    if cmd == "health":
        print(json.dumps(call("GET", "/health"), indent=2, ensure_ascii=False))
    elif cmd == "list":
        p = sys.argv[2] if len(sys.argv) > 2 else "."
        print(json.dumps(call("GET", f"/list?path={urllib.request.quote(p)}"),
                         indent=2, ensure_ascii=False))
    elif cmd == "read":
        print(call("GET", f"/read?path={urllib.request.quote(sys.argv[2])}").get("content", ""))
    elif cmd == "write":
        print(json.dumps(call("POST", "/write",
                              {"path": sys.argv[2], "content": sys.argv[3]}),
                         indent=2, ensure_ascii=False))
    elif cmd == "exec":
        print(json.dumps(call("POST", "/exec", {"command": sys.argv[2]}),
                         indent=2, ensure_ascii=False))
    else:
        print(f"perintah tidak dikenal: {cmd}")
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
