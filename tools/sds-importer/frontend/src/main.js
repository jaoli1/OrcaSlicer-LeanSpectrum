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

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
}

// ----- tabs -----
let filamentInitialised = false;
document.querySelectorAll(".tab").forEach(tab => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach(x => x.classList.toggle("active", x === tab));
    document.querySelectorAll(".tab-panel").forEach(p => {
      p.classList.toggle("active", p.id === `tab-${tab.dataset.tab}`);
    });
    if (tab.dataset.tab === "filament" && !filamentInitialised) {
      filamentInitialised = true;
      loadFilaments("");
    }
  });
});

function fillSelect(sel, items, placeholder) {
  if (!sel) return;
  sel.innerHTML = "";
  const o0 = document.createElement("option");
  o0.value = "";
  o0.textContent = placeholder;
  sel.appendChild(o0);
  for (const it of items) {
    const o = document.createElement("option");
    o.value = String(it && it.value !== undefined ? it.value : it);
    o.textContent = String(it && it.label !== undefined ? it.label : it);
    sel.appendChild(o);
  }
}

// ============================================================
// Global slicer selector (v0.3.0) — chooses which OrcaSlicer-family slicer the
// generated presets are written into. "Autre…" reveals a custom folder input
// (with a native folder picker). Persisted in localStorage like the language.
// ============================================================
const gSlicer      = document.getElementById("gSlicer");
const gCustomDir   = document.getElementById("gCustomDir");
const customDirRow = document.getElementById("customDirRow");
const pickFolderBtn = document.getElementById("pickFolderBtn");
const SLICER_KEY = "leanspectrum_slicer";
const CUSTOM_DIR_KEY = "leanspectrum_custom_dir";

function refreshCustomDirVisibility() {
  if (customDirRow) customDirRow.style.display = (gSlicer && gSlicer.value === "custom") ? "" : "none";
}

/// { slicer: string, customDir: string|null } passed into every generate/import.
function slicerArgs() {
  const slicer = (gSlicer && gSlicer.value) || "snapmaker";
  const customDir = (gCustomDir && gCustomDir.value.trim()) || null;
  return { slicer, customDir };
}

if (gSlicer) {
  // Restore the persisted choice.
  const savedSlicer = localStorage.getItem(SLICER_KEY);
  if (savedSlicer) gSlicer.value = savedSlicer;
  if (gCustomDir) gCustomDir.value = localStorage.getItem(CUSTOM_DIR_KEY) || "";
  refreshCustomDirVisibility();
  gSlicer.addEventListener("change", () => {
    localStorage.setItem(SLICER_KEY, gSlicer.value);
    refreshCustomDirVisibility();
  });
}
if (gCustomDir) {
  gCustomDir.addEventListener("input", () => localStorage.setItem(CUSTOM_DIR_KEY, gCustomDir.value.trim()));
}
if (pickFolderBtn && invoke) {
  pickFolderBtn.addEventListener("click", async () => {
    try {
      const p = await invoke("pick_folder");
      if (p && gCustomDir) {
        gCustomDir.value = p;
        localStorage.setItem(CUSTOM_DIR_KEY, p);
      }
    } catch (e) { console.error(e); }
  });
}

// ============================================================
// Global printer selector (v0.2.0) — brand → model → nozzle (or "all nozzles").
// Shared by the Filament library (one-click) and the Process library tab.
// ============================================================
const gVendor = document.getElementById("gVendor");
const gModel  = document.getElementById("gModel");
const gNozzle = document.getElementById("gNozzle");

/// {vendor, model, nozzle:number|null, all:bool} or null when incomplete.
function currentPrinter() {
  if (!gVendor || !gVendor.value || !gModel.value) return null;
  const nv = gNozzle.value;
  if (nv === "all") return { vendor: gVendor.value, model: gModel.value, nozzle: null, all: true };
  if (nv)           return { vendor: gVendor.value, model: gModel.value, nozzle: parseFloat(nv), all: false };
  // Model chosen but no nozzle yet → backend defaults to 0.4 / smallest.
  return { vendor: gVendor.value, model: gModel.value, nozzle: null, all: false };
}

function refreshGenEnabled() {
  const p = currentPrinter();
  if (genForPrinter)       genForPrinter.disabled = !p;
  if (genFilamentProcess)  genFilamentProcess.disabled = !(p && selectedFilamentIds.size > 0);
}

