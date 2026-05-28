# Fiche produit — Optimisateur de filament et de profils d'impression by Maison Drabiec

> Document de travail destiné au propriétaire pour relecture. **Ne pas publier en l'état.**
> Tous les chiffres sont formulés en « jusqu'à » et reflètent l'état réel du logiciel (v0.1.19).

---

## 1. Titre produit

**Titre retenu (< 70 caractères)**

> **Optimisateur MD — profils filament & impression prêts à l'emploi**

**Variantes alternatives**

- *Optimisateur de filament MD — réglages 3D fiables sans tâtonner*
- *Optimisateur MD : votre fiche fabricant devient un profil optimisé*
- *Optimisateur MD pour Snapmaker U1 — filament + process en un clic*

---

## 2. Accroche / sous-titre

**Déposez la fiche de votre filament, récupérez un profil d'impression optimisé. Fini les heures de réglages au hasard.**

---

## 3. Description courte (vignette boutique, ≈ 300 caractères)

> Transformez n'importe quelle fiche fabricant (PDF SDS/TDS) ou URL catalogue en un profil **filament** optimisé + des profils de **process** prêts à imprimer pour Snapmaker_Orca. Base de 700+ matériaux issus des sites officiels des fabricants. Réglages buse, plateau, séchage et débit fiables, sans être expert. Windows / macOS / Linux.

---

## 4. Description longue

### Arrêtez de deviner vos réglages

Chaque nouvelle bobine, c'est la même corvée : retrouver la fiche technique, déchiffrer un tableau de températures, lancer une calibration, rater une première impression… et recommencer. L'**Optimisateur MD** supprime cette étape. Vous déposez la fiche fabricant de votre filament (le PDF SDS ou TDS), ou vous collez l'URL de la page « certificats » d'une marque, et le logiciel produit **automatiquement** un profil prêt à imprimer.

Plus besoin d'être un expert en extrusion : la température de buse, la température de plateau, le séchage, la densité et le débit volumétrique sont renseignés à partir des **données officielles du fabricant** — pas de valeurs approximatives glanées ailleurs.

### Des données dignes de confiance, pas du copier-coller de forum

La force du logiciel, c'est sa **base de données de plus de 700 matériaux** (709 références, 122 marques) construite pour privilégier **les sites et fiches officiels des fabricants**. La règle interne est stricte : quand une donnée fabricant existe (Polymaker, Prusament, Bambu Lab, eSUN, SUNLU, Eryone…), elle **prime** toujours sur toute autre source. Pour chaque matériau, vous disposez des températures recommandées, de la densité, des conditions de séchage, des codes couleur et de **liens directs vers les fiches officielles TDS / MSDS / RoHS** hébergées par le fabricant lui-même.

> Important : le logiciel ne réhéberge **jamais** les PDF des fabricants. Il stocke les faits utiles (températures, densité…) et un lien profond vers le document d'origine.

### Une extraction qui lit vraiment la fiche

Quand vous importez un PDF, l'analyseur ne se contente pas du tableau de paramètres. Il lit aussi la **note « éprouvette »** — ces conditions de test que beaucoup de fabricants indiquent (« toutes les éprouvettes sont imprimées à 210 °C, 80 mm/s, plateau 60 °C »). Ces valeurs **font autorité** et remplacent les moyennes du tableau, parce qu'elles décrivent exactement comment le fabricant a obtenu ses résultats mécaniques. Sécurité intégrée : la température de buse retenue reste toujours sous le seuil de décomposition annoncé.

### Une bibliothèque de profils PROCESS par type de projet

Au-delà du profil filament, l'Optimisateur génère des **profils de process par type de projet** pour **n'importe quelle imprimante** prise en charge par OrcaSlicer (Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…) : vous choisissez **marque → modèle → buse** et l'app produit les 7 profils calibrés pour cette machine. Un bouton génère aussi, en un clic, le jeu complet **Snapmaker U1** (7 types × 4 buses = 28 profils) :

- **Prototype rapide** — couches épaisses, vitesse maximale, accélérations élevées
- **Objet du quotidien** — l'équilibre solidité / vitesse / finition
- **Figurine** — couches fines, cornering serré et accélérations/jerk bas pour effacer les artefacts verticaux et la résonance (VFA)
- **Vase** — mode spirale, paroi unique
- **Décoration** — repassage de surface, finition soignée
- **Jouet** — parois renforcées, remplissage généreux
- **Pièce mécanique** — parois multiples, remplissage dense

