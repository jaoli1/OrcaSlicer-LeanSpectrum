// Bilingual translation table (English + French). To add another language,
// extend `messages` with a new key (e.g. `de`, `es`) carrying every string
// the UI uses. HTML elements opt into translation via `data-i18n="key"` or
// `data-i18n-placeholder="key"`.

const messages = {
  en: {
    app_title:           "Filament & Print-Profile Optimiser by Maison Drabiec",
    app_subtitle:        "Turn a manufacturer safety / technical data sheet into optimised Snapmaker_Orca filament + process profiles.",

    tab_single:          "Single PDF",
    tab_catalog:         "Vendor catalog",
    tab_database:        "Local database",
    tab_library:         "Process library",
    library_intro:       "Generate the shared set of process profiles by project type (Vase, Decoration, Figurine, Mechanical part…) for every nozzle (0.2 / 0.4 / 0.6 / 0.8) — 28 profiles tuned for cornering, resonance and the fork features (scarf seams, filament economy, color mixing). Filament-specific tuning stays on the filament profile.",
    library_generate:    "Generate process library",
    library_working:     "Generating 28 process profiles…",
    library_done:        "process profiles written to",
    library_fail:        "Could not generate the library",

    drop_hint:           "Drop a .pdf here, or click to pick a file.",
    fetch_online:        "Also look for the manufacturer's TDS online (recommended)",
    create_profile:      "Create filament profile",

    catalog_intro:       "Paste a vendor's certificates or downloads page URL. The app fetches the page, lists every SDS / TDS PDF it can identify, and lets you batch-import them in a single click.",
    catalog_placeholder: "https://<vendor>/path/to/certificates",
    catalog_discover:    "Discover PDFs",
    catalog_fetch:       "Also try to fetch related TDS for each downloaded PDF",
    catalog_select_all:  "Select all",
    catalog_select_none: "Select none",
    catalog_import_btn:  "Import selected",
    catalog_discovered:  "Discovered documents",

    database_intro:      "Browse PDFs already downloaded on this machine. Defaults to the corpus folder under your Downloads directory; change the path if your collection lives elsewhere.",
    database_path:       "Corpus folder",
    database_scan:       "Scan folder",
    database_pick:       "Import this PDF",
    database_empty:      "No PDFs found at the given path.",

    lang_label:          "Language",
    needs_review_badge:  "Needs review",
    ready_badge:         "Ready",
    log_heading:         "Log",
    field_density:       "Density (g/cm³)",
    field_glass:         "Glass transition (°C)",
    field_melt:          "Melting range (°C)",
    field_decomp:        "Decomposition (°C)",
    field_nozzle:        "Nozzle range (°C)",
    field_bed:           "Bed range (°C)",
    field_max_flow:      "Max volumetric speed (mm³/s)",
    field_profile_saved: "Profile written to",
    estimated_fields:    "Estimated fields",
    open_orca:           "Done — open Snapmaker_Orca to see the new filament.",
    status_done:         "Done.",
    status_working:      "Working…",
    status_no_docs:      "No documents selected.",
    status_discovering:  "Discovering…",

    err_picker:          "Picker error",
    err_discovery:       "Discovery failed",
    err_batch:           "Batch import failed",
    err_scan:            "Scan failed",

    batch_summary:       "Batch import — {ok} ok / {fail} failed",
    batch_done_status:   "Done — {ok} profile(s) created, {fail} error(s).",
  },

  fr: {
    app_title:           "Optimisateur de filament et de profils d'impression by Maison Drabiec",
    app_subtitle:        "Transforme une fiche fabricant (SDS/TDS) en profils filament + process Snapmaker_Orca optimisés.",

    tab_single:          "PDF unique",
    tab_catalog:         "Catalogue fabricant",
    tab_database:        "Base de données locale",
    tab_library:         "Bibliothèque process",
    library_intro:       "Génère le jeu partagé de profils de process par type de projet (Vase, Décoration, Figurine, Pièce mécanique…) pour chaque buse (0.2 / 0.4 / 0.6 / 0.8) — 28 profils calibrés pour le cornering, la résonance et les fonctions du fork (coutures scarf, économie de filament, mélange de couleurs). Le réglage propre au filament reste sur le profil filament.",
    library_generate:    "Générer la bibliothèque de process",
    library_working:     "Génération des 28 profils de process…",
    library_done:        "profils de process écrits dans",
    library_fail:        "Échec de la génération de la bibliothèque",

    drop_hint:           "Glisser un .pdf ici, ou cliquer pour choisir un fichier.",
    fetch_online:        "Chercher aussi la TDS du fabricant en ligne (recommandé)",
    create_profile:      "Créer le profil filament",

    catalog_intro:       "Coller l'URL de la page « certificats » ou « téléchargements » d'un fabricant. L'application récupère la page, liste toutes les FDS / TDS détectées, et permet de les importer en lot.",
    catalog_placeholder: "https://<fabricant>/chemin/vers/certificats",
    catalog_discover:    "Découvrir les PDFs",
    catalog_fetch:       "Chercher en plus la TDS associée pour chaque PDF téléchargé",
    catalog_select_all:  "Tout sélectionner",
    catalog_select_none: "Tout désélectionner",
    catalog_import_btn:  "Importer la sélection",
    catalog_discovered:  "Documents détectés",

    database_intro:      "Parcourir les PDFs déjà téléchargés sur cette machine. Par défaut le dossier corpus dans Téléchargements ; modifier le chemin si la collection est ailleurs.",
    database_path:       "Dossier corpus",
    database_scan:       "Scanner le dossier",
    database_pick:       "Importer ce PDF",
    database_empty:      "Aucun PDF trouvé dans ce chemin.",

    lang_label:          "Langue",
    needs_review_badge:  "À vérifier",
    ready_badge:         "Prêt",
    log_heading:         "Journal",
    field_density:       "Densité (g/cm³)",
    field_glass:         "Transition vitreuse (°C)",
    field_melt:          "Plage de fusion (°C)",
    field_decomp:        "Décomposition (°C)",
    field_nozzle:        "Plage buse (°C)",
    field_bed:           "Plage plateau (°C)",
    field_max_flow:      "Débit volumétrique max (mm³/s)",
    field_profile_saved: "Profil enregistré dans",
    estimated_fields:    "Champs estimés",
    open_orca:           "Terminé — ouvrir Snapmaker_Orca pour voir le nouveau filament.",
    status_done:         "Terminé.",
    status_working:      "En cours…",
    status_no_docs:      "Aucun document sélectionné.",
    status_discovering:  "Découverte en cours…",

    err_picker:          "Erreur sélecteur",
    err_discovery:       "Échec de la découverte",
    err_batch:           "Échec de l'import par lot",
    err_scan:            "Échec du scan",

    batch_summary:       "Import par lot — {ok} OK / {fail} échec(s)",
    batch_done_status:   "Terminé — {ok} profil(s) créé(s), {fail} erreur(s).",
  },
};