if (gVendor && invoke) {
  gVendor.addEventListener("change", async () => {
    gModel.disabled = true; gNozzle.disabled = true;
    fillSelect(gModel, [], tr("library_model"));
    fillSelect(gNozzle, [], tr("library_nozzle"));
    refreshGenEnabled();
    if (!gVendor.value) return;
    try {
      fillSelect(gModel, await invoke("list_printer_models", { vendor: gVendor.value }), tr("library_model"));
      gModel.disabled = false;
    } catch (e) { console.error(e); }
  });
  gModel.addEventListener("change", async () => {
    gNozzle.disabled = true;
    fillSelect(gNozzle, [], tr("library_nozzle"));
    refreshGenEnabled();
    if (!gModel.value) return;
    try {
      const nz = await invoke("list_printer_nozzles", { vendor: gVendor.value, model: gModel.value });
      const items = nz.map(n => ({ value: n, label: n + " mm" }));
      items.push({ value: "all", label: tr("printer_all") });
      fillSelect(gNozzle, items, tr("library_nozzle"));
      gNozzle.disabled = false;
      refreshGenEnabled(); // model chosen → process generation allowed (nozzle optional)
    } catch (e) { console.error(e); }
  });
  gNozzle.addEventListener("change", refreshGenEnabled);
  invoke("list_printer_vendors")
    .then(v => fillSelect(gVendor, v, tr("library_vendor")))
    .catch(() => {});
}

// ============================================================
// Filament library tab (v0.2.0) — search the DB, pick a material, then
// one-click generate the filament profile + the 7 process profiles for the
// printer chosen in the global selector.
// ============================================================
const filamentSearch     = document.getElementById("filamentSearch");
const filamentBrand      = document.getElementById("filamentBrand");
const filamentList       = document.getElementById("filamentList");
const filamentCount      = document.getElementById("filamentCount");
const genFilamentProcess = document.getElementById("genFilamentProcess");
const filamentStatus     = document.getElementById("filamentStatus");
const filamentResult     = document.getElementById("filamentResult");
// v0.3.0 — multi-select: a Set of selected material ids (click-to-toggle).
let selectedFilamentIds = new Set();
let searchTimer = null;

/// Update the count line: "<N matches> · <M selected>".
function updateFilamentCount(matchCount) {
  if (!filamentCount) return;
  const parts = [];
  parts.push(matchCount ? String(matchCount) : tr("filament_none"));
  if (selectedFilamentIds.size > 0) {
    parts.push(tr("filament_selected", { n: selectedFilamentIds.size }));
  }
  filamentCount.textContent = parts.join(" · ");
}

/// Currently-selected brand in the "Marque" filter, or null for "all brands".
function currentBrand() {
  return (filamentBrand && filamentBrand.value) ? filamentBrand.value : null;
}

async function loadFilaments(query) {
  if (!invoke || !filamentList) return;
  try {
    const rows = await invoke("list_filaments", { query: query || null, brand: currentBrand() });
    renderFilaments(rows);
  } catch (e) {
    filamentList.innerHTML = "";
    filamentCount.textContent = "";
    filamentResult.style.display = "block";
    filamentResult.textContent = String(e);
  }
}

function renderFilaments(rows) {
  // Keep selections that are still visible in the (possibly filtered) list.
  const visibleIds = new Set(rows.map(r => r.id));
  selectedFilamentIds = new Set([...selectedFilamentIds].filter(id => visibleIds.has(id)));
  updateFilamentCount(rows.length);
  refreshGenEnabled();
  filamentList.innerHTML = rows.map(r => {
    const fam    = escapeHtml(r.base_type || "?") + (r.filled_type ? " " + escapeHtml(r.filled_type) : "");
    const params = r.has_params ? `<span class="badge params" title="manufacturer temps">°C</span>` : "";
    const colors = r.colors ? `<span class="sub">${r.colors} ◐</span>` : "";
    const dens   = r.density ? `<span class="sub">${r.density} g/cm³</span>` : "";
    const checked = selectedFilamentIds.has(r.id) ? " checked" : "";
    const sel     = selectedFilamentIds.has(r.id) ? " selected" : "";
    return `<li data-id="${r.id}" class="${sel.trim()}">
      <input type="checkbox" class="filament-check"${checked} aria-label="select" />
      <div style="flex:1;">
        <div class="meta">
          <span class="badge fam">${fam}</span>
          <span class="anchor">${escapeHtml(r.brand)} — ${escapeHtml(r.label)}</span>
          ${params} ${colors} ${dens}
        </div>
      </div>
    </li>`;
  }).join("");
  for (const li of filamentList.querySelectorAll("li[data-id]")) {
    const id = parseInt(li.dataset.id, 10);
    const check = li.querySelector(".filament-check");
    const toggle = (on) => {
      if (on) selectedFilamentIds.add(id); else selectedFilamentIds.delete(id);
      li.classList.toggle("selected", on);
      if (check) check.checked = on;
      updateFilamentCount(rows.length);
      refreshGenEnabled();
    };
    li.addEventListener("click", (e) => {
      // A direct checkbox click already flips it; honour its new state.
      if (e.target === check) { toggle(check.checked); return; }
      toggle(!selectedFilamentIds.has(id));
    });
  }
}

