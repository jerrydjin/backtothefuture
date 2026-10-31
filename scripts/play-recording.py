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
p.add_argument("--legacy", action="store_true", help="play the original demo → grant → denial")
p.add_argument("--recording", type=Path, help="play one saved rush-hour archive")
a = p.parse_args()
if a.legacy and a.recording:
    p.error("choose --legacy or --recording")
root = Path(__file__).resolve().parent.parent
binary = Path(a.binary).resolve() if a.binary else root / "target/debug/malleable"
sources = [root / "Cargo.toml", root / "Cargo.lock", *root.glob("src/**/*.rs")]
if not a.binary and (not binary.exists() or any(path.stat().st_mtime > binary.stat().st_mtime for path in sources if path.exists())):
    subprocess.run(["cargo", "build", "--quiet"], cwd=root, check=True)
with tempfile.TemporaryDirectory(prefix="malleable-recording-") as directory:
    rush = not a.legacy and (a.recording or (root / "recordings/rush-hour.json").exists())
    if rush:
        clips = ["rush-hour"] if a.recording else [c for c in ["rush-hour", "rush-grant", "rush-liar", "rush-deny"] if (root / f"recordings/{c}.json").exists()]
        sequence = [(c, clips[i+1] if i+1<len(clips) else None, None) for i,c in enumerate(clips)]
    else:
        sequence = [("demo", "grant", None), ("grant", "deny", 35), ("deny", None, 14)]
    for clip, next_clip, seconds in sequence:
        output = Path(directory) / f"{clip}.html"
        archive = a.recording.resolve() if a.recording else root / f"recordings/{clip}.json"
        subprocess.run([str(binary), "--replay", str(archive), "--export-replay", str(output)], check=True, stdout=subprocess.DEVNULL)
        if next_clip:
            done = 'document.querySelector("#network").dataset.rushComplete === "true"' if rush else ('document.querySelector("#network").dataset.demoComplete === "true"' if seconds is None else f"performance.now() >= {seconds * 1000}")
            script = "<script>var recordingAdvanced=false;setInterval(function(){if(!recordingAdvanced && (" + done + ")){recordingAdvanced=true;location.replace(" + json.dumps(f"/{next_clip}.html?replay=1") + ");}},100);</script>"
            output.write_text(output.read_text() + script)
        elif rush:
            output.write_text(output.read_text() + '<script>setInterval(function(){if(document.querySelector("#network").dataset.rushComplete==="true")document.querySelector("#network").dataset.recordingComplete="true";},100);</script>')
        else:
            output.write_text(output.read_text() + '<script>setTimeout(function(){document.querySelector("#network").dispatchEvent(new MouseEvent("click"));document.querySelector("#network").dataset.recordingComplete="true";},14000);</script>')
    handler = partial(SimpleHTTPRequestHandler, directory=directory)
    with ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        url = f"http://127.0.0.1:{server.server_port}/{sequence[0][0]}.html?replay=1"
        print(f"recording: {url}", flush=True)
        print("Saved words and trajectories; model calls=0. Ctrl-C closes playback.", flush=True)
        if not a.no_open:
            webbrowser.open(url)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
