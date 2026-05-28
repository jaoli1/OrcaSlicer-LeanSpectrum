#!/usr/bin/env python3
"""
Render a Markdown file to a print-friendly PDF.
Markdown -> styled HTML (python-markdown) -> PDF via Microsoft Edge headless.
No pandoc / LaTeX needed.

Usage: python scripts/md_to_pdf.py <input.md> <output.pdf> [title]
"""
import os
import pathlib
import subprocess
import sys
import tempfile

import markdown  # pip install markdown

EDGE_CANDIDATES = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
]

CSS = """
<style>
@page { size: A4; margin: 18mm 16mm; }
body { font-family: 'Segoe UI', Arial, sans-serif; font-size: 11pt; line-height: 1.5; color: #1a1a1a; }
h1 { font-size: 21pt; color: #1a1a1a; border-bottom: 3px solid #d35400; padding-bottom: 5px; }
h2 { font-size: 15pt; color: #d35400; margin-top: 1.5em; border-bottom: 1px solid #e4e4e4; padding-bottom: 3px; }
h3 { font-size: 12.5pt; margin-top: 1.1em; }
code { background: #f4f4f4; padding: 1px 5px; border-radius: 3px; font-family: Consolas, 'SF Mono', monospace; font-size: 10pt; }
pre { background: #f4f4f4; padding: 10px 12px; border-radius: 6px; overflow-x: auto; }
pre code { background: none; padding: 0; }
table { border-collapse: collapse; width: 100%; margin: 10px 0; }
th, td { border: 1px solid #ccc; padding: 5px 9px; text-align: left; font-size: 10pt; vertical-align: top; }
th { background: #f0f0f0; }
blockquote { border-left: 3px solid #d35400; margin: 10px 0; padding: 4px 14px; color: #555; background: #faf3ee; }
a { color: #2980b9; text-decoration: none; }
ul, ol { margin: 6px 0 6px 0; padding-left: 22px; }
li { margin: 2px 0; }
h1, h2, h3 { page-break-after: avoid; }
table, pre, blockquote { page-break-inside: avoid; }
</style>
"""

def find_edge():
    for p in EDGE_CANDIDATES:
        if os.path.exists(p):
            return p
    raise SystemExit("Microsoft Edge not found (needed for headless PDF print).")

def main():
    src = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2]).resolve()  # Edge resolves --print-to-pdf vs its own cwd
    title = sys.argv[3] if len(sys.argv) > 3 else src.stem
    text = src.read_text(encoding="utf-8")
    body = markdown.markdown(
        text, extensions=["tables", "fenced_code", "toc", "sane_lists", "attr_list"]
    )
    html = (f"<!doctype html><html lang='fr'><head><meta charset='utf-8'>"
            f"<title>{title}</title>{CSS}</head><body>{body}</body></html>")
    with tempfile.NamedTemporaryFile("w", suffix=".html", delete=False, encoding="utf-8") as f:
        f.write(html)
        html_path = f.name
    url = pathlib.Path(html_path).as_uri()
    out.parent.mkdir(parents=True, exist_ok=True)
    profile = tempfile.mkdtemp(prefix="edge-pdf-")
    subprocess.run(
        [find_edge(), "--headless", "--disable-gpu", "--no-pdf-header-footer",
         f"--user-data-dir={profile}", f"--print-to-pdf={out}", url],
        check=True, timeout=180,
    )
    try:
        os.unlink(html_path)
    except OSError:
        pass
    print(f"wrote {out} ({out.stat().st_size} bytes)")

if __name__ == "__main__":
    main()