function getLang() {
  return localStorage.getItem('leanspectrum_lang')
    ?? (navigator.language?.toLowerCase().startsWith('fr') ? 'fr' : 'en');
}

function setLang(lang) {
  if (!messages[lang]) return;
  localStorage.setItem('leanspectrum_lang', lang);
  applyTranslations();
  document.documentElement.lang = lang;
  // Update language picker active state.
  for (const el of document.querySelectorAll('[data-lang-pick]')) {
    el.classList.toggle('active', el.dataset.langPick === lang);
  }
}

function t(key, vars) {
  const lang = getLang();
  let s = messages[lang]?.[key] ?? messages.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}

function applyTranslations() {
  for (const el of document.querySelectorAll('[data-i18n]')) {
    el.textContent = t(el.dataset.i18n);
  }
  for (const el of document.querySelectorAll('[data-i18n-placeholder]')) {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  }
  for (const el of document.querySelectorAll('[data-i18n-title]')) {
    el.title = t(el.dataset.i18nTitle);
  }
}

// Auto-init on page load.
document.addEventListener('DOMContentLoaded', () => {
  applyTranslations();
  document.documentElement.lang = getLang();
  for (const el of document.querySelectorAll('[data-lang-pick]')) {
    el.addEventListener('click', () => setLang(el.dataset.langPick));
    if (el.dataset.langPick === getLang()) el.classList.add('active');
  }
});

window.leanspectrumI18n = { t, setLang, getLang, applyTranslations };
