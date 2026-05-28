// Tauri global (exposed via withGlobalTauri: true). Tauri 2 stable puts
// invoke at window.__TAURI__.core.invoke; some older builds expose it at
// window.__TAURI__.invoke. Probe both so a runtime version drift doesn't
// silently kill the whole script (which would in turn break the tabs,
// the drop zone, the i18n picker — i.e. everything).
function resolveInvoke() {
  const t = window.__TAURI__;
  if (!t) {
    console.error("[Optimisateur MD] window.__TAURI__ is undefined — Tauri runtime did not inject the global. Check tauri.conf.json withGlobalTauri + CSP script-src.");
    return null;
  }
  if (t.core && typeof t.core.invoke === "function") return t.core.invoke;
  if (typeof t.invoke === "function")                return t.invoke;
  console.error("[Optimisateur MD] Could not find invoke() on window.__TAURI__. Keys:", Object.keys(t));
  return null;
}
const invoke = resolveInvoke();
// i18n helpers loaded from i18n.js (window.leanspectrumI18n.t / setLang).
const tr = (key, vars) => window.leanspectrumI18n.t(key, vars);

// ----- tabs -----
document.querySelectorAll(".tab").forEach(tab => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach(x => x.classList.toggle("active", x === tab));
    document.querySelectorAll(".tab-panel").forEach(p => {
      p.classList.toggle("active", p.id === `tab-${tab.dataset.tab}`);
    });
    if (tab.dataset.tab === "database" && !corpusInitialised) {
      corpusInitialised = true;
      initCorpusPath();
    }
  });
});

// ============================================================
// Single PDF tab
// ============================================================
const drop      = document.getElementById("drop");
const pickedEl  = document.getElementById("pickedPath");
const runBtn    = document.getElementById("run");
const status    = document.getElementById("status");
const result    = document.getElementById("result");
const logPanel  = document.getElementById("logPanel");
const logEl     = document.getElementById("log");
const fetchOnline = document.getElementById("fetchOnline");

let chosenPath = null;
function setChosen(path) {
  chosenPath = path;
  pickedEl.textContent = path ?? "";
  runBtn.disabled = !path;
}

drop.addEventListener("click", async () => {
  try {
    const p = await invoke("pick_pdf");
    if (p) setChosen(p);
  } catch (e) { status.textContent = `${tr("err_picker")}: ${e}`; }
});
["dragenter", "dragover"].forEach(ev => drop.addEventListener(ev, e => {
  e.preventDefault(); drop.classList.add("hover");
}));
["dragleave", "drop"].forEach(ev => drop.addEventListener(ev, e => {
  e.preventDefault(); drop.classList.remove("hover");
}));
drop.addEventListener("drop", e => {
  const path = e.dataTransfer?.files?.[0]?.path;
  if (path) setChosen(path);
});

runBtn.addEventListener("click", () => importPdfAndShow(chosenPath, fetchOnline.checked, result, status, logPanel, logEl, runBtn));

async function importPdfAndShow(path, online, resultEl, statusEl, logPanelEl, logElEl, runButton) {
  if (!path) return;
  if (runButton) runButton.disabled = true;
  statusEl.textContent = tr("status_working");
  resultEl.style.display = "none";
  if (logPanelEl) logPanelEl.style.display = "none";
  try {
    const r = await invoke("import_pdf", { req: { pdfPath: path, fetchOnline: online } });
    renderResult(r, resultEl, statusEl, logPanelEl, logElEl);
  } catch (e) {
    statusEl.textContent = `${tr("err_picker")}: ${e}`;
  } finally {
    if (runButton) runButton.disabled = false;
  }
}

function field(label, value) {
  if (value === null || value === undefined || value === "") return "";
  return `<div class="field"><span>${label}</span><span>${value}</span></div>`;
}
function renderResult(r, resultEl, statusEl, logPanelEl, logElEl) {
  const e = r.extracted;
  const badge = e.needs_review
    ? `<span class="badge review">${tr("needs_review_badge")}</span>`
    : `<span class="badge ok">${tr("ready_badge")}</span>`;
  resultEl.innerHTML = `
    <h2 style="margin-top:0;">${e.product_name ?? "Imported filament"} ${badge}</h2>
    <div class="sub">${e.manufacturer ?? "Unknown manufacturer"} — ${e.polymer ?? "Unknown polymer"}</div>
    ${field(tr("field_density"), e.density_g_cm3)}
    ${field(tr("field_glass"), e.glass_transition_c)}
    ${field(tr("field_melt"), e.melt_temp_min_c && e.melt_temp_max_c ? `${e.melt_temp_min_c} – ${e.melt_temp_max_c}` : null)}
    ${field(tr("field_decomp"), e.decomposition_c)}
    ${field(tr("field_nozzle"), e.nozzle_temp_min_c && e.nozzle_temp_max_c ? `${e.nozzle_temp_min_c} – ${e.nozzle_temp_max_c}` : null)}
    ${field(tr("field_bed"), e.bed_temp_min_c && e.bed_temp_max_c ? `${e.bed_temp_min_c} – ${e.bed_temp_max_c}` : null)}
    ${field(tr("field_max_flow"), e.max_flow_mm3_s)}
    ${field(tr("field_profile_saved"), r.profile_path ?? "(not saved)")}
    ${e.estimated_fields?.length ? `<div class="sub">${tr("estimated_fields")}: ${e.estimated_fields.join(", ")}</div>` : ""}
  `;
  resultEl.style.display = "block";
  if (logPanelEl && r.log?.length) {
    logElEl.innerHTML = r.log.map(l => `<div>${escapeHtml(l)}</div>`).join("");
    logPanelEl.style.display = "block";
  }
  statusEl.textContent = r.profile_path ? tr("open_orca") : tr("status_done");
}

