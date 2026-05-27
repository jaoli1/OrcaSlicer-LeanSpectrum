# Deploy site/ to slicer.maisondrabiec.fr via IONOS

User-specific runbook for hosting LeanSpectrum landing at IONOS instead
of Cloudflare Pages. Replaces étapes 12-18 of `RELEASE_v0.1.0_RUNBOOK.md`.

## Pre-flight

- IONOS account active, `maisondrabiec.fr` administered there
- IONOS hosting product: typically **Web Hosting**, **WebSpace
  Essential/Plus**, or **Managed WordPress**. For a static 2-file site,
  the lightest tier works.
- Local PowerShell or equivalent with `curl.exe`, `WinSCP.com` or
  `lftp` for SFTP upload

## Phase A — IONOS panel: create the slicer subdomain

### A1 — Create the slicer.maisondrabiec.fr subdomain

1. Log in to `https://my.ionos.fr/` (or `.com` depending on your account).
2. Go to **Domains & SSL** → click on `maisondrabiec.fr`.
3. In the domain details, find the **Subdomains** section.
4. Click **Add Subdomain** (or **Ajouter un sous-domaine**).
5. Subdomain: `slicer`. Target: choose how to handle the routing — two
   options, A2 (alias / DNS pointer) or A3 (independent WebSpace).

### A2 — Option simple: alias on existing WebSpace

If you already have a WebSpace hosting `maisondrabiec.fr`, point the
subdomain there with a subfolder:

1. Subdomain `slicer` → **Use existing webspace** → target folder
   `/slicer/` (or whatever path you prefer).
2. IONOS creates a CNAME + folder. TLS is auto-provisioned via Let's
   Encrypt by IONOS in 2-15 min.
3. You'll upload `site/index.html` + `site/style.css` into that
   `/slicer/` folder.

### A3 — Option propre: independent WebSpace

For a dedicated hosting product (recommended if you want isolated
analytics, separate SFTP credentials, and no risk of bleeding into
the parent site):

1. **Hosting** → **Order new product** → **WebSpace Essential** (or
   any free-tier static-friendly product). Typically free if your
   IONOS plan includes it.
2. Once activated (5-30 min), go to the new WebSpace product page.
3. **Domains** → **Connect a domain** → `slicer.maisondrabiec.fr`.
4. IONOS creates the necessary DNS records automatically because the
   domain is registered with them.
5. Wait for **SSL Certificate** to show "Active" (Let's Encrypt
   auto-provisioned, 5-15 min).

## Phase B — Upload site/ to IONOS

You'll need SFTP credentials from the IONOS panel. They are usually
found at:

