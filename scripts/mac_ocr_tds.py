#!/usr/bin/env python3
"""
Runs ON the Mac (mac-roman). Deep-OCR a filament TDS PDF via a local Ollama
vision model and extract printing params as JSON. No process killing — uses the
GPU via Metal as Ollama already does.

Usage: python3 mac_ocr_tds.py <pdf_url> [model]
Renders page 1 at HD (pdftoppm -r 300 if available, else sips upscale), sends the
image to Ollama /api/generate, prints the model's JSON answer.
"""
import base64, json, os, subprocess, sys, tempfile, urllib.request

def main():
    url = sys.argv[1]
    model = sys.argv[2] if len(sys.argv) > 2 else "gemma3:27b"
    d = tempfile.mkdtemp()
    pdf, png = os.path.join(d, "f.pdf"), os.path.join(d, "f.png")
    urllib.request.urlretrieve(url, pdf)
    have_poppler = subprocess.run(["bash", "-lc", "command -v pdftoppm"],
                                  capture_output=True).returncode == 0
    if have_poppler:
        subprocess.run(["bash", "-lc", f"pdftoppm -png -r 300 -f 1 -l 1 -singlefile {pdf!r} {os.path.join(d,'f')!r}"],
                       check=True)
        renderer = "pdftoppm@300dpi"
    else:
        subprocess.run(["sips", "-s", "format", "png", "-Z", "2400", pdf, "--out", png],
                       capture_output=True)
        renderer = "sips@2400px"
    img = base64.b64encode(open(png, "rb").read()).decode()
    prompt = ("This image is a 3D-printing filament Technical Data Sheet. Read it carefully "
              "(deep OCR, including small tables). Output ONLY compact JSON with keys: "
              "nozzle_min, nozzle_max, bed_min, bed_max, density, dry_temp, dry_time. "
              "Temps in °C, density in g/cm3, dry_time in hours; numbers only; null if absent. No prose.")
    body = json.dumps({"model": model, "prompt": prompt, "images": [img],
                       "stream": False, "options": {"temperature": 0}}).encode()
    req = urllib.request.Request("http://localhost:11434/api/generate", data=body,
                                 headers={"Content-Type": "application/json"})
    resp = json.loads(urllib.request.urlopen(req, timeout=900).read())
    sys.stderr.write(f"[renderer={renderer} model={model} eval={resp.get('eval_count')} "
                     f"dur={round(resp.get('total_duration',0)/1e9,1)}s]\n")
    print(resp["response"].strip())

if __name__ == "__main__":
    main()
