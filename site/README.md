# LeanSpectrum landing page

Static one-page presentation site for the LeanSpectrum slicer, designed
to deploy at https://slicer.maisondrabiec.fr.

## Files

- `index.html` — the page. Bilingual FR / EN (FR default, EN toggle in
  top-right; selection persists via localStorage).
- `style.css` — single stylesheet, dark theme, mobile-first.

No build step, no JS framework, no external assets. Open `index.html`
directly in a browser to preview.

## Deploy

Any static host works. Two paths:

### Static upload to maisondrabiec.fr

Upload `index.html` and `style.css` to the
`slicer.maisondrabiec.fr` document root. That's it.

### GitHub Pages (alternative)

Add a workflow that publishes this folder to a `gh-pages` branch.
Example minimal `.github/workflows/deploy-site.yml`:

```yaml
name: Deploy site
on:
  push:
    branches: [main]
    paths: [site/**]
permissions:
  contents: read
  pages: write
  id-token: write
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: github-pages
    steps:
      - uses: actions/checkout@v4
      - uses: actions/upload-pages-artifact@v3
        with:
          path: site
      - uses: actions/deploy-pages@v4
```

Then point `slicer.maisondrabiec.fr` (CNAME) at
`<owner>.github.io`.

## Content sources

Tables and numbers come straight from real test data:

- BambuConvert deltaE numbers — `tools/bambu-3mf-probe/probe.py`
  output on the user-provided `HarryPotter-+Color+Painted.3mf`
- Auto-Profile values — `src/libslic3r/AutoProfile.cpp` intent table
- Wave Overhangs status — `doc/leanspectrum/ROADMAP.md`
- Snapmaker U1 max_vol ceiling — wiki.snapmaker.com (snapshot in
  `doc/leanspectrum/FORKS_FEATURE_SURVEY.md`)
