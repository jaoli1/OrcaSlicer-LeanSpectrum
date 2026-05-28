#!/usr/bin/env python3
"""
Render a Markdown file to a standalone, brand-styled HTML5 document.

The output uses semantic headings (<h1>/<h2>/<h3>), real <table>/<ul>/<ol>
markup and <blockquote> call-outs, so it can be:
  * opened directly in a browser to preview the product listing, and
  * pasted into a store/CMS rich-text editor via its "source code" view
    (the inner <body> content uses only standard TinyMCE-safe tags).

Markdown -> HTML via python-markdown (same extensions as md_to_pdf.py).

Usage: python scripts/md_to_html.py <input.md> <output.html> [title] [lang]
"""
import pathlib
import sys

import markdown  # pip install markdown

CSS = """
<style>
  :root { --accent: #d35400; --ink: #1a1a1a; --muted: #555; --line: #e4e4e4; }
  * { box-sizing: border-box; }
  body {
    font-family: 'Segoe UI', system-ui, -apple-system, Arial, sans-serif;
    font-size: 16px; line-height: 1.6; color: var(--ink);
    margin: 0; padding: 32px 16px; background: #fafafa;
  }
  main { max-width: 860px; margin: 0 auto; background: #fff;
         padding: 40px 48px; border-radius: 10px;
         box-shadow: 0 1px 4px rgba(0,0,0,.08); }
  h1 { font-size: 30px; line-height: 1.2; margin: 0 0 .6em;
       border-bottom: 3px solid var(--accent); padding-bottom: 10px; }
  h2 { font-size: 22px; color: var(--accent); margin-top: 1.8em;
       border-bottom: 1px solid var(--line); padding-bottom: 5px; }
  h3 { font-size: 18px; margin-top: 1.4em; }
  p { margin: .7em 0; }
  a { color: #2980b9; text-decoration: none; }
  a:hover { text-decoration: underline; }
  ul, ol { margin: .6em 0; padding-left: 26px; }
  li { margin: .25em 0; }
  code { background: #f4f4f4; padding: 2px 6px; border-radius: 4px;
         font-family: Consolas, 'SF Mono', monospace; font-size: .88em; }
  pre { background: #f4f4f4; padding: 12px 14px; border-radius: 6px; overflow-x: auto; }
  pre code { background: none; padding: 0; }
  table { border-collapse: collapse; width: 100%; margin: 14px 0; }
  th, td { border: 1px solid #ccc; padding: 8px 11px; text-align: left;
           vertical-align: top; font-size: .95em; }
  th { background: #f0f0f0; }
  blockquote { border-left: 4px solid var(--accent); margin: 14px 0;
               padding: 8px 16px; color: var(--muted); background: #faf3ee;
               border-radius: 0 6px 6px 0; }
  hr { border: 0; border-top: 1px solid var(--line); margin: 2em 0; }
</style>
"""


def main():
    src = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    title = sys.argv[3] if len(sys.argv) > 3 else src.stem
    lang = sys.argv[4] if len(sys.argv) > 4 else "fr"
    text = src.read_text(encoding="utf-8")
    body = markdown.markdown(
        text, extensions=["tables", "fenced_code", "sane_lists", "attr_list"]
    )
    html = (
        f"<!doctype html>\n<html lang='{lang}'>\n<head>\n"
        f"<meta charset='utf-8'>\n"
        f"<meta name='viewport' content='width=device-width, initial-scale=1'>\n"
        f"<title>{title}</title>\n{CSS}</head>\n<body>\n<main>\n"
        f"{body}\n</main>\n</body>\n</html>\n"
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(html, encoding="utf-8")
    print(f"wrote {out} ({out.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
