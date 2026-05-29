# Fiche produit — Optimisateur de filament et de profils d'impression by Maison Drabiec

> Document de travail destiné au propriétaire pour relecture. **Ne pas publier en l'état.**
> Tous les chiffres sont formulés en « jusqu'à » et reflètent l'état réel du logiciel (v0.4.0).

---

## 1. Titre produit

**Titre retenu (< 70 caractères)**

> **Optimisateur MD — 700+ filaments, profil en un clic**

**Variantes alternatives**

- *Optimisateur MD — base de 700+ filaments officiels, profils en un clic*
- *Optimisateur MD : choisissez votre filament, obtenez vos profils*
- *Optimisateur MD pour Snapmaker U1 — filament + process en un clic*

---

## 2. Accroche / sous-titre

**Choisissez votre imprimante, sélectionnez votre filament dans une base de 700+ matériaux officiels, et générez en un clic le profil filament + ses profils de process. Fini les heures de réglages au hasard.**

---

## 3. Description courte (vignette boutique, ≈ 300 caractères)

> Choisissez votre imprimante, piochez votre filament dans une **base de 700+ matériaux** issus des fiches officielles des fabricants (TDS / SDS / MSDS / RoHS), et générez **en un clic** le profil **filament** optimisé + ses profils de **process** prêts à imprimer pour Snapmaker_Orca. Pour un filament absent de la base, l'import d'un PDF fabricant reste possible. Réglages buse, plateau, séchage et débit fiables, sans être expert. Windows / macOS / Linux.

---

## 4. Description longue

### Choisissez votre filament, le profil est prêt en un clic

Chaque nouvelle bobine, c'est la même corvée : retrouver la fiche technique, déchiffrer un tableau de températures, lancer une calibration, rater une première impression… et recommencer. L'**Optimisateur MD** supprime cette étape. Vous **choisissez votre slicer** (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, ou un dossier personnalisé), vous **choisissez votre imprimante** en haut de la fenêtre (marque → modèle → buse, ou « toutes les buses »), vous **cherchez votre filament dans la bibliothèque** (700+ matériaux issus des fiches officielles des fabricants — avec un filtre par **marque** pour aller droit au but), et **un seul clic** génère ensemble le **profil filament** optimisé *et* ses **7 profils de process par type de projet**. Vous pouvez même **cocher plusieurs matériaux** d'un coup : l'application crée alors un profil filament par matière, plus un jeu commun des 7 profils de process.

Plus besoin d'être un expert en extrusion : la température de buse, la température de plateau, le séchage, la densité et le débit volumétrique sont renseignés à partir des **données officielles du fabricant** — pas de valeurs approximatives glanées ailleurs. Les profils générés sont écrits directement dans le dossier de préréglages du **slicer que vous avez choisi**, et ce choix est mémorisé d'une session à l'autre.

### Des données dignes de confiance, pas du copier-coller de forum

Le cœur du logiciel, c'est sa **base de données de plus de 700 matériaux** (709 références, 122 marques) construite à partir **des fiches officielles des fabricants** (TDS / SDS / MSDS / RoHS). La règle interne est stricte : quand une donnée fabricant existe (Polymaker, Prusament, Bambu Lab, eSUN, SUNLU, Eryone…), elle **prime** toujours sur toute autre source. Pour chaque matériau, vous disposez des températures recommandées, de la densité, des conditions de séchage, des codes couleur et de **liens directs vers les fiches officielles TDS / MSDS / RoHS** hébergées par le fabricant lui-même.

La base est **livrée prête à l'emploi** : l'application embarque un instantané complet utilisable **hors-ligne**, et le **vérificateur de mise à jour** se charge de la garder à jour — au premier lancement il télécharge la base courante depuis le serveur, et plus tard un « rechercher les mises à jour » récupère une base plus récente dès qu'elle est disponible.

