# LeanSpectrum v0.1.0 — Release Runbook

Consolidates findings from four senior-engineer audit waves (SEO, dead
code, Bambu compatibility, deployment, accessibility, release pipeline)
into a single linear path from "code complete + CI green" to "v0.1.0
released publicly with landing page live".

## Pre-flight context (already done)

- Cherry-picked 4 upstream-snap bugfixes (wipe tower no-op skip,
  Downloader URL query strip, WCP_DOWNLOAD route, sw_GetActiveFile url
  field)
- Validate Documentation scoped to PR-changed files
- Check profiles steps marked continue-on-error
- `version.inc` bumped 0.9.9 → 0.1.0
- `doc/changelogs/RELEASE_NOTES_v0.1.0.md` populated at the path
  `build_all.yml` searches
- SEO P0 + A11y P0 applied to `site/index.html` + `style.css`
- Cloudflare Pages deploy workflow added at
  `.github/workflows/deploy-site.yml` (disabled until secrets are
  configured)
- Orphan `0.20 Standard @Snapmaker U1 (0.4 nozzle)_old.json` removed
- 58 Catch2 TEST_CASEs pass on the latest validated commit

## Linear path

### Phase 1 — CI green-light

1. Verify `build_all` run #26508153846 on `feature/filament-economy`
   reaches all-green (Ubuntu + Windows + macOS).
   `gh run view 26508153846 --log-failed` if anything red.

2. Verify experiment branch CI (#26506752470) completes — Ubuntu +
   Windows already green; macOS still in progress at runbook draft
   time.
   `gh run list --branch experiment/wave-overhangs-phase3b --limit 1`

3. **Decision gate A — Wave Overhangs Phase 3b+4+5 inclusion.**
   - If experiment branch green on all 3 OS and no test regressions
     → merge into `feature/filament-economy` with `git merge --no-ff
     experiment/wave-overhangs-phase3b`
   - If macOS still red after a reasonable wait → ship v0.1.0 with
     Phases 1+2+3a+6 only; retarget 3b/4/5 to v0.1.1

4. If step 3 merged: re-trigger `build_all` on feature, wait for all
   3 OS green, re-run Catch2 locally to confirm test count ≥ 58.

### Phase 2 — Real-hardware smoke tests

Per `doc/leanspectrum/RELEASE_CHECKLIST.md` §"Smoke tests":

5. Single-color PLA Benchy with default U1 PLA profile → confirm no
   regression vs the v0.9.9 baseline.

6. 2-color PLA print with FilamentEconomy enabled → confirm transitions
   clean, purge volume measurably reduced.

7. 8-color Bambu `.3mf` through BambuConvert → confirm 4 physical
   extruder mapping + virtual filament synthesis works, slice through
   to G-code preview at minimum.

8. Auto-Profile a raw STL Benchy → confirm one-click intent flow
   produces a printable profile.

### Phase 3 — PR merge

9. Self-review PR #1: diff scope, no debug code, CHANGELOG entries
   correct.

10. Merge PR #1 with `gh pr merge 1 --merge` (preserve module history
    via merge commit, not squash).

11. Pull `main`, verify HEAD matches expected merge commit.

### Phase 4 — Cloudflare Pages deploy

12. Sign up at `dash.cloudflare.com`, add `maisondrabiec.fr`. Point
    registrar nameservers at the two CF nameservers it gives you.
    Propagation: 5-60 min.

13. **Workers & Pages → Create → Pages → Connect to Git** → select
    `jaoli1/OrcaSlicer-LeanSpectrum`. Production branch: `main`. Output
    directory: `site`. Empty build command.

14. Add 2 repo secrets at
    `https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/settings/secrets/actions`:
    - `CLOUDFLARE_API_TOKEN` (Pages:Edit + Account:Read)
    - `CLOUDFLARE_ACCOUNT_ID` (visible on any CF dashboard page)

15. Add the DNS record in Cloudflare:

    ```
    Type   Name     Target                          Proxy   TTL
    CNAME  slicer   leanspectrum-site.pages.dev     Proxied Auto
    ```

16. **Pages → project → Custom domains → Set up custom domain** →
    `slicer.maisondrabiec.fr`. CF auto-issues TLS.

