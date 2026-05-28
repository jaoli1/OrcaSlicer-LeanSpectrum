# OptimusOrca landing page

Static one-page presentation site for the OptimusOrca slicer (by Maison
Drabiec), designed to deploy at https://slicer.maisondrabiec.fr.

## Files

- `index.html` — the page. Bilingual FR / EN (FR default, EN toggle in
  top-right; selection persists via localStorage). SEO-hardened:
  canonical URL, Open Graph, Twitter Card, hreflang, JSON-LD
  SoftwareApplication schema.
- `style.css` — single stylesheet, dark theme, mobile-first.

No build step, no JS framework, no external assets except Google Fonts
(JetBrains Mono + Inter, loaded with `preconnect` + `display=swap`).
Open `index.html` directly in a browser to preview.

## Deploy — recommended: Cloudflare Pages

Free, RGPD-compliant cookieless analytics, edge caching at 330+ PoPs,
auto Let's Encrypt TLS, PR previews. Setup is dashboard-driven:

1. Sign up at https://dash.cloudflare.com (free tier).
2. **Add site** `maisondrabiec.fr`. Cloudflare gives 2 nameservers — set
   these at your registrar. Propagation takes 5-60 min.
3. **Workers & Pages → Create → Pages → Connect to Git** → select this
   repo.
4. Build config:
   - Production branch: `main`
   - Build command: *(leave empty)*
   - Output directory: `site`
   - Root directory: *(leave empty)*
5. Add the DNS record at Cloudflare:

```
Type   Name     Target                          Proxy   TTL
CNAME  slicer   <project-name>.pages.dev        Proxied Auto
```

6. In **Pages → project → Custom domains → Set up custom domain** enter
   `slicer.maisondrabiec.fr`. Cloudflare auto-issues the TLS cert.
7. **Pages → project → Settings → Web Analytics → Enable** for
   cookieless RGPD-safe analytics (no consent banner needed under CNIL
   guidance).

The dashboard git connector auto-deploys every push to `main` as
production and every PR / branch as a preview at
`https://<hash>.leanspectrum-site.pages.dev`.

### Optional git-tracked workflow

`.github/workflows/deploy-site.yml` ships an alternative deploy flow
using `cloudflare/pages-action@v1`. Requires two repo secrets
(`CLOUDFLARE_API_TOKEN` with `Pages:Edit` + `Account:Read`, and
`CLOUDFLARE_ACCOUNT_ID`). Use this if you prefer the deploy config to
travel with the repo. It does the same thing as the dashboard
connector but with an explicit YAML trail.

## Deploy — alternatives

### Static upload to maisondrabiec.fr

Upload `index.html` and `style.css` to the `slicer.maisondrabiec.fr`
document root via FTP/SSH. Manual TLS setup required. No CI/CD.

### GitHub Pages

Add a workflow that publishes this folder to a `gh-pages` branch.
Example minimal `.github/workflows/deploy-site-pages.yml`:

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

Then point `slicer.maisondrabiec.fr` (CNAME) at `<owner>.github.io`.

## Verification (after deploy)

```powershell
nslookup slicer.maisondrabiec.fr 1.1.1.1
curl.exe -vI https://slicer.maisondrabiec.fr
curl.exe -s https://slicer.maisondrabiec.fr | Select-String 'SoftwareApplication'
```

All three should respond cleanly; the last must echo the JSON-LD type.

## Content sources

Tables and numbers come straight from real test data:

- BambuConvert deltaE numbers — `tools/bambu-3mf-probe/probe.py`
  output on the user-provided `HarryPotter-+Color+Painted.3mf`
- Auto-Profile values — `src/libslic3r/AutoProfile.cpp` intent table
- Wave Overhangs status — `doc/leanspectrum/ROADMAP.md`
- Snapmaker U1 max_vol ceiling — wiki.snapmaker.com (snapshot in
  `doc/leanspectrum/FORKS_FEATURE_SURVEY.md`)