// ============================================================
// Catalog tab
// ============================================================
const catalogUrl       = document.getElementById("catalogUrl");
const crawlBtn         = document.getElementById("crawlBtn");
const catalogPanel     = document.getElementById("catalogPanel");
const catalogList      = document.getElementById("catalogList");
const catalogSkipped   = document.getElementById("catalogSkipped");
const catalogStatus    = document.getElementById("catalogStatus");
const selectAllBtn     = document.getElementById("selectAll");
const selectNoneBtn    = document.getElementById("selectNone");
const batchImportBtn   = document.getElementById("batchImport");
const batchResult      = document.getElementById("batchResult");
const batchProgress    = document.getElementById("batchProgress");
const batchProgressBar = batchProgress.querySelector(".bar");
const catalogFetch     = document.getElementById("catalogFetchOnline");

let catalogEntries = [];

crawlBtn.addEventListener("click", async () => {
  const url = catalogUrl.value.trim();
  if (!url) return;
  crawlBtn.disabled = true;
  catalogStatus.textContent = tr("status_discovering");
  catalogPanel.style.display = "none";
  batchResult.style.display = "none";
  try {
    const r = await invoke("crawl_catalog", { url });
    catalogEntries = r.entries;
    renderCatalog(r);
  } catch (e) {
    catalogStatus.textContent = `${tr("err_discovery")}: ${e}`;
  } finally {
    crawlBtn.disabled = false;
  }
});

function badgeFor(docType) {
  const lower = (docType || "Unknown").toLowerCase();
  return `<span class="badge ${lower}">${docType ?? "Unknown"}</span>`;
}

function renderCatalog(r) {
  catalogPanel.style.display = "block";
  catalogStatus.textContent = `${r.entries.length} doc(s)`;
  catalogList.innerHTML = r.entries.map((e, i) => `
    <li>
      <input type="checkbox" data-i="${i}" ${e.doc_type !== "Unknown" ? "checked" : ""} />
      <div style="flex:1;">
        <div class="meta">
          ${badgeFor(e.doc_type)}
          ${e.guessed_polymer ? `<span class="badge unknown">${e.guessed_polymer}</span>` : ""}
          <span class="anchor">${escapeHtml(e.anchor_text || "(no anchor)")}</span>
        </div>
        <div class="url">${escapeHtml(e.url)}</div>
      </div>
    </li>
  `).join("");
  catalogSkipped.textContent = r.skipped?.length
    ? `Skipped ${r.skipped.length}: ${r.skipped.slice(0, 4).join("; ")}${r.skipped.length > 4 ? "…" : ""}`
    : "";
}

selectAllBtn .addEventListener("click", () => catalogList.querySelectorAll("input[type=checkbox]").forEach(c => c.checked = true));
selectNoneBtn.addEventListener("click", () => catalogList.querySelectorAll("input[type=checkbox]").forEach(c => c.checked = false));

batchImportBtn.addEventListener("click", async () => {
  const selected = Array.from(catalogList.querySelectorAll("input[type=checkbox]:checked"))
    .map(c => catalogEntries[parseInt(c.dataset.i, 10)].url);
  if (!selected.length) {
    catalogStatus.textContent = tr("status_no_docs");
    return;
  }
  batchImportBtn.disabled = true;
  catalogStatus.textContent = tr("status_working");
  batchProgress.style.display = "block";
  batchProgressBar.style.width = "10%";
  try {
    const r = await invoke("import_from_urls", {
      req: { urls: selected, fetchOnline: catalogFetch.checked }
    });
    batchProgressBar.style.width = "100%";
    renderBatchResult(r);
  } catch (e) {
    catalogStatus.textContent = `${tr("err_batch")}: ${e}`;
  } finally {
    batchImportBtn.disabled = false;
    setTimeout(() => { batchProgress.style.display = "none"; batchProgressBar.style.width = "0%"; }, 800);
  }
});