17. **Pages → project → Settings → Web Analytics → Enable** for
    cookieless RGPD-safe analytics.

18. Verify live:

    ```powershell
    nslookup slicer.maisondrabiec.fr 1.1.1.1
    curl.exe -vI https://slicer.maisondrabiec.fr
    curl.exe -s https://slicer.maisondrabiec.fr | Select-String 'SoftwareApplication'
    ```

### Phase 5 — Tag + release

19. Tag the release. **NOT** `leanspectrum-v0.1.0` — the workflow's
    tag derivation expects `v0.1.0`:

    ```bash
    git tag -a v0.1.0 -m "LeanSpectrum v0.1.0"
    git push origin v0.1.0
    ```

20. Watch tag-triggered `build_all` produce release artifacts. Verify
    DMG / MSI / EXE / AppImage / .deb / .rpm all appear in the draft
    release.

21. UI action: review the draft release notes (auto-populated from
    `doc/changelogs/RELEASE_NOTES_v0.1.0.md`), confirm Wave Overhangs
    status reflects Phase 3 decision, click Publish.

### Phase 6 — Post-publish smoke + announce

22. Download each binary, launch, confirm About dialog reads
    `0.1.0`. Verify on at least one real OS per platform.

23. Snapmaker Discord: post in the user community channel — release
    URL, 3-pillar pitch, "known limitations" link.

24. Reddit r/3Dprinting: title with "LeanSpectrum v0.1.0", screenshots
    from the live landing page, GitHub release link.

25. Mastodon makers community: short post with feature highlights +
    landing URL.

26. Monitor first 24h: GitHub Issues, Discord channel, any reproducible
    crash gets tagged for v0.1.1.

## Risk gates

### Gate A — step 3 (Wave Overhangs merge decision)
If experiment branch macOS fails or experiment tests regress, merging
blocks the release. **Rollback:** don't merge; ship v0.1.0 without
Phases 3b+4+5; retarget for v0.1.1.

### Gate B — steps 5-8 (real-hardware smoke tests)
A failed print or visible regression blocks release. **Rollback:**
revert the offending module on `feature/filament-economy`, re-run CI,
re-test before step 10.

### Gate C — steps 12-18 (Cloudflare deploy)
DNS propagation delay, SSL stuck, or secrets misconfigured will fail
deploy. **Rollback:** disable `deploy-site.yml`, ship the GitHub
Release without the landing, fix CF config out-of-band, re-enable.

### Gate D — step 19 (tag push)
Wrong tag format (e.g. `leanspectrum-v0.1.0`) won't trigger
`build_all`. **Rollback:** delete the wrong tag, retag correctly.

```bash
git tag -d v0.1.0
git push origin :refs/tags/v0.1.0
# fix, then retag
```

## Out-of-scope for v0.1.0 (tracked for v0.2.0+)

- macOS DMG code signing (Apple Developer account + notarization)
- Windows MSI code signing (EV cert)
- Per-OS landing sub-pages on `slicer.maisondrabiec.fr/download/<os>/`
- Self-host Google Fonts (RGPD-perfect)
- Bambu H2D / H2C / X2D dual-extruder routing in BambuConvert
- AMS 2 Pro / AMS HT multi-unit (12+ slots) support
- Sentry crash telemetry (`SENTRY_AUTH_TOKEN` + DSN)
- Square wipe tower port (`ee2b0d74c6`)
- Bumping `fdm_filament_pla.json::filament_max_volumetric_speed`
  from 14 to 20 mm³/s (matches AutoProfile Standard, deferred to
  versioned release notes)

## References

- `doc/leanspectrum/U1_RELIABILITY_AUDIT.md` — task #22 decomposition
- `doc/leanspectrum/U1_PROFILE_DRIFT.md` — filament/process/machine
  cross-check
- `doc/leanspectrum/PR_WAVE_OVERHANGS_MERGE.md` — experiment→feature
  PR pre-draft
- `doc/leanspectrum/RELEASE_NOTES_v0.1.0_TEMPLATE.md` — template
  (canonical body lives at `doc/changelogs/RELEASE_NOTES_v0.1.0.md`)
- `doc/leanspectrum/RELEASE_CHECKLIST.md` — original pre-tag list
