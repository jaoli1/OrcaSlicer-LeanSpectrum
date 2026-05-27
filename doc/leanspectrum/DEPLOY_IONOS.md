# Deploy site/ to slicer.maisondrabiec.fr — IONOS VPS + nginx SSH workflow

## What's actually deployed

`maisondrabiec.fr` runs on an IONOS VPS at **217.160.175.248**
(Ubuntu, nginx, certbot, HTTP/3 enabled). NOT on a WebSpace product.

The VPS already serves `maisondrabiec.fr`, `coiffeur.maisondrabiec.fr`,
`strates-new.maisondrabiec.fr`, `social.maisondrabiec.fr`. Adding
`slicer.maisondrabiec.fr` is just another nginx vhost.

SSH access is via the `md-vps` host alias in `~/.ssh/config`
(IdentityFile `~/.ssh/maison_drabiec_vps_ed25519`, user `root`).

## What's already done (v0.1.0 deploy)

- `/var/www/slicer.maisondrabiec.fr/` created, owned `www-data`
- `site/index.html` + `site/style.css` uploaded via `scp`
- `/etc/nginx/sites-available/slicer` HTTP-only vhost installed +
  symlinked into `sites-enabled/`
- nginx reloaded, content verified via Host header (HTTP 200,
  title + hreflang + JSON-LD all served)
- `/usr/local/bin/slicer-tls-bootstrap.sh` installed on the VPS — a
  background loop that polls DNS for `slicer.maisondrabiec.fr → 217.160.175.248`,
  runs `certbot --nginx -d slicer.maisondrabiec.fr` once DNS resolves,
  adds HSTS, reloads nginx. Started under nohup, log at
  `/var/log/slicer-tls-bootstrap.log`

## Remaining manual step

**Add the DNS A record at IONOS panel.** The VPS can't do that for
you because the DNS zone is hosted at IONOS, not on the VPS.

1. Log into `https://my.ionos.fr/`
2. **Domaines & SSL** → click `maisondrabiec.fr`
3. **DNS** section (or **Adjust DNS settings**)
4. **Add record**:
   - Type: **A**
   - Host name: `slicer`
   - Points to: `217.160.175.248`
   - TTL: `3600` (1 hour) — IONOS default is fine
5. **Save**

Propagation: 5-30 min typical, up to 1h worst case.

The TLS bootstrap script on the VPS polls every 30 s. As soon as the
DNS record resolves correctly, it:
- Requests a Let's Encrypt cert via `certbot --nginx`
- Auto-rewrites the vhost to add HTTPS listener + 80→443 redirect
- Adds HSTS `max-age=15768000; includeSubDomains`
- Reloads nginx

You can `ssh md-vps 'tail -f /var/log/slicer-tls-bootstrap.log'` to
watch progress.

## Verification (once TLS is live)

```powershell
# DNS resolves to the VPS
nslookup slicer.maisondrabiec.fr 1.1.1.1
# Expected: Address 217.160.175.248

# HTTP redirects to HTTPS
curl.exe -sI http://slicer.maisondrabiec.fr/
# Expected: HTTP/1.1 301, Location: https://slicer.maisondrabiec.fr/

# HTTPS serves the page
curl.exe -sI https://slicer.maisondrabiec.fr/
# Expected: HTTP/2 200, Content-Type: text/html

# SEO surface (canonical, OG, JSON-LD, hreflang)
curl.exe -s https://slicer.maisondrabiec.fr/ | Select-String 'SoftwareApplication'

# HSTS header set
curl.exe -sI https://slicer.maisondrabiec.fr/ | Select-String 'Strict-Transport-Security'
```

All 5 passing = production live.

## Future updates

To redeploy after `site/` changes:

```powershell
# From repo root
scp site/index.html site/style.css md-vps:/var/www/slicer.maisondrabiec.fr/
ssh md-vps 'chown www-data:www-data /var/www/slicer.maisondrabiec.fr/*.{html,css}'
# nginx auto-serves new content — no reload needed
```

Or git-tracked workflow (would require setting up an `ssh-key` GitHub
Action with the deploy key + a webhook that scps on push to main).
Out of scope for v0.1.0.

## Architecture notes

- HTTP/3 + QUIC available on the VPS — `certbot --nginx` will not
  enable them automatically. The strates-new and social vhosts use
  `listen 443 quic reuseport;` — we can copy that pattern post-cert
  if h3 is wanted.
- Anti-leak rules in the vhost: deny `.git`, `.env`, `.htaccess`.
- Cache-Control: 5 min on `index.html`, 1 year `immutable` on .css.
- Gzip on for text/* mimetypes.

## Rollback

If we ever need to take slicer.maisondrabiec.fr down:

```bash
ssh md-vps 'rm /etc/nginx/sites-enabled/slicer && nginx -t && systemctl reload nginx'
```

Files in `/var/www/slicer.maisondrabiec.fr/` stay, cert at
`/etc/letsencrypt/live/slicer.maisondrabiec.fr/` stays. Re-symlinking
the vhost brings it back instantly. To fully remove:

```bash
ssh md-vps 'certbot delete --cert-name slicer.maisondrabiec.fr && \
  rm -rf /var/www/slicer.maisondrabiec.fr /etc/nginx/sites-available/slicer'
```
