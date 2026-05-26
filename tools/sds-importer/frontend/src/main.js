import { invoke } from "@tauri-apps/api/core";

// ----- tabs -----
document.querySelectorAll(".tab").forEach(t => {
  t.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach(x => x.classList.toggle("active", x === t));
    document.querySelectorAll(".tab-panel").forEach(p => {
      p.classList.toggle("active", p.id === `tab-${t.dataset.tab}`);
    });
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
  } catch (e) { status.textContent = `Picker error: ${e}`; }
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

runBtn.addEventListener("click", async () => {
  if (!chosenPath) return;
  runBtn.disabled = true;
  status.textContent = "Working…";
  result.style.display = "none";
  logPanel.style.display = "none";
  try {
    const r = await invoke("import_pdf", {
      req: { pdfPath: chosenPath, fetchOnline: fetchOnline.checked }
    });
    renderResult(r);
  } catch (e) {
    status.textContent = `Error: ${e}`;
  } finally {
    runBtn.disabled = false;
  }
});

function field(label, value) {
  if (value === null || value === undefined || value === "") return "";
  return `<div class="field"><span>${label}</span><span>${value}</span></div>`;
}
function renderResult(r) {
  const e = r.extracted;
  const badge = e.needs_review
    ? `<span class="badge review">Needs review</span>`
    : `<span class="badge ok">Ready</span>`;
  result.innerHTML = `
    <h2 style="margin-top:0;">${e.product_name ?? "Imported filament"} ${badge}</h2>
    <div class="sub">${e.manufacturer ?? "Unknown manufacturer"} — ${e.polymer ?? "Unknown polymer"}</div>
    ${field("Density (g/cm³)", e.density_g_cm3)}
    ${field("Glass transition (°C)", e.glass_transition_c)}
    ${field("Melting range (°C)", e.melt_temp_min_c && e.melt_temp_max_c ? `${e.melt_temp_min_c} – ${e.melt_temp_max_c}` : null)}
    ${field("Decomposition (°C)", e.decomposition_c)}
    ${field("Nozzle range (°C)", e.nozzle_temp_min_c && e.nozzle_temp_max_c ? `${e.nozzle_temp_min_c} – ${e.nozzle_temp_max_c}` : null)}
    ${field("Bed range (°C)", e.bed_temp_min_c && e.bed_temp_max_c ? `${e.bed_temp_min_c} – ${e.bed_temp_max_c}` : null)}
    ${field("Profile written to", r.profile_path ?? "(not saved)")}
    ${e.estimated_fields?.length ? `<div class="sub">Estimated fields: ${e.estimated_fields.join(", ")}</div>` : ""}
  `;
  result.style.display = "block";
  if (r.log?.length) {
    logEl.innerHTML = r.log.map(l => `<div>${escapeHtml(l)}</div>`).join("");
    logPanel.style.display = "block";
  }
  status.textContent = r.profile_path ? "Done — open Snapmaker_Orca to see the new filament." : "Done.";
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
  catalogStatus.textContent = "Discovering…";
  catalogPanel.style.display = "none";
  batchResult.style.display = "none";
  try {
    const r = await invoke("crawl_catalog", { url });
    catalogEntries = r.entries;
    renderCatalog(r);
  } catch (e) {
    catalogStatus.textContent = `Discovery failed: ${e}`;
  } finally {
    crawlBtn.disabled = false;
  }
});

function badgeFor(docType) {
  const t = (docType || "Unknown").toLowerCase();
  return `<span class="badge ${t}">${docType ?? "Unknown"}</span>`;
}

function renderCatalog(r) {
  catalogPanel.style.display = "block";
  catalogStatus.textContent = `${r.entries.length} document(s) discovered`;
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
    catalogStatus.textContent = "No documents selected.";
    return;
  }
  batchImportBtn.disabled = true;
  catalogStatus.textContent = `Importing ${selected.length} document(s)…`;
  batchProgress.style.display = "block";
  batchProgressBar.style.width = "10%";
  try {
    const r = await invoke("import_from_urls", {
      req: { urls: selected, fetchOnline: catalogFetch.checked }
    });
    batchProgressBar.style.width = "100%";
    renderBatchResult(r);
  } catch (e) {
    catalogStatus.textContent = `Batch import failed: ${e}`;
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
    <h2 style="margin-top:0;">Batch import — ${ok} ok / ${fail} failed</h2>
    <div class="log">${lines.map(l => `<div>${escapeHtml(l)}</div>`).join("")}</div>
  `;
  batchResult.style.display = "block";
  catalogStatus.textContent = `Done — ${ok} profile(s) created, ${fail} error(s).`;
}

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
}
