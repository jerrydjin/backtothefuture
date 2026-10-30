#!/usr/bin/env python3
"""Play saved demo → grant → denial in one white board, with no model calls."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import tempfile
import webbrowser

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--no-open", action="store_true")
p.add_argument("--binary")
a = p.parse_args()
root = Path(__file__).resolve().parent.parent
binary = Path(a.binary).resolve() if a.binary else root / "target/debug/muse"
if not a.binary:
    subprocess.run(["cargo", "build", "--quiet"], cwd=root, check=True)
with tempfile.TemporaryDirectory(prefix="muse-recording-") as directory:
    for clip, next_clip, seconds in [("demo", "grant", None), ("grant", "deny", 35), ("deny", None, 14)]:
        output = Path(directory) / f"{clip}.html"
        subprocess.run([str(binary), "--replay", str(root / f"recordings/{clip}.json"), "--export-replay", str(output)], check=True, stdout=subprocess.DEVNULL)
        if next_clip:
            done = 'document.querySelector("#network").dataset.demoComplete === "true"' if seconds is None else f"performance.now() >= {seconds * 1000}"
            script = "<script>setInterval(function(){if(" + done + "){location.replace(" + json.dumps(f"/{next_clip}.html?replay=1") + ");}},100);</script>"
            output.write_text(output.read_text() + script)
        else:
            output.write_text(output.read_text() + '<script>setTimeout(function(){document.querySelector("#network").dispatchEvent(new MouseEvent("click"));document.querySelector("#network").dataset.recordingComplete="true";},14000);</script>')
    handler = partial(SimpleHTTPRequestHandler, directory=directory)
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        url = f"http://127.0.0.1:{server.server_port}/demo.html?replay=1"
        print(f"recording: {url}", flush=True)
        print("Saved live words and trajectories; model calls=0. Ctrl-C closes playback.", flush=True)
        if not a.no_open:
            webbrowser.open(url)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
