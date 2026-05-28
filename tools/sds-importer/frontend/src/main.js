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
  if (genFilamentProcess)  genFilamentProcess.disabled = !(p && selectedFilamentId != null);
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
const filamentList       = document.getElementById("filamentList");
const filamentCount      = document.getElementById("filamentCount");
const genFilamentProcess = document.getElementById("genFilamentProcess");
const filamentStatus     = document.getElementById("filamentStatus");
const filamentResult     = document.getElementById("filamentResult");
let selectedFilamentId = null;
let searchTimer = null;

async function loadFilaments(query) {
  if (!invoke || !filamentList) return;
  try {
    const rows = await invoke("list_filaments", { query: query || null });
    renderFilaments(rows);
  } catch (e) {
    filamentList.innerHTML = "";
    filamentCount.textContent = "";
    filamentResult.style.display = "block";
    filamentResult.textContent = String(e);
  }
}

function renderFilaments(rows) {
  filamentCount.textContent = rows.length ? String(rows.length) : tr("filament_none");
  selectedFilamentId = null;
  refreshGenEnabled();
  filamentList.innerHTML = rows.map(r => {
    const fam    = escapeHtml(r.base_type || "?") + (r.filled_type ? " " + escapeHtml(r.filled_type) : "");
    const params = r.has_params ? `<span class="badge params" title="manufacturer temps">°C</span>` : "";
    const colors = r.colors ? `<span class="sub">${r.colors} ◐</span>` : "";
    const dens   = r.density ? `<span class="sub">${r.density} g/cm³</span>` : "";
    return `<li data-id="${r.id}">
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
    li.addEventListener("click", () => {
      selectedFilamentId = parseInt(li.dataset.id, 10);
      for (const x of filamentList.querySelectorAll("li")) x.classList.toggle("selected", x === li);
      refreshGenEnabled();
    });
  }
}

if (filamentSearch) {
  filamentSearch.addEventListener("input", () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => loadFilaments(filamentSearch.value.trim()), 200);
  });
}

if (genFilamentProcess) {
  genFilamentProcess.addEventListener("click", async () => {
    const p = currentPrinter();
    if (!p)                       { filamentStatus.textContent = tr("filament_pick_printer"); return; }
    if (selectedFilamentId == null) { filamentStatus.textContent = tr("filament_pick_one"); return; }
    genFilamentProcess.disabled = true;
    filamentStatus.textContent = tr("filament_working");
    filamentResult.style.display = "none";
    try {
      const r = await invoke("generate_filament_and_process", {
        materialId: selectedFilamentId,
        vendor: p.vendor,
        model: p.model,
        nozzle: p.nozzle,
        allNozzles: p.all,
      });
      const printers = (r.printers || []).join(", ");
      filamentResult.style.display = "block";
      filamentResult.innerHTML =
        `<h2 style="margin-top:0;">${escapeHtml(tr("filament_result_for", { printer: printers }))}</h2>`
        + `<div class="field"><span>${escapeHtml(tr("filament_result_filament"))}</span><span><code>${escapeHtml(r.filamentName)}</code></span></div>`
        + `<div class="field"><span>${escapeHtml(tr("filament_result_process"))}</span><span><strong>${r.processCount}</strong></span></div>`
        + `<div class="sub" style="margin-top:8px;"><code>${escapeHtml(r.processDir)}</code></div>`;
      filamentStatus.textContent = tr("open_orca");
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
// chosen in the global selector, or the full Snapmaker U1 set (28 profiles).
// ============================================================
const genForPrinter = document.getElementById("genForPrinter");
const genLibraryBtn = document.getElementById("genLibraryBtn");
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
    genForPrinter.disabled = true;
    libraryStatus.textContent = tr("library_working");
    libraryResult.style.display = "none";
    try {
      const r = await invoke("generate_process_library_for", {
        vendor: p.vendor, model: p.model, nozzle: p.nozzle, allNozzles: p.all,
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

if (genLibraryBtn) {
  genLibraryBtn.addEventListener("click", async () => {
    genLibraryBtn.disabled = true;
    libraryStatus.textContent = tr("library_working");
    libraryResult.style.display = "none";
    try {
      renderLibraryResult(await invoke("generate_process_library"));
    } catch (e) {
      libraryResult.style.display = "block";
      libraryResult.textContent = `${tr("library_fail")}: ${e}`;
    } finally {
      libraryStatus.textContent = "";
      genLibraryBtn.disabled = false;
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
  runBtn.addEventListener("click", () => importPdfAndShow(chosenPath, fetchOnline.checked, result, status, logPanel, logEl, runBtn));
}

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
  statusEl.textContent = r.profile_path ? tr("open_orca") : tr("status_done");
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
