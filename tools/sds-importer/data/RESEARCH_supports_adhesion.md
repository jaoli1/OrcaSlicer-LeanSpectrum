# Réglages OrcaSlicer — Supports faciles à retirer, accroche plateau, anti-warping

Famille OrcaSlicer / Bambu Studio / SnapmakerOrca (schéma de config dérivé de BambuStudio).
Domaine = PROCESS. Tous les noms de clés ci-dessous sont vérifiés sur le wiki officiel OrcaSlicer (voir Sources).
Les valeurs en mm supposent une hauteur de couche de référence 0.2 mm et une buse 0.4 mm. Adapter au besoin.

Niveau de confiance des NOMS de clés : **élevé** (confirmés sur le wiki OrcaSlicer/GitHub).
Niveau de confiance des VALEURS chiffrées : **moyen** (recommandations de communauté/Prusa ; le wiki Orca documente rarement des valeurs numériques par défaut — voir notes).

---

## A. Détachement support → MODÈLE (retrait propre, marquage minimal)

| Clé OrcaSlicer (vérifiée) | Valeur recommandée | Justification (1 phrase) | Source |
|---|---|---|---|
| `support_top_z_distance` | 0.20–0.25 mm (≈ 50–75 % de la hauteur de couche, soit 1 couche complète) | C'est le jeu vertical entre le sommet du support et la face inférieure du modèle ; plus il est grand, plus le support se détache facilement, au prix d'un dessous plus rugueux. | Prusa KB (50–75 % de la hauteur de couche) ; wiki Orca « Support Advanced » |
| `support_bottom_z_distance` | = `support_top_z_distance` (ou légèrement > pour supports posés sur le modèle) | Jeu vertical sous le support là où il atterrit SUR le modèle ; même logique de détachabilité. | wiki Orca « Support Advanced » |
| `support_interface_spacing` | 0.2–0.5 mm (0 = interface pleine) | Espace entre les lignes d'interface en contact avec le modèle ; augmenter réduit la surface de contact et facilite le retrait (trop = dessous qui pend). | wiki Orca « Support Advanced » ; Prusa KB |
| `support_interface_pattern` | `rectilinear` (ou grille), avec angle croisé | Un motif rectiligne dont l'angle croise les lignes en dessous casse plus facilement à la séparation. | wiki Orca « Support Advanced » |
| `support_interface_top_layers` | 2–3 | Quelques couches d'interface denses donnent un beau dessous tout en restant détachables ; 0 colle moins mais donne un dessous médiocre. | wiki Orca « Support Advanced » ; Simplify3D (dense support layers) |
| `support_interface_bottom_layers` | 0–2 (0 si supports posés plateau uniquement) | Couches d'interface là où le support touche le modèle par le bas ; à réduire pour faciliter le détachement. | wiki Orca « Support Advanced » |
| `support_object_xy_distance` | 0.35–0.40 mm (≈ 70–100 % du diamètre buse) | Séparation horizontale support/modèle ; augmenter facilite le retrait sur parois verticales mais dégrade le maintien des surplombs. | wiki Orca « Support Advanced » ; Simplify3D « Horizontal Offset » |
| `support_object_first_layer_gap` | ≥ `support_object_xy_distance` | Séparation XY support/objet à la 1re couche uniquement, utile pour éviter la fusion en pied. | wiki Orca « Support Advanced » |
| `support_on_build_plate_only` | `true` quand la géométrie le permet | « Don't create support on model surface, only on build plate » : supprime tout contact support→modèle hors surplombs au-dessus du vide, donc zéro marquage sur les faces. | wiki Orca « Support » |
| `support_style` | `tree_organic` (arborescent) pour pièces complexes ; sinon `grid`/`snug` | Les supports arborescents (organic) touchent le modèle sur peu de points → retrait très propre ; mais accroche plateau plus faible (voir B). | wiki Orca « Support » |
| `support_type` | `normal(auto)` ou `tree(auto)` | Choix support classique vs arborescent ; arborescent = moins de marquage, classique = plus stable au plateau. | wiki Orca « Support » |
| `support_threshold_angle` | 30–55° (def. usuel ~30°) | Génère du support sous le seuil d'angle ; monter le seuil = moins de support superflu à retirer. | wiki Orca « Support » |