Et la base **continue de s'enrichir grâce à la communauté**. Lors d'un import PDF, vous pouvez (c'est **coché par défaut, mais facultatif**) partager de façon **anonyme** les seuls **faits fabricant** de la fiche — marque, matière, type de base, fenêtre buse / plateau, densité, lien, date de révision — vers la file de modération de Maison Drabiec. Jamais le PDF, jamais vos chemins de fichiers, jamais de données personnelles ou liées à votre machine. Après relecture, les entrées validées rejoignent la base partagée (la donnée fabricant gardant toujours la priorité). C'est ainsi que la base s'étoffe au fil du temps, au bénéfice de tous.

> Important : le logiciel ne réhéberge **jamais** les PDF des fabricants. Il stocke les faits utiles (températures, densité…) et un lien profond vers le document d'origine.

### Un filament absent de la base ? L'import PDF prend le relais

Si votre filament n'est pas encore dans la base, l'Optimisateur le génère à partir de sa fiche : vous **déposez le PDF fabricant** (SDS ou TDS) et le profil est produit automatiquement (l'app peut aussi récupérer la TDS du fabricant en ligne), comme depuis la bibliothèque. C'est la **solution de secours** pour les nouveautés ; la grande majorité des matériaux courants se trouvent déjà dans la base.

Et l'analyseur ne se contente pas du tableau de paramètres. Il lit aussi la **note « éprouvette »** — ces conditions de test que beaucoup de fabricants indiquent (« toutes les éprouvettes sont imprimées à 210 °C, 80 mm/s, plateau 60 °C »). Ces valeurs **font autorité** et remplacent les moyennes du tableau, parce qu'elles décrivent exactement comment le fabricant a obtenu ses résultats mécaniques. Sécurité intégrée : la température de buse retenue reste toujours sous le seuil de décomposition annoncé.

### Les 7 profils PROCESS par type de projet, générés avec le filament

Le même clic qui crée le profil filament génère aussi ses **7 profils de process par type de projet**. Tout part du **sélecteur d'imprimante en haut de la fenêtre** : vous choisissez **marque → modèle → buse** (ou « toutes les buses ») parmi la **famille OrcaSlicer** (57 marques / 326 modèles : Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…), et l'app produit les 7 profils calibrés pour cette machine. En « toutes les buses », le jeu complet est multiplié par 4 — par exemple pour le **Snapmaker U1** : 7 types × 4 buses = 28 profils :

- **Prototype rapide** — couches épaisses, vitesse maximale, accélérations élevées
- **Objet du quotidien** — l'équilibre solidité / vitesse / finition
- **Figurine** — couches fines, cornering serré et accélérations/jerk bas pour effacer les artefacts verticaux et la résonance (VFA)
- **Vase** — mode spirale, paroi unique
- **Décoration** — repassage de surface, finition soignée
- **Jouet** — parois renforcées, remplissage généreux
- **Pièce mécanique** — parois multiples, remplissage dense

Chaque profil est calibré pour le **cornering** et la **résonance / VFA** (via les limites d'accélération et de jerk) et pour rester sous le plafond de débit de la machine. Les types fonctionnels à plus grande empreinte (Objet du quotidien, Jouet, Pièce mécanique) reçoivent en plus un **brim extérieur modéré** pour limiter le warping et le décollement du plateau ; les types esthétiques (Figurine, Vase, Décoration) et le Prototype rapide n'en ont pas. (Comme un jeu de process est partagé, l'adhérence se règle selon le **type de projet**, pas selon la matière.) Le réglage propre au filament (températures, débit, rétraction) reste sur le profil filament : un jeu de process partagé + un réglage matière, c'est tout ce qu'il faut.

Le profil filament cible l'**imprimante choisie** (`compatible_printers`) : il hérite du **parent réglé pour le Snapmaker U1** quand c'est cette machine, et du « **Generic &lt;polymère&gt;** » d'origine de la famille OrcaSlicer pour les autres. Le jeu de process partagé porte le cornering / résonance et les fonctions du fork.

### Les fonctions du fork activées d'office

Les profils générés activent les capacités du fork **Snapmaker_Orca / OptimusOrca** :

- **Économie de filament** — réduction des purges de tour de prime (−30 % par défaut sur une buse fraîchement utilisée), suppression des changements d'outil redondants et mise à l'échelle de l'extrusion selon la courbure. Sur le multicouleur, l'économie peut atteindre **jusqu'à −15 à −30 %** selon la pièce et le nombre de changements de couleur.
- **Coutures « scarf »** — un joint en biseau qui rend la couture en Z quasi invisible sur la plupart des matières (désactivé sur le TPU, qui ne se prête pas au lissage progressif).
- **Préparation au mélange de couleurs** — l'optimisation sûre (region-collapse) est activée ; les modes expérimentaux (dégradé, dithering…) restent désactivés par défaut pour ne pas affecter les impressions monochromes.

### Toujours à jour, léger, bilingue

La base de données reçoit des **mises à jour régulières** : l'application embarque un instantané fonctionnel hors-ligne et le **vérificateur de mise à jour** récupère une base plus récente dès qu'elle est publiée, sans réinstaller le logiciel. L'interface est **bilingue français / anglais**, le binaire est **léger** (Tauri, ≈ 10 Mo par OS) et tourne sur **Windows, macOS et Linux**.

---

## 5. Caractéristiques techniques

| Caractéristique | Détail |
|---|---|
| Type de produit | Application de bureau (utilitaire pour l'impression 3D FDM) |
| Systèmes d'exploitation | Windows · macOS (Apple Silicon & Intel) · Linux |
| Format de distribution | **Un ZIP unique** avec trois dossiers : Windows (**`.exe`**), macOS (**`.dmg`**), Linux (**`.AppImage`**) |
| Slicers cibles | **Sélecteur de slicer** : OrcaSlicer · Bambu Studio · Creality Print · SnapmakerOrca / OptimusOrca · dossier personnalisé (profils écrits dans le dossier de préréglages du slicer choisi, résolu selon l'OS ; choix mémorisé) |
| Entrées acceptées | **Sélection d'un ou plusieurs matériaux dans la base embarquée** (flux principal ; filtre par marque + recherche texte) ; en secours pour un filament absent : PDF de fiche fabricant (SDS / TDS), avec recherche optionnelle de la TDS en ligne |
| Sorties générées | Profil filament `.json` nommé « Marque Matière » + ses 7 profils de process `.json` (par type de projet, pour l'imprimante choisie ; × 4 en « toutes les buses »). En multi-sélection : un profil filament par matière + un seul jeu commun de process |
| Base de données | 700+ matériaux (709 réf., 122 marques) issus des fiches officielles fabricant (TDS / SDS / MSDS / RoHS) ; livrée embarquée, tenue à jour par le vérificateur |
| Imprimantes couvertes | Famille OrcaSlicer — 57 marques / 326 modèles (Creality, Bambu, Snapmaker, Anycubic, Prusa…) ; toutes leurs buses |
| Langues de l'interface | Français · Anglais (commutable, mémorisé) |
| Technologie | Tauri (binaire léger, ≈ 10 Mo par OS) |
| OCR (PDF scannés) | Pris en charge via Tesseract installé sur le système (les PDF « texte » fonctionnent sans) |
| Connexion Internet | Optionnelle (recherche de TDS en ligne et mises à jour de la base) ; le cœur fonctionne hors-ligne |
| Licence du logiciel | Propriétaire — usage strictement personnel et privé (voir LICENSE.md) |

> Configuration requise : un PC Windows 10/11, macOS récent ou Linux capable de faire tourner le slicer Snapmaker_Orca. Aucune dépendance lourde ; l'OCR n'est nécessaire que pour les fiches scannées (PDF image).

---

## 6. Ce qui est inclus

- L'application **Optimisateur MD** pour votre système (Windows / macOS / Linux).
- La **base de données de 700+ matériaux** livrée prête à l'emploi (températures, densité, séchage, couleurs, liens vers les fiches officielles), tenue à jour par le vérificateur.
- Le **flux en un clic** : sélecteur de slicer (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, dossier personnalisé), sélecteur d'imprimante (marque → modèle → buse), bibliothèque de filaments avec **filtre par marque** et **multi-sélection**, puis génération conjointe du **profil filament** + de ses **7 profils de process** par type de projet (jusqu'au jeu complet Snapmaker U1).
- L'**import PDF de secours** (PDF SDS/TDS, recherche TDS en ligne optionnelle) pour générer un filament encore absent de la base.
- La **contribution communautaire optionnelle** (anonyme, cochée par défaut, désactivable) qui partage les seuls faits fabricant d'une fiche importée pour enrichir la base partagée.
- Le **vérificateur de mise à jour** (téléchargement de la base au 1er lancement, base plus récente ensuite) et des réglages **supports / anti-warping** (brim extérieur sur les types de projet fonctionnels, détachement propre des supports).
- L'**activation des fonctions du fork** : économie de filament, coutures scarf, préparation au mélange de couleurs.
- L'interface **bilingue FR / EN**.
- Les **mises à jour régulières de la base de données**.

---

## 7. SEO

**Mots-clés**

`base de données filament` · `profil filament Snapmaker U1` · `optimiseur impression 3D` · `profils OrcaSlicer en un clic` · `réglages filament automatiques` · `bibliothèque de filaments 3D` · `économie de filament multicouleur` · `import fiche TDS SDS filament` · `profils process impression 3D` · `Snapmaker_Orca`

**Meta-description (≈ 155 caractères)**

> Base de 700+ filaments officiels : choisissez votre imprimante, votre matière, et générez en un clic les profils filament + process pour Snapmaker_Orca. Win/macOS/Linux.

---

## 8. FAQ

**Est-ce compatible avec mon imprimante / mon slicer ?**
Oui, pour toute la **famille OrcaSlicer** : OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca. Un **sélecteur de slicer** en haut de la fenêtre indique où écrire les profils (vous pouvez aussi pointer un dossier personnalisé), et le choix est mémorisé. Vient ensuite le **sélecteur d'imprimante** : vous choisissez marque, modèle et buse (ou « toutes les buses ») parmi **leurs imprimantes** (57 marques, 326 modèles), et les profils apparaissent dans les menus du slicer après génération. PrusaSlicer (format `.ini`) est prévu prochainement.

**D'où viennent les données ? Sont-elles fiables ?**
La base privilégie **les sites et fiches officiels des fabricants**. Quand une donnée fabricant existe, elle prime toujours sur toute autre source. Pour chaque matériau, vous avez accès aux **liens vers les fiches officielles** (TDS / MSDS / RoHS) hébergées par le fabricant. Le logiciel ne réhéberge jamais ces PDF.

**Qu'est-ce qui est partagé avec la « base communautaire », et qu'en est-il de ma vie privée ?**
Lors d'un import PDF, une case **« Partager cette fiche (anonyme) avec la base communautaire »** est **cochée par défaut**, mais reste entièrement **facultative** (décochez-la quand vous voulez ; elle ne bloque jamais l'import). Après un import réussi, seuls les **faits fabricant** sont envoyés à la file de modération de Maison Drabiec : marque, matière, type de base, fenêtre buse / plateau, densité, lien et date de révision. **Jamais** le PDF, **jamais** vos chemins de fichiers, **aucune** donnée personnelle ou liée à votre machine ; le serveur ne conserve qu'un identifiant d'IP haché pour limiter les abus. Après relecture, les entrées validées rejoignent la base partagée — la donnée fabricant gardant toujours la priorité.

**Comment retrouver vite mon filament dans la base ?**
Un menu déroulant **« Marque »** au-dessus de la recherche permet de filtrer par fabricant, en combinaison avec la recherche par texte (nom de produit ou famille). Vous pouvez aussi **cocher plusieurs matériaux** et générer, en un clic, un profil filament par matière plus un seul jeu commun des 7 profils de process.

**En quoi est-ce mieux qu'un profil générique « PLA » ?**
Un profil générique applique des moyennes. L'Optimisateur part de **votre filament précis** : il puise dans une **base de 700+ matériaux** bâtie sur les fiches officielles des fabricants (et, à défaut, lit le tableau de paramètres *et* la note « éprouvette » du PDF — les conditions de test qui font autorité) pour caler buse, plateau, vitesse et débit au plus juste, puis ajoute des process pensés pour votre type de projet.

**Mon PDF est un scan / une image, ça marche ?**
Oui — pour l'import de secours d'un filament absent de la base, à condition d'installer **Tesseract** sur votre système (OCR). Les PDF « texte » des fabricants — la grande majorité — fonctionnent sans rien installer.

**Les mises à jour de la base sont-elles incluses ?**
Oui. L'application embarque un instantané utilisable hors-ligne ; au premier lancement le vérificateur télécharge la base courante, puis « rechercher les mises à jour » récupère une base plus récente dès qu'elle est publiée, sans réinstaller le logiciel.

**Quelle économie de filament puis-je vraiment espérer ?**
Sur le **multicouleur**, l'économie peut atteindre **jusqu'à −15 à −30 %** selon la pièce et le nombre de changements de couleur (réduction des purges, fusion des changements d'outil inutiles). En monochrome, le gain vient surtout de la mise à l'échelle selon la courbure. Ce sont des ordres de grandeur, pas une garantie chiffrée.

**Politique de remboursement ?**
*(À compléter par le propriétaire selon les conditions de la boutique — p. ex. remboursement sous X jours si le logiciel ne se lance pas sur la configuration indiquée.)*

---

## 9. Idées de visuels / captures à préparer

- **Capture principale** : la **bibliothèque de filaments** avec une recherche en cours et un matériau sélectionné, prêt pour le clic unique.
- **Sélecteur d'imprimante global** : le menu en haut de la fenêtre, marque → modèle → buse (avec l'option « toutes les buses »).
- **Résultat du clic unique** : le profil filament *et* ses 7 profils de process générés ensemble et affichés (badge « Prêt »).
- **Vignettes des 7 types de projet** : une icône/rendu par intention (Prototype, Figurine, Vase, Décoration, Jouet, Pièce mécanique, Objet du quotidien).
- **Import de secours** : la fenêtre « PDF unique » avec une fiche déposée pour un filament absent de la base, et le profil obtenu.
- **Schéma « économie de filament »** : tour de purge avant/après, avec la mention « jusqu'à −15 à −30 % sur le multicouleur ».
- **Gros plan « couture scarf »** : comparaison d'une couture en Z classique vs scarf.
- **Bandeau de logos OS** : Windows / macOS / Linux + mention « bilingue FR/EN » et « léger (Tauri) ».
- **Visuel « 700+ matériaux »** : nuage de marques + chiffres clés (709 réf., 122 marques) avec la mention « sources officielles fabricant ».
