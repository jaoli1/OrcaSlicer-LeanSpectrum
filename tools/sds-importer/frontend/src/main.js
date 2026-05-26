import { invoke } from "@tauri-apps/api/core";

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
  } catch (e) {
    status.textContent = `Picker error: ${e}`;
  }
});

["dragenter", "dragover"].forEach(ev => drop.addEventListener(ev, e => {
  e.preventDefault();
  drop.classList.add("hover");
}));
["dragleave", "drop"].forEach(ev => drop.addEventListener(ev, e => {
  e.preventDefault();
  drop.classList.remove("hover");
}));
drop.addEventListener("drop", e => {
  // Tauri 2 surfaces dropped files via a window event; the simplest
  // path is to let the user click to pick. We log the drop for now.
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

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}[c]));
}