if (filamentSearch) {
  filamentSearch.addEventListener("input", () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => loadFilaments(filamentSearch.value.trim()), 200);
  });
}

// "Marque" filter — populated once on load; re-queries (keeping the free-text
// search) whenever the brand changes. First option ("All brands") = no filter.
if (filamentBrand && invoke) {
  filamentBrand.addEventListener("change", () => {
    loadFilaments(filamentSearch ? filamentSearch.value.trim() : "");
  });
  invoke("list_filament_brands")
    .then(brands => {
      for (const b of brands || []) {
        const o = document.createElement("option");
        o.value = b;
        o.textContent = b;
        filamentBrand.appendChild(o);
      }
    })
    .catch(() => {});
}

if (genFilamentProcess) {
  genFilamentProcess.addEventListener("click", async () => {
    const p = currentPrinter();
    if (!p)                         { filamentStatus.textContent = tr("filament_pick_printer"); return; }
    if (selectedFilamentIds.size === 0) { filamentStatus.textContent = tr("filament_pick_one"); return; }
    const sl = slicerArgs();
    if (sl.slicer === "custom" && !sl.customDir) { filamentStatus.textContent = tr("slicer_pick_folder"); return; }
    genFilamentProcess.disabled = true;
    filamentStatus.textContent = tr("filament_working");
    filamentResult.style.display = "none";
    try {
      const r = await invoke("generate_filament_and_process", {
        materialIds: [...selectedFilamentIds],
        vendor: p.vendor,
        model: p.model,
        nozzle: p.nozzle,
        allNozzles: p.all,
        slicer: sl.slicer,
        customDir: sl.customDir,
      });
      const printers = (r.printers || []).join(", ");
      const names = (r.filamentNames || []).map(n => `<code>${escapeHtml(n)}</code>`).join("<br/>");
      filamentResult.style.display = "block";
      filamentResult.innerHTML =
        `<h2 style="margin-top:0;">${escapeHtml(tr("filament_result_for", { printer: printers }))}</h2>`
        + `<div class="field"><span>${escapeHtml(tr("filament_result_filament"))} (${r.filamentCount})</span><span>${names}</span></div>`
        + `<div class="field"><span>${escapeHtml(tr("filament_result_process"))}</span><span><strong>${r.processCount}</strong></span></div>`
        + `<div class="sub" style="margin-top:8px;"><code>${escapeHtml(r.processDir)}</code></div>`;
      filamentStatus.textContent = tr("open_slicer");
    } catch (e) {
      filamentResult.style.display = "block";
      filamentResult.textContent = String(e);
      filamentStatus.textContent = "";
    } finally {
      genFilamentProcess.disabled = false;
      refreshGenEnabled();
    }
  });
}

// ============================================================
// Process library tab — the 7 project-type process profiles for the printer
// chosen in the global selector (the nozzle selector's "all nozzles" option
// covers generating the full set). Written into the chosen slicer.
// ============================================================
const genForPrinter = document.getElementById("genForPrinter");
const libraryStatus = document.getElementById("libraryStatus");
const libraryResult = document.getElementById("libraryResult");

function renderLibraryResult(r) {
  libraryResult.style.display = "block";
  libraryResult.innerHTML = `<strong>${r.count}</strong> ${escapeHtml(tr("library_done"))} <code>${escapeHtml(r.dir)}</code>`;
}

if (genForPrinter) {
  genForPrinter.addEventListener("click", async () => {
    const p = currentPrinter();
    if (!p) { libraryStatus.textContent = tr("filament_pick_printer"); return; }
    const sl = slicerArgs();
    if (sl.slicer === "custom" && !sl.customDir) { libraryStatus.textContent = tr("slicer_pick_folder"); return; }
    genForPrinter.disabled = true;
    libraryStatus.textContent = tr("library_working");
    libraryResult.style.display = "none";
    try {
      const r = await invoke("generate_process_library_for", {
        vendor: p.vendor, model: p.model, nozzle: p.nozzle, allNozzles: p.all,
        slicer: sl.slicer, customDir: sl.customDir,
      });
      renderLibraryResult(r);
    } catch (e) {
      libraryResult.style.display = "block";
      libraryResult.textContent = `${tr("library_fail")}: ${e}`;
    } finally {
      libraryStatus.textContent = "";
      genForPrinter.disabled = false;
      refreshGenEnabled();
    }
  });
}

// ============================================================
// Single PDF tab — fallback for a filament that is not yet in the database.
// Imports one SDS/TDS PDF and produces a Snapmaker U1 filament profile.
// ============================================================
const drop      = document.getElementById("drop");
const pickedEl  = document.getElementById("pickedPath");
const runBtn    = document.getElementById("run");
const status    = document.getElementById("status");
const result    = document.getElementById("result");
const logPanel  = document.getElementById("logPanel");
const logEl     = document.getElementById("log");
const fetchOnline = document.getElementById("fetchOnline");
const shareContribution = document.getElementById("shareContribution");