Compromis clé A : **augmenter `support_top_z_distance` + `support_interface_spacing` + `support_object_xy_distance` = retrait plus facile** ; les baisser = meilleur état de surface mais supports collés. Le point d'équilibre cité par la communauté Prusa : Z ≈ 0.25 mm, XY ≈ 75 % buse.

---

## B. Accroche support → PLATEAU (les supports ne se décollent pas en cours d'impression)

| Clé OrcaSlicer (vérifiée) | Valeur recommandée | Justification | Source |
|---|---|---|---|
| `raft_first_layer_density` | 90–100 % | « Density of the first raft or support layer » : 1re couche de support/raft quasi pleine = bien plus de surface accrochée au plateau. | wiki Orca « Support » |
| `raft_first_layer_expansion` | 1–3 mm | « Expand the first raft or support layer to improve bed plate adhesion » : élargit le pied du support → accroche renforcée. | wiki Orca « Support » |
| `raft_layers` | 0 (défaut) ; 1–2 si supports/petite base qui décollent | Couches de raft sous l'objet ; améliore l'accroche et réduit le warping mais consomme matière (raft à détacher ensuite via `raft_contact_distance`). | wiki Orca « Raft » |
| `raft_contact_distance` | 0.1–0.2 mm | Jeu Z entre le sommet du raft et la 1re couche du modèle ; règle la facilité de séparation du raft. | wiki Orca « Raft » |
| `support_base_pattern` | `rectilinear` | Motif de la base du support, stable et économe. | wiki Orca « Support Advanced » |
| Brim de support (`brim_width` actif avec supports) | 3–5 mm pour matériaux sensibles | Un brim ceinture aussi les piliers de support et empêche les fins supports de se détacher du plateau. | wiki Orca « Brim » ; cf. section C |

Note B : pour les supports **arborescents** (peu de contact au sol), augmenter `raft_first_layer_expansion` / `raft_first_layer_density` ou activer un brim est particulièrement utile, car leurs pieds fins décollent facilement.

---

## C. Anti-warping (gauchissement)

| Clé OrcaSlicer (vérifiée) | Valeur recommandée | Justification | Source |
|---|---|---|---|
| `brim_type` | `outer_only` (général) ; `brim_ears` pour pièces avec coins/petits points | Brim extérieur = barrière anti-décollement des bords ; « ears » concentre l'adhérence aux coins sujets au curling. | wiki Orca « Brim » |
| `brim_width` | PLA : 0–5 mm · PETG : 3–5 mm · **ABS/ASA/PC/Nylon : 5–10 mm** | Plus la largeur est grande, plus l'ancrage des bords résiste au retrait thermique ; **fortement dépendant du matériau** (gros pour ABS/ASA/PC, faible/nul pour PLA). | wiki Orca « Brim » ; Prusa KB Skirt&Brim |
| `brim_object_gap` | 0–0.1 mm (0 pour adhérence max) | « Gap between the innermost brim line and the object » : 0 maximise l'accroche anti-warp ; augmenter facilite le retrait du brim mais réduit l'effet. | wiki Orca « Brim » |
| `draft_shield` | `disabled` (PLA / caisson) · `enabled` ou « limited » pour **ABS/ASA/PC sans caisson** | Bouclier (jupe haute) qui retient la chaleur autour de la pièce et bloque les courants d'air → réduit le warp ; surtout utile sans enceinte. **Dépend du matériau.** | wiki Orca « Skirt » ; discussion Orca #1809 |
| `single_loop_draft_shield` | `true` (économie) / `false` (bouclier plus robuste) | Limite le bouclier à une seule boucle au-delà de la 1re couche ; `false` = bouclier plus efficace contre le warp mais plus de matière. | wiki Orca « Skirt » |
| `skirt_loops` | 2 (ou plus si draft shield) | Amorce l'extrusion et, combiné à `skirt_height`, sert de bouclier thermique. | wiki Orca « Skirt » |
| Vitesse 1re couche `initial_layer_speed` | 20–30 mm/s (imprimantes lentes) · 40–50 mm/s (rapides) | Une 1re couche lente écrase mieux le filament sur le plateau → adhérence accrue et bords moins enclins à curler. | wiki Orca « Initial layer speed » |
| Hauteur 1re couche `initial_layer_print_height` | 0.20–0.25 mm (≤ 65 % du Ø buse) | Une 1re couche plus épaisse colle mieux et absorbe les défauts de planéité. | wiki Orca « Layer Height » |
| Refroidissement 1res couches (fan) | PLA : modéré · **ABS/ASA/PC : fan OFF ou ≤ 20 %, surtout 1res couches** | Couper/limiter le ventilateur évite le refroidissement brutal qui provoque le retrait et le warp des matériaux techniques. **Dépend fortement du matériau.** | recherche communautaire (ABS/ASA 95–105 °C plateau, fan ≤20 %) |
| Temp. plateau 1re couche | PLA ~55–60 °C · PETG ~70–80 °C · **ABS/ASA ~95–105 °C · PC ~100–110 °C** | Un plateau chaud maintient la base au-dessus de la Tg le temps de l'impression et limite le warp ; **dépend du matériau**. | recherche communautaire ; Prusa KB |
| `elefant_foot_compensation` | 0.1–0.2 mm (orthographe Orca : « elefant ») | N'agit pas sur le warp mais corrige l'évasement de pied causé par une 1re couche écrasée/plateau chaud — utile quand on pousse l'adhérence. | wiki Orca « Precision » |