Chaque profil est calibré pour le **cornering** et la **résonance / VFA** (via les limites d'accélération et de jerk) et pour rester sous le plafond de débit du U1. Le réglage propre au filament (températures, débit, rétraction) reste sur le profil filament : un jeu de process partagé + un réglage matière, c'est tout ce qu'il faut.

### Les fonctions du fork activées d'office

Les profils générés activent les capacités du fork **Snapmaker_Orca / OptimusOrca** :

- **Économie de filament** — réduction des purges de tour de prime (−30 % par défaut sur une buse fraîchement utilisée), suppression des changements d'outil redondants et mise à l'échelle de l'extrusion selon la courbure. Sur le multicouleur, l'économie peut atteindre **jusqu'à −15 à −30 %** selon la pièce et le nombre de changements de couleur.
- **Coutures « scarf »** — un joint en biseau qui rend la couture en Z quasi invisible sur la plupart des matières (désactivé sur le TPU, qui ne se prête pas au lissage progressif).
- **Préparation au mélange de couleurs** — l'optimisation sûre (region-collapse) est activée ; les modes expérimentaux (dégradé, dithering…) restent désactivés par défaut pour ne pas affecter les impressions monochromes.

### Toujours à jour, léger, bilingue

La base de données reçoit des **mises à jour régulières** : l'application embarque un instantané fonctionnel hors-ligne et se rafraîchit automatiquement quand une nouvelle version des données est disponible. L'interface est **bilingue français / anglais**, le binaire est **léger** (Tauri, ≈ 10 Mo par OS) et tourne sur **Windows, macOS et Linux**.

---

## 5. Caractéristiques techniques

| Caractéristique | Détail |
|---|---|
| Type de produit | Application de bureau (utilitaire pour l'impression 3D FDM) |
| Systèmes d'exploitation | Windows · macOS (Apple Silicon & Intel) · Linux |
| Format de distribution | **Un ZIP unique** avec trois dossiers : Windows (**`.exe`**), macOS (**`.dmg`**), Linux (**`.AppImage`**) |
| Slicers cibles | **Famille OrcaSlicer** : OrcaSlicer · Creality Print · Bambu Studio · SnapmakerOrca / OptimusOrca (profils écrits dans le dossier utilisateur du slicer) |
| Entrées acceptées | PDF de fiche fabricant (SDS / TDS), URL de page catalogue/certificats, dossier local de PDF |
| Sorties générées | Profil filament `.json` + profils de process `.json` (par type de projet, pour l'imprimante choisie) |
| Base de données | 700+ matériaux (709 réf., 122 marques), priorité aux données officielles fabricant |
| Imprimantes couvertes | Famille OrcaSlicer — 57 marques / 326 modèles (Creality, Bambu, Snapmaker, Anycubic, Prusa…) ; toutes leurs buses |
| Langues de l'interface | Français · Anglais (commutable, mémorisé) |
| Technologie | Tauri (binaire léger, ≈ 10 Mo par OS) |
| OCR (PDF scannés) | Pris en charge via Tesseract installé sur le système (les PDF « texte » fonctionnent sans) |
| Connexion Internet | Optionnelle (recherche de TDF en ligne et mises à jour de la base) ; le cœur fonctionne hors-ligne |
| Licence du logiciel | AGPL-3.0-or-later |

> Configuration requise : un PC Windows 10/11, macOS récent ou Linux capable de faire tourner le slicer Snapmaker_Orca. Aucune dépendance lourde ; l'OCR n'est nécessaire que pour les fiches scannées (PDF image).

---

## 6. Ce qui est inclus

- L'application **Optimisateur MD** pour votre système (Windows / macOS / Linux).
- La **base de données de 700+ matériaux** (températures, densité, séchage, couleurs, liens vers les fiches officielles).
- Le **générateur de profil filament** depuis un PDF SDS/TDS, une URL catalogue ou un dossier local.
- La **génération de profils de process** par type de projet pour n'importe quelle imprimante de la famille OrcaSlicer (+ le jeu complet Snapmaker U1).
- Le **vérificateur de mise à jour** (base de données auto + nouvelle version proposée) et des réglages **supports / anti-warping** adaptés à la matière.
- L'**activation des fonctions du fork** : économie de filament, coutures scarf, préparation au mélange de couleurs.
- L'**import par lot** depuis une page « certificats » de fabricant.
- L'interface **bilingue FR / EN**.
- Les **mises à jour régulières de la base de données**.

---

## 7. SEO

**Mots-clés**

`profil filament Snapmaker U1` · `optimiseur impression 3D` · `profils OrcaSlicer` · `réglages filament automatiques` · `économie de filament multicouleur` · `import fiche TDS SDS filament` · `profils process impression 3D` · `Snapmaker_Orca`

**Meta-description (≈ 155 caractères)**

> Générez des profils filament et process optimisés pour Snapmaker_Orca depuis n'importe quelle fiche fabricant. 700+ matériaux officiels. Windows/macOS/Linux.

---

## 8. FAQ

**Est-ce compatible avec mon imprimante / mon slicer ?**
Oui, pour toute la **famille OrcaSlicer** : OrcaSlicer, Creality Print, Bambu Studio, SnapmakerOrca / OptimusOrca. L'app couvre **leurs imprimantes** (57 marques, 326 modèles) — vous choisissez marque, modèle et buse, et les profils apparaissent dans les menus du slicer après génération. PrusaSlicer (format `.ini`) est prévu prochainement.

**D'où viennent les données ? Sont-elles fiables ?**
La base privilégie **les sites et fiches officiels des fabricants**. Quand une donnée fabricant existe, elle prime toujours sur toute autre source. Pour chaque matériau, vous avez accès aux **liens vers les fiches officielles** (TDS / MSDS / RoHS) hébergées par le fabricant. Le logiciel ne réhéberge jamais ces PDF.

**En quoi est-ce mieux qu'un profil générique « PLA » ?**
Un profil générique applique des moyennes. L'Optimisateur part de la fiche **de votre filament précis** : il lit le tableau de paramètres *et* la note « éprouvette » (les conditions de test qui font autorité) pour caler buse, plateau, vitesse et débit au plus juste, puis ajoute des process pensés pour votre type de projet.

**Mon PDF est un scan / une image, ça marche ?**
Oui, à condition d'installer **Tesseract** sur votre système (OCR). Les PDF « texte » des fabricants — la grande majorité — fonctionnent sans rien installer.

**Les mises à jour de la base sont-elles incluses ?**
Oui. L'application embarque un instantané utilisable hors-ligne et se met à jour automatiquement quand de nouvelles données sont publiées, sans réinstaller le logiciel.

**Quelle économie de filament puis-je vraiment espérer ?**
Sur le **multicouleur**, l'économie peut atteindre **jusqu'à −15 à −30 %** selon la pièce et le nombre de changements de couleur (réduction des purges, fusion des changements d'outil inutiles). En monochrome, le gain vient surtout de la mise à l'échelle selon la courbure. Ce sont des ordres de grandeur, pas une garantie chiffrée.

**Politique de remboursement ?**
*(À compléter par le propriétaire selon les conditions de la boutique — p. ex. remboursement sous X jours si le logiciel ne se lance pas sur la configuration indiquée.)*

---

## 9. Idées de visuels / captures à préparer

- **Capture principale** : la fenêtre « PDF unique » avec une fiche déposée et le profil généré affiché (badge « Prêt »).
- **Avant / après** : un tableau de fiche fabricant brut, flèche, le profil filament prêt dans le menu du slicer.
- **Onglet « Bibliothèque process »** : la grille des 7 types de projet × 4 buses (28 profils) avec le bouton « Générer ».
- **Vignettes des 7 types de projet** : une icône/rendu par intention (Prototype, Figurine, Vase, Décoration, Jouet, Pièce mécanique, Objet du quotidien).
- **Schéma « économie de filament »** : tour de purge avant/après, avec la mention « jusqu'à −15 à −30 % sur le multicouleur ».
- **Gros plan « couture scarf »** : comparaison d'une couture en Z classique vs scarf.
- **Onglet « Catalogue fabricant »** : une page de certificats analysée, liste des PDF détectés avec badges SDS/TDS et import par lot.
- **Bandeau de logos OS** : Windows / macOS / Linux + mention « bilingue FR/EN » et « léger (Tauri) ».
- **Visuel « 700+ matériaux »** : nuage de marques + chiffres clés (709 réf., 122 marques) avec la mention « sources officielles fabricant ».