**Hosting** → your WebSpace → **Access Data** (or **Données d'accès**)

Note three values:
- **Server**: typically `access-xxx.webspace-data.io` or
  `homexxx.1and1.com`
- **Username**: `Uxxxxxxxx` (8-char alphanumeric)
- **Password**: set in the panel; can be regenerated if forgotten

### B1 — Upload via PowerShell + curl

For a 2-file static site, native PowerShell + curl works fine:

```powershell
$server   = "sftp://access-xxx.webspace-data.io"
$user     = "Uxxxxxxxx"
$pass     = Read-Host -AsSecureString "IONOS password" | ConvertFrom-SecureString -AsPlainText
$localDir = "C:\Users\olive\FORK ORCA POUR SNAPMAKER\repo\site"
$remote   = "/slicer/"   # or "/" if independent webspace

curl.exe -T "$localDir/index.html" -u "${user}:${pass}" "${server}${remote}index.html"
curl.exe -T "$localDir/style.css"  -u "${user}:${pass}" "${server}${remote}style.css"
```

### B2 — Upload via WinSCP (GUI, recommended for first-time)

1. Install WinSCP if absent: `winget install WinSCP.WinSCP`
2. Launch WinSCP. New site:
   - File protocol: **SFTP**
   - Host name: `access-xxx.webspace-data.io`
   - Username: `Uxxxxxxxx`
   - Password: from IONOS panel
3. Connect. Left panel: navigate to local `site/`. Right panel:
   navigate to the remote folder (`/slicer/` or `/`).
4. Drag-drop `index.html` and `style.css` to the right panel.

### B3 — Upload via lftp (cross-platform)

```bash
lftp -e "
  set sftp:auto-confirm yes;
  open -u Uxxxxxxxx,YOUR_PASSWORD sftp://access-xxx.webspace-data.io;
  cd /slicer/;
  put C:/Users/olive/FORK\ ORCA\ POUR\ SNAPMAKER/repo/site/index.html;
  put C:/Users/olive/FORK\ ORCA\ POUR\ SNAPMAKER/repo/site/style.css;
  bye
"
```

## Phase C — DNS verification

If A2 (alias), DNS is implicit (CNAME slicer → parent webspace).
If A3 (independent), IONOS sets `A` or `CNAME` records automatically.

### C1 — Confirm DNS resolution

```powershell
# Should resolve to an IONOS-managed IP
nslookup slicer.maisondrabiec.fr 1.1.1.1
```

Expected: an A record pointing to an IONOS IP (often `217.160.x.x` or
`82.165.x.x` ranges).

### C2 — Confirm TLS handshake + cert validity

```powershell
curl.exe -vI https://slicer.maisondrabiec.fr 2>&1 | Select-String "subject:|issuer:|HTTP/"
```

Expected: `HTTP/2 200`, issuer "Let's Encrypt" or "Sectigo" (IONOS
default CAs), CN matches `slicer.maisondrabiec.fr`.

### C3 — Confirm content + SEO surface served

```powershell
curl.exe -s https://slicer.maisondrabiec.fr | Select-String "LeanSpectrum|SoftwareApplication|hreflang"
```

Expected output includes:
- A line with `<title>LeanSpectrum — OrcaSlicer fork...</title>`
- A line with `"@type":"SoftwareApplication"` (the JSON-LD block)
- A line with `<link rel="alternate" hreflang="fr"`

If all 3 grep hits present → live and SEO-ready.

### C4 — Confirm no broken assets

```powershell
curl.exe -sI https://slicer.maisondrabiec.fr/style.css
```

Expected: `HTTP/2 200`, `content-type: text/css`.

## Phase D — Caching headers (optional, IONOS Pro tier)

IONOS WebSpace doesn't expose nginx config by default, but you can
ship an `.htaccess` next to `index.html` to set far-future cache on
static assets. Drop this in `site/` and re-upload:

```apache
<IfModule mod_headers.c>
  # Cache 1 year on static assets
  <FilesMatch "\.(css|js|svg|woff2|png|jpg|webp)$">
    Header set Cache-Control "public, max-age=31536000, immutable"
  </FilesMatch>
  # Short cache on index.html so updates propagate
  <FilesMatch "index\.html$">
    Header set Cache-Control "public, max-age=300"
  </FilesMatch>
</IfModule>
```

Skip if you don't see Apache mod_headers in your IONOS product. Most
WebSpace tiers run on Apache and support this by default.

## Phase E — Analytics (IONOS doesn't ship cookieless natively)

Cloudflare Pages would have given you free RGPD-safe analytics. IONOS
does not. Two options:

### E1 — Skip analytics for v0.1.0

Simplest. Re-evaluate post-launch if you actually need numbers.

### E2 — Plausible self-hosted on the same WebSpace

If you want stats:
1. Sign up at `https://plausible.io` (~9 €/mo cloud) OR self-host on
   a separate IONOS WebSpace.
2. Add their tracking script (8 lines) to `site/index.html` head:
   `<script defer data-domain="slicer.maisondrabiec.fr" src="https://plausible.io/js/script.js"></script>`
3. Cookieless, no consent banner needed under CNIL guidance.

Out of scope for v0.1.0 launch unless you already have a Plausible
account.

## Quick reference — full deploy command (PowerShell, one-shot)

Once IONOS subdomain + SFTP creds are set up:

```powershell
$creds = Get-Credential -UserName "Uxxxxxxxx" -Message "IONOS SFTP"
$plain = $creds.GetNetworkCredential().Password
$srv   = "sftp://access-xxx.webspace-data.io"
$path  = "/slicer/"
$local = "C:\Users\olive\FORK ORCA POUR SNAPMAKER\repo\site"

curl.exe -T "$local\index.html" -u "$($creds.UserName):$plain" "${srv}${path}index.html"
curl.exe -T "$local\style.css"  -u "$($creds.UserName):$plain" "${srv}${path}style.css"

# Verify
curl.exe -s https://slicer.maisondrabiec.fr | Select-String "SoftwareApplication"
```

## Updating later

Every time `site/` changes on `main`:
1. `git pull origin main` (local sync)
2. Re-run the two `curl.exe -T` lines above
3. Re-run the verify curl

No CI/CD needed since the user explicitly opted out of Cloudflare
Pages's git connector. If you change your mind, the
`.github/workflows/deploy-site.yml` file can be repointed at IONOS
via lftp-action (https://github.com/marketplace/actions/lftp-mirror-action)
— let me know and I'll wire that.
