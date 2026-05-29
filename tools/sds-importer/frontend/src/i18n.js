// Bilingual translation table (English + French). To add another language,
// extend `messages` with a new key (e.g. `de`, `es`) carrying every string
// the UI uses. HTML elements opt into translation via `data-i18n="key"` or
// `data-i18n-placeholder="key"`.

const messages = {
  en: {
    app_title:           "Filament & Print-Profile Optimiser by Maison Drabiec",
    app_subtitle:        "Turn a manufacturer safety / technical data sheet into optimised Snapmaker_Orca filament + process profiles.",

    tab_filament:        "Filament library",
    tab_process:         "Process library",
    tab_single:          "Single PDF",

    slicer_label:        "Slicer",
    slicer_custom:       "Other…",
    slicer_custom_ph:    "Destination folder (absolute path)",
    slicer_browse:       "Browse…",
    slicer_pick_folder:  "Choose a destination folder for the custom slicer option.",

    printer_label:       "Printer",
    printer_all:         "All nozzles",
    printer_hint:        "Pick brand → model → nozzle (or all nozzles). Shared by both libraries.",

    filament_intro:      "Search the filament database (built from manufacturers' own TDS / SDS sheets), select one OR MORE materials, choose your printer above, then generate one filament profile per material PLUS the shared 7 process profiles in one click.",
    filament_search_ph:  "Search a filament (brand, name, PLA / PETG…)",
    filament_generate:   "Generate filament + process",
    filament_working:    "Generating filament + process…",
    filament_none:       "No filament matches.",
    filament_db_missing: "The filament database is not installed yet — click \"Check for updates\".",
    filament_pick_printer: "Pick a printer above first.",
    filament_pick_one:   "Select at least one filament in the list.",
    filament_selected:   "{n} selected",
    filament_result_for: "Generated for {printer}",
    filament_result_filament: "Filament profiles",
    filament_result_process:  "process profiles",
    process_intro:       "Generate the 7 project-type process profiles for the printer selected above (use the \"all nozzles\" option for the full set). Filament-specific tuning stays on the filament profile.",
    process_generate_printer: "Generate process for the selected printer",

    library_intro:       "Generate ready-to-use process profiles by project type (Prototype, Everyday, Figurine, Vase, Decoration, Toy, Mechanical part) for YOUR printer — pick a brand, model and nozzle below. Each profile is tuned for cornering, resonance and the fork features (scarf seams, filament economy, color mixing); filament-specific tuning stays on the filament profile.",
    library_vendor:      "— brand —",
    library_model:       "— model —",
    library_nozzle:      "— nozzle —",
    library_generate_printer: "Generate for this printer",
    library_working:     "Generating process profiles…",
    library_done:        "process profiles written to",
    library_fail:        "Could not generate the library",
    update_check_btn:    "Check for updates",
    update_checking:     "Checking…",
    update_uptodate:     "You already have the latest version.",
    update_db_done:      "Filament database updated to",
    update_app_available: "A new version is available:",
    update_current:      "you have",
    update_download_btn: "Download",
    update_dismiss:      "Later",
    update_error:        "Update check failed",

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
    open_slicer:         "Done — open your slicer to see the new filament.",
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

    tab_filament:        "Bibliothèque Filament",
    tab_process:         "Bibliothèque process",
    tab_single:          "PDF unique",

    slicer_label:        "Slicer",
    slicer_custom:       "Autre…",
    slicer_custom_ph:    "Dossier de destination (chemin absolu)",
    slicer_browse:       "Parcourir…",
    slicer_pick_folder:  "Choisissez un dossier de destination pour l'option « Autre… ».",

    printer_label:       "Imprimante",
    printer_all:         "Toutes les buses",
    printer_hint:        "Choisissez marque → modèle → buse (ou toutes les buses). Utilisé par les deux bibliothèques.",

    filament_intro:      "Recherchez dans la base de filaments (construite depuis les fiches TDS / SDS officielles des fabricants), sélectionnez un OU PLUSIEURS matériaux, choisissez votre imprimante ci-dessus, puis générez un profil filament par matériau ET les 7 profils process partagés en un clic.",
    filament_search_ph:  "Rechercher un filament (marque, nom, PLA / PETG…)",
    filament_generate:   "Générer filament + process",
    filament_working:    "Génération filament + process…",
    filament_none:       "Aucun filament ne correspond.",
    filament_db_missing: "La base de filaments n'est pas encore installée — cliquez sur « Rechercher une mise à jour ».",
    filament_pick_printer: "Choisissez d'abord une imprimante ci-dessus.",
    filament_pick_one:   "Sélectionnez au moins un filament dans la liste.",
    filament_selected:   "{n} sélectionné(s)",
    filament_result_for: "Généré pour {printer}",
    filament_result_filament: "Profils filament",
    filament_result_process:  "profils process",
    process_intro:       "Génère les 7 profils process par type de projet pour l'imprimante choisie ci-dessus (utilisez l'option « toutes les buses » pour le jeu complet). Le réglage propre au filament reste sur le profil filament.",
    process_generate_printer: "Générer les process pour l'imprimante choisie",

    library_intro:       "Génère des profils process prêts à l'emploi par type de projet (Prototype, Quotidien, Figurine, Vase, Décoration, Jouet, Pièce mécanique) pour TON imprimante — choisis ci-dessous la marque, le modèle et la buse. Chaque profil est calibré pour le cornering, la résonance et les fonctions du fork (coutures scarf, économie de filament, mélange de couleurs) ; le réglage propre au filament reste sur le profil filament.",
    library_vendor:      "— marque —",
    library_model:       "— modèle —",
    library_nozzle:      "— buse —",
    library_generate_printer: "Générer pour cette imprimante",
    library_working:     "Génération des profils process…",
    library_done:        "profils de process écrits dans",
    library_fail:        "Échec de la génération de la bibliothèque",
    update_check_btn:    "Rechercher une mise à jour",
    update_checking:     "Vérification…",
    update_uptodate:     "Vous avez déjà la dernière version.",
    update_db_done:      "Base de filaments mise à jour vers",
    update_app_available: "Une nouvelle version est disponible :",
    update_current:      "vous avez",
    update_download_btn: "Télécharger",
    update_dismiss:      "Plus tard",
    update_error:        "Échec de la vérification des mises à jour",

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
    open_slicer:         "Terminé — ouvrir votre slicer pour voir le nouveau filament.",
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