let chosenPath = null;
function setChosen(path) {
  chosenPath = path;
  pickedEl.textContent = path ?? "";
  runBtn.disabled = !path;
}

if (drop) {
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
}

if (runBtn) {
  runBtn.addEventListener("click", () => importPdfAndShow(
    chosenPath, fetchOnline.checked,
    shareContribution ? shareContribution.checked : true,
    result, status, logPanel, logEl, runBtn));
}

async function importPdfAndShow(path, online, share, resultEl, statusEl, logPanelEl, logElEl, runButton) {
  if (!path) return;
  const sl = slicerArgs();
  if (sl.slicer === "custom" && !sl.customDir) { statusEl.textContent = tr("slicer_pick_folder"); return; }
  // The single-PDF flow targets the printer chosen in the global selector when
  // one is set (so it generates the same 7 process); otherwise the backend
  // falls back to the Snapmaker U1.
  const p = currentPrinter();
  if (runButton) runButton.disabled = true;
  statusEl.textContent = tr("status_working");
  resultEl.style.display = "none";
  if (logPanelEl) logPanelEl.style.display = "none";
  try {
    const r = await invoke("import_pdf", { req: {
      pdfPath: path,
      fetchOnline: online,
      slicer: sl.slicer,
      customDir: sl.customDir,
      vendor: p ? p.vendor : null,
      model: p ? p.model : null,
      nozzle: p ? p.nozzle : null,
      allNozzles: p ? p.all : false,
      share: share,
    } });
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
    <h2 style="margin-top:0;">${escapeHtml(e.product_name ?? "Imported filament")} ${badge}</h2>
    <div class="sub">${escapeHtml(e.manufacturer ?? "Unknown manufacturer")} — ${escapeHtml(e.polymer ?? "Unknown polymer")}</div>
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
  statusEl.textContent = r.profile_path ? tr("open_slicer") : tr("status_done");
}

// ============================================================
// Update checker (v0.1.17) — manual button + silent launch check.
// The database is downloaded automatically when newer; a newer APP is only
// proposed (button opens the download page in the browser).
// ============================================================
const updateBtn    = document.getElementById("updateBtn");
const updateStatus = document.getElementById("updateStatus");
const updateBanner = document.getElementById("updateBanner");

function renderUpdate(st, manual) {
  if (st.error) {
    updateStatus.textContent = `${tr("update_error")}: ${st.error}`;
  } else if (st.dbDownloaded) {
    updateStatus.textContent = `${tr("update_db_done")} ${st.latestDbVersion}`;
    // A fresh database may have new materials — refresh the list if it loaded.
    if (filamentInitialised) loadFilaments(filamentSearch ? filamentSearch.value.trim() : "");
  } else if (st.upToDate) {
    updateStatus.textContent = manual ? tr("update_uptodate") : "";
  } else {
    updateStatus.textContent = "";
  }
  if (st.appUpdateAvailable && st.downloadUrl) {
    updateBanner.style.display = "block";
    updateBanner.innerHTML = "";
    const msg = document.createElement("div");
    msg.innerHTML = `<strong>${tr("update_app_available")}</strong> `
      + `${escapeHtml(st.latestAppVersion)} (${tr("update_current")} ${escapeHtml(st.currentAppVersion)}).`
      + (st.notes ? ` <span class="sub">${escapeHtml(st.notes)}</span>` : "");
    const row = document.createElement("div");
    row.className = "row";
    const dl = document.createElement("button");
    dl.textContent = tr("update_download_btn");
    dl.addEventListener("click", () => invoke("open_external", { url: st.downloadUrl }).catch(() => {}));
    const dismiss = document.createElement("button");
    dismiss.className = "secondary";
    dismiss.textContent = tr("update_dismiss");
    dismiss.addEventListener("click", () => { updateBanner.style.display = "none"; });
    row.appendChild(dl);
    row.appendChild(dismiss);
    updateBanner.appendChild(msg);
    updateBanner.appendChild(row);
  } else {
    updateBanner.style.display = "none";
  }
}

async function checkUpdates(manual) {
  if (!invoke) return;
  if (manual) updateStatus.textContent = tr("update_checking");
  try {
    const st = await invoke("check_updates");
    renderUpdate(st, manual);
  } catch (e) {
    updateStatus.textContent = `${tr("update_error")}: ${e}`;
  }
}

if (updateBtn) updateBtn.addEventListener("click", () => checkUpdates(true));

// ----- initial load -----
// Filament library is the default tab → populate it immediately.
if (invoke && filamentList) {
  filamentInitialised = true;
  loadFilaments("");
}
refreshGenEnabled();
// Silent background check at launch — only surfaces something if found.
checkUpdates(false);