**Règle matériau (warping) :** PLA peu sujet (brim faible, pas de bouclier, fan OK) ; PETG modéré ; **ABS / ASA / PC / Nylon très sujets** → brim large 5–10 mm, plateau très chaud, ventilateur quasi coupé, draft shield ou caisson fermé obligatoire.

---

## D. Anti-décollement de la PIÈCE elle-même

| Clé OrcaSlicer (vérifiée) | Valeur recommandée | Justification | Source |
|---|---|---|---|
| `brim_type` + `brim_width` | voir C (ex. `outer_only`, 3–5 mm PLA / 5–10 mm ABS-ASA) | Le brim augmente la surface d'accroche au plateau et empêche le soulèvement de la pièce ; principal levier anti-décollement pour petites empreintes. | wiki Orca « Brim » ; Prusa KB |
| `brim_object_gap` | 0 mm (adhérence maximale) | Brim au contact direct de la pièce = ancrage maximal (au prix d'un retrait du brim un peu plus dur). | wiki Orca « Brim » |
| `initial_layer_speed` | 20–30 mm/s | 1re couche lente → meilleure adhérence de toute la pièce, pas seulement des bords. | wiki Orca « Initial layer speed » |
| `initial_layer_print_height` | 0.20–0.25 mm | 1re couche épaisse = plus de matière écrasée contre le plateau. | wiki Orca « Layer Height » |
| Débit 1re couche (initial layer flow) | ~ +0 à +5 % si besoin | Un léger sur-débit 1re couche augmente l'écrasement et l'accroche (attention à l'elephant foot). | recherche communautaire ; cf. `elefant_foot_compensation` |
| Temp. buse 1re couche | +5 à +10 °C vs reste de l'impression | Une 1re couche plus chaude adhère mieux au plateau. | wiki Orca « Material Temperatures » ; recherche communautaire |
| Temp. plateau 1re couche | selon matériau (voir C) | L'adhérence de la pièce dépend d'abord d'un plateau à la bonne température. | Prusa KB ; recherche communautaire |
| `raft_layers` | 1–2 (dernier recours) | Si brim + réglages 1re couche insuffisants (pièce gauchissante ou plateau imparfait), un raft garantit l'adhérence. | wiki Orca « Raft » |

---

## Synthèse — les 6–8 réglages à régler en priorité

1. `support_top_z_distance` ≈ 0.20–0.25 mm — détachabilité support/modèle.
2. `support_interface_spacing` 0.2–0.5 mm + `support_interface_pattern` = rectilinear — retrait propre.
3. `support_object_xy_distance` ≈ 0.35–0.40 mm — séparation latérale.
4. `support_on_build_plate_only` = true (si géométrie OK) — zéro marquage des faces.
5. `raft_first_layer_density` 90–100 % + `raft_first_layer_expansion` 1–3 mm — supports accrochés au plateau.
6. `brim_type` = outer_only/brim_ears, `brim_width` 5–10 mm (ABS/ASA) ou 3–5 mm (PLA/PETG), `brim_object_gap` 0 — anti-warp + anti-décollement.
7. `draft_shield` = enabled pour ABS/ASA/PC sans caisson — anti-warp matériaux techniques.
8. `initial_layer_speed` 20–30 mm/s + `initial_layer_print_height` 0.20–0.25 mm — adhérence pièce.

> Conflit à arbitrer : A (gros jeux = retrait facile) vs surface du dessous ; D/C (gap brim 0 = accroche max) vs facilité de retrait du brim. Privilégier l'accroche (B/C/D) pendant l'impression, et la détachabilité (A) pour l'après-impression — ce sont des zones distinctes, donc compatibles.

---

## Réserves / vérifications
- Les **noms de clés** ci-dessus sont confirmés sur le wiki OrcaSlicer (pages Support / Support Advanced / Brim / Skirt / Raft / Initial layer speed / Layer Height / Precision).
- Le wiki officiel OrcaSlicer **ne publie pas de valeurs numériques par défaut** pour la plupart de ces clés ; les valeurs chiffrées proviennent de la KB Prusa, de Simplify3D et de la pratique communautaire et doivent être validées par des impressions de test sur la Snapmaker U1.
- Orthographe à noter : la clé est bien `elefant_foot_compensation` (avec « f »), pas « elephant ».
- Je n'ai **pas** trouvé de clé nommée explicitement « support brim » distincte : le brim des supports est géré par les clés brim générales (`brim_type`/`brim_width`) appliquées aux supports, plus `raft_first_layer_expansion`/`raft_first_layer_density` pour le pied de support.

## Sources
- OrcaSlicer Wiki — Support Advanced (jeux Z, interface, XY) : https://github.com/OrcaSlicer/OrcaSlicer/wiki/support_settings_advanced
- OrcaSlicer Wiki — Support (type/style/on build plate only/raft first layer) : https://github.com/OrcaSlicer/OrcaSlicer/wiki/support_settings_support
- OrcaSlicer Wiki — Brim (brim_type, brim_width, brim_object_gap, brim_ears) : https://github.com/OrcaSlicer/OrcaSlicer/wiki/others_settings_brim
- OrcaSlicer Wiki — Skirt & Draft shield : https://github.com/OrcaSlicer/OrcaSlicer/wiki/others_settings_skirt
- OrcaSlicer Wiki — Raft : https://www.orcaslicer.com/wiki/print_settings/support/support_settings_raft
- OrcaSlicer Wiki — Initial layer speed : https://github.com/OrcaSlicer/OrcaSlicer/wiki/speed_settings_initial_layer_speed
- OrcaSlicer Wiki — Layer Height (1re couche) : https://github.com/OrcaSlicer/OrcaSlicer/wiki/quality_settings_layer_height
- OrcaSlicer Wiki — Material Temperatures : https://github.com/OrcaSlicer/OrcaSlicer/wiki/material_temperatures
- OrcaSlicer Wiki — Precision / Elephant foot compensation : https://www.orcaslicer.com/wiki/print_settings/quality/quality_settings_precision
- OrcaSlicer Discussion #1809 — Draft wall / réduction warping ABS/ASA : https://github.com/SoftFever/OrcaSlicer/discussions/1809
- Prusa Knowledge Base — Support material (contact Z 50–75 % hauteur couche, XY, spacing) : https://help.prusa3d.com/article/support-material_1698
- Prusa Knowledge Base — Skirt and Brim : https://help.prusa3d.com/article/skirt-and-brim_133969
- Simplify3D — Adding and Modifying Support Structures (horizontal/vertical offset, dense support) : https://www.simplify3d.com/resources/articles/adding-and-modifying-support-structures/