function renderBatchResult(r) {
  const ok   = r.succeeded?.length || 0;
  const fail = r.failed?.length    || 0;
  const lines = [];
  for (const item of (r.succeeded || [])) {
    const e = item.extracted;
    lines.push(`✓ ${e.product_name ?? "Imported"} (${e.polymer ?? "?"}) → ${item.profile_path ?? "(not saved)"}`);
  }
  for (const [url, err] of (r.failed || [])) {
    lines.push(`✗ ${url} — ${err}`);
  }
  batchResult.innerHTML = `
    <h2 style="margin-top:0;">${tr("batch_summary", { ok, fail })}</h2>
    <div class="log">${lines.map(l => `<div>${escapeHtml(l)}</div>`).join("")}</div>
  `;
  batchResult.style.display = "block";
  catalogStatus.textContent = tr("batch_done_status", { ok, fail });
}

// ============================================================
// Database tab (local corpus browser)
// ============================================================
const corpusPath    = document.getElementById("corpusPath");
const corpusScan    = document.getElementById("corpusScanBtn");
const corpusPanel   = document.getElementById("corpusPanel");
const corpusBrands  = document.getElementById("corpusBrands");
const corpusStatus  = document.getElementById("corpusStatus");
const corpusResult  = document.getElementById("corpusResult");
let corpusInitialised = false;

async function initCorpusPath() {
  try {
    const def = await invoke("corpus_default_path");
    corpusPath.value = corpusPath.value || def;
  } catch { /* ignore — user types path manually */ }
}

corpusScan.addEventListener("click", async () => {
  const path = corpusPath.value.trim();
  if (!path) return;
  corpusScan.disabled = true;
  corpusPanel.style.display = "none";
  corpusResult.style.display = "none";
  try {
    const r = await invoke("scan_corpus", { path });
    renderCorpus(r);
  } catch (e) {
    corpusPanel.style.display = "block";
    corpusStatus.textContent = `${tr("err_scan")}: ${e}`;
    corpusBrands.innerHTML = "";
  } finally {
    corpusScan.disabled = false;
  }
});

function renderCorpus(idx) {
  corpusPanel.style.display = "block";
  if (!idx.brands.length) {
    corpusStatus.textContent = tr("database_empty");
    corpusBrands.innerHTML = "";
    return;
  }
  corpusStatus.textContent = `${idx.pdf_count} PDFs / ${idx.brands.length} brands`;
  corpusBrands.innerHTML = idx.brands.map(b => `
    <div class="brand-group">
      <div class="brand-name">${escapeHtml(b.brand)} <span class="size">(${b.pdfs.length})</span></div>
      <ul class="brand-pdfs">
        ${b.pdfs.map(p => `
          <li data-path="${escapeHtml(p.absolute_path)}">
            <span>${escapeHtml(p.filename)}</span>
            <span class="size">${Math.round(p.size_bytes / 1024)} KB</span>
          </li>
        `).join("")}
      </ul>
    </div>
  `).join("");

  for (const li of corpusBrands.querySelectorAll("li[data-path]")) {
    li.addEventListener("click", () => {
      const p = li.dataset.path;
      const statusSpan = document.createElement("span");
      corpusStatus.textContent = `${tr("status_working")} — ${p}`;
      importPdfAndShow(p, false, corpusResult, corpusStatus, null, null, null);
    });
  }
}

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
}

// ============================================================
// Process library tab (v0.1.16) — write the 28 shared project-type process
// profiles (7 project types × 4 nozzles) into the Snapmaker_Orca user folder.
// ============================================================
const genLibraryBtn = document.getElementById("genLibraryBtn");
const libraryStatus = document.getElementById("libraryStatus");
const libraryResult = document.getElementById("libraryResult");
if (genLibraryBtn) {
  genLibraryBtn.addEventListener("click", async () => {
    genLibraryBtn.disabled = true;
    libraryStatus.textContent = tr("library_working");
    libraryResult.style.display = "none";
    try {
      const r = await invoke("generate_process_library");
      libraryResult.style.display = "block";
      libraryResult.innerHTML =
        `<strong>${r.count}</strong> ${escapeHtml(tr("library_done"))} <code>${escapeHtml(r.dir)}</code>`;
    } catch (e) {
      libraryResult.style.display = "block";
      libraryResult.textContent = `${tr("library_fail")}: ${e}`;
    } finally {
      libraryStatus.textContent = "";
      genLibraryBtn.disabled = false;
    }
  });
}
