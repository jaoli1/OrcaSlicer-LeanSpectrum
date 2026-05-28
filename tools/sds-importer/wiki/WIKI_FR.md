# Optimisateur de filament et de profils d'impression by Maison Drabiec — Manuel utilisateur

> **Version du logiciel :** 0.1.17 · **Langue de ce document :** Français ([English version](WIKI_EN.md))
>
> Application de bureau pour transformer une fiche fabricant (PDF SDS/TDS) ou une URL catalogue en profils **filament** et **process** optimisés pour le slicer **OptimusOrca / Snapmaker_Orca**.

---

## Table des matières

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Premier lancement & interface](#3-premier-lancement--interface)
4. [Mode 1 — Importer une fiche PDF](#4-mode-1--importer-une-fiche-pdf)
5. [Mode 2 — Catalogue fabricant](#5-mode-2--catalogue-fabricant)
6. [Mode 3 — Base locale](#6-mode-3--base-locale)
7. [La bibliothèque de profils PROCESS](#7-la-bibliothèque-de-profils-process)
8. [Mises à jour](#8-mises-à-jour)
9. [Utiliser les profils dans OptimusOrca / Snapmaker_Orca](#9-utiliser-les-profils-dans-optimusorca--snapmaker_orca)
10. [Dépannage / FAQ](#10-dépannage--faq)
11. [Mentions](#11-mentions)

---

## 1. Introduction

L'**Optimisateur de filament et de profils d'impression by Maison Drabiec** (en abrégé : *Optimisateur MD*) est un petit utilitaire de bureau qui supprime l'étape fastidieuse du réglage manuel d'un nouveau filament.

Vous lui fournissez l'une de ces trois entrées :

- une **fiche fabricant** au format PDF (SDS — fiche de sécurité, ou TDS — fiche technique) ;
- l'**URL d'une page catalogue / certificats** d'une marque ;
- un **dossier local** de PDF déjà téléchargés.

En retour, le logiciel écrit pour vous :

- un **profil filament** `.json` (températures de buse et de plateau, densité, débit volumétrique, fournisseur…) ;
- une **bibliothèque de profils process** `.json` par type de projet et par diamètre de buse.

Ces profils apparaissent ensuite directement dans les menus du slicer **OptimusOrca / Snapmaker_Orca**.

### Pour qui ?

- Les **possesseurs de Snapmaker U1** : les profils sont calibrés pour cette machine et ses quatre buses (0.2 / 0.4 / 0.6 / 0.8 mm).
- Plus largement, les **imprimeurs FDM** qui utilisent le slicer Snapmaker_Orca / OptimusOrca et veulent partir de réglages fiables, issus des données officielles du fabricant, plutôt que de valeurs glanées sur des forums.

> **Note**
> Le logiciel ne réhéberge jamais les PDF des fabricants. Il extrait les faits utiles (températures, densité, séchage…) et, le cas échéant, conserve un lien vers le document d'origine.

---

## 2. Installation

L'application est distribuée sous forme de binaire léger (technologie Tauri) pour chaque système.

### Windows (`.exe` / `.msi`)

1. Téléchargez le fichier d'installation Windows.
2. Lancez l'installateur et suivez les étapes.
3. Démarrez l'application depuis le menu Démarrer.

> **Note**
> Le format `.msi` est recommandé pour Windows ; un installateur `.exe` (NSIS) est également proposé.

### macOS (`.dmg` — application non signée)

1. Ouvrez le fichier `.dmg` téléchargé.
2. Glissez l'application dans le dossier **Applications**.
3. **Au premier lancement**, ne double-cliquez pas : faites un **clic droit sur l'application > Ouvrir**, puis confirmez dans la boîte de dialogue.

> **Important**
> L'application n'est **pas signée** par un certificat de développeur Apple payant. macOS affichera donc un avertissement « application non vérifiée » au premier démarrage. Le contournement par **clic droit > Ouvrir** est normal et n'est nécessaire qu'une seule fois. Voir aussi la [FAQ](#10-dépannage--faq).

### Linux (`.AppImage`)

1. Téléchargez le fichier `.AppImage`.
2. Rendez-le exécutable :

   ```bash
   chmod +x Optimisateur-MD-*.AppImage
   ```

3. Lancez-le par double-clic ou en ligne de commande :

   ```bash
   ./Optimisateur-MD-*.AppImage
   ```

> **Note**
> Des paquets `.deb` et `.rpm` peuvent également être proposés pour les distributions correspondantes.

### Où l'application écrit-elle les profils ?

L'Optimisateur écrit ses profils dans le **dossier utilisateur de Snapmaker_Orca**, là où le slicer va les lire :

- les **profils filament** vont dans le sous-dossier `filament/` de votre profil utilisateur Snapmaker_Orca ;
- les **profils process** vont dans le sous-dossier `process/`.

> **Important**
> Le dossier utilisateur de Snapmaker_Orca doit déjà exister. Si ce n'est pas le cas, **ouvrez OptimusOrca / Snapmaker_Orca au moins une fois** pour qu'il crée son dossier de profils, puis relancez la génération.

L'application tient aussi un **journal** dans son propre dossier de données système (utile pour le support en cas de problème).

---

## 3. Premier lancement & interface

Au premier démarrage, l'interface s'ouvre dans la langue du système (français ou anglais), puis mémorise votre choix.

### Sélecteur de langue (FR / EN)

En haut à droite, deux boutons **EN** et **FR** permettent de basculer l'interface. Le choix est conservé d'une session à l'autre.

### Bouton « Rechercher une mise à jour »

Sous l'en-tête se trouve un bouton **« Rechercher une mise à jour »**, accompagné d'une zone de statut. Voir la section [Mises à jour](#8-mises-à-jour).

### Les 4 onglets

L'interface s'organise en quatre onglets :

| Onglet | Rôle |
|---|---|
| **PDF unique** | Importer une seule fiche fabricant PDF (glisser-déposer ou sélection de fichier). |
| **Catalogue fabricant** | Coller l'URL d'une page « certificats / téléchargements » et importer en lot les PDF détectés. |
| **Base de données locale** | Scanner un dossier de PDF déjà présents sur la machine. |
| **Bibliothèque process** | Générer en un clic le jeu de 28 profils de process par type de projet. |

Les trois premiers onglets produisent un **profil filament** ; le quatrième produit les **profils process**.

---

## 4. Mode 1 — Importer une fiche PDF

C'est le mode le plus direct, dans l'onglet **PDF unique**.

### Étapes

1. **Déposez un PDF** dans la zone prévue (« Glisser un .pdf ici, ou cliquer pour choisir un fichier »), ou cliquez pour ouvrir le sélecteur de fichiers.
2. Laissez cochée (ou non) l'option **« Chercher aussi la TDS du fabricant en ligne (recommandé) »**. Si votre PDF est une simple fiche de sécurité (SDS) sans données d'impression, cette option permet d'aller chercher la fiche technique (TDS) correspondante sur le site du fabricant pour compléter le profil.
3. Cliquez sur **« Créer le profil filament »**.
4. L'application affiche le résultat (champs extraits, éventuels badges) et un **journal** détaillant ce qui a été fait.

### Ce qui est extrait

L'analyseur ne se contente pas de recopier un tableau : il lit la fiche et en tire les paramètres réellement utiles :

- la **température de buse** (plage min–max et valeur retenue) ;
- la **température de plateau** ;
- les **conditions de séchage** et la **densité** ;
- la **vitesse d'impression** ;
- la **note « éprouvette »** lorsqu'elle est présente.

> **Note — la note « éprouvette » fait autorité**
> Beaucoup de fiches indiquent les conditions exactes dans lesquelles les barreaux de test mécanique ont été imprimés (par exemple : *« toutes les éprouvettes sont imprimées à 210 °C, 80 mm/s, plateau 60 °C »*). Quand cette note existe, ses valeurs **remplacent** les moyennes du tableau de paramètres, car elles décrivent précisément la façon dont le fabricant a obtenu ses résultats.

Les champs qui ont dû être **estimés** (faute de valeur dans la fiche) sont signalés, et le profil peut alors porter un badge **« À vérifier »** pour vous inviter à contrôler avant une impression critique. Un profil complet porte le badge **« Prêt »**.

---

## 5. Mode 2 — Catalogue fabricant

Dans l'onglet **Catalogue fabricant**, vous traitez plusieurs fiches d'un coup à partir d'une page web.

### Étapes

1. **Collez l'URL** de la page « certificats » ou « téléchargements » d'un fabricant dans le champ prévu.
2. Cliquez sur **« Découvrir les PDFs »**. L'application récupère la page et **liste tous les PDF SDS / TDS** qu'elle parvient à identifier, avec un badge de type (SDS / TDS / inconnu).
3. Cochez les documents qui vous intéressent. Les boutons **« Tout sélectionner »** / **« Tout désélectionner »** facilitent la sélection.
4. Vous pouvez activer **« Chercher en plus la TDS associée pour chaque PDF téléchargé »** pour compléter chaque fiche.
5. Cliquez sur **« Importer la sélection »**. Une barre de progression suit l'avancement, et un récapitulatif indique le nombre de profils créés et d'éventuelles erreurs.

> **Note**
> L'import par lot est robuste : si un PDF de la sélection pose problème, il est signalé en erreur mais n'interrompt pas le traitement des autres.

---

## 6. Mode 3 — Base locale

Dans l'onglet **Base de données locale**, vous travaillez à partir de PDF déjà présents sur votre disque.

### Étapes

1. Le champ de chemin est pré-rempli avec un **dossier par défaut** situé sous votre dossier *Téléchargements* (un dossier de corpus). Modifiez-le si votre collection se trouve ailleurs.
2. Cliquez sur **« Scanner le dossier »**.
3. L'application liste les PDF trouvés, **regroupés par marque** (sous-dossier).
4. Cliquez sur un PDF pour l'importer et générer son profil filament.

> **Note**
> Le scan explore un niveau d'imbrication : `dossier/marque/*.pdf` et `dossier/marque/produit/*.pdf`. Les arborescences plus profondes ne sont pas parcourues.

---

## 7. La bibliothèque de profils PROCESS

L'onglet **Bibliothèque process** génère un **jeu partagé de 28 profils de process**, organisés par **type de projet** et déclinés pour les **4 diamètres de buse** du Snapmaker U1.

Le principe est « **un jeu de process partagé + un réglage matière** » : le réglage propre au filament (températures, débit, rétraction) reste sur le **profil filament**, tandis que les profils process portent la géométrie d'impression (couches, parois, remplissage, vitesses, accélérations, finition).

### Les 7 types de projet × 4 buses = 28 profils

Chaque type de projet est généré pour les buses **0.2 / 0.4 / 0.6 / 0.8 mm**. La hauteur de couche s'adapte automatiquement au diamètre de la buse (calée entre 25 % et 75 % du diamètre).

Valeurs de référence (au diamètre **0.4 mm**) :

| Type de projet | Couche (0.4) | Parois | Remplissage | Motif | Vitesse paroi ext. | Accél. / Jerk | Spécificité |
|---|---|---|---|---|---|---|---|
| **Prototype rapide** | 0.28 mm | 1 | 8 % | grille | 150 mm/s | 10000 / 12 | Couches épaisses, vitesse et accélérations maximales |
| **Objet du quotidien** | 0.20 mm | 3 | 15 % | grille | 120 mm/s | 6000 / 9 | Équilibre solidité / vitesse / finition |
| **Figurine** | 0.12 mm | 3 | 15 % | gyroïde | 50 mm/s | 2000 / 5 | Couches fines, **cornering serré et accél./jerk bas** pour effacer les artefacts verticaux et la résonance (VFA) |
| **Vase** | 0.20 mm | 1 | 0 % | gyroïde | 60 mm/s | 4000 / 7 | **Mode spirale**, paroi unique, pièce creuse |
| **Décoration** | 0.16 mm | 2 | 10 % | éclair (lightning) | 80 mm/s | 4000 / 7 | **Repassage** de la surface supérieure, finition soignée |
| **Jouet** | 0.20 mm | 4 | 30 % | grille | 100 mm/s | 6000 / 9 | Parois renforcées, remplissage généreux |
| **Pièce mécanique** | 0.24 mm | 5 | 45 % | grille | 60 mm/s | 4000 / 7 | Parois multiples, remplissage dense |

### Ce que chaque profil optimise

- **Hauteur de couche** — adaptée à la buse, dans la fenêtre imprimable 25–75 % du diamètre ; première couche un peu plus épaisse pour l'adhérence.
- **Parois (wall loops)** et **coques haut / bas** — selon la solidité visée.
- **Remplissage** — densité et motif (grille, gyroïde, éclair) selon l'intention.
- **Vitesses** — paroi extérieure, paroi intérieure, remplissage et surface supérieure.
- **Cornering & résonance / VFA** — via les limites d'**accélération** et de **jerk** : bas et serré pour la Figurine (moins d'artefacts verticaux fins), haut et rapide pour le Prototype.
- **Mode vase (spirale)** pour le type Vase ; **repassage** (ironing) pour la Décoration.
- **Coutures « scarf »** — activées sur les intentions de qualité de surface (Objet du quotidien, Figurine, Décoration, Jouet, Pièce mécanique) pour des coutures en Z quasi invisibles ; désactivées sur le Vase (paroi continue unique) et le Prototype.
- **Économie de filament** — activée d'office (réduction des purges à −30 %, suppression des changements d'outil redondants, mise à l'échelle de l'extrusion selon la courbure, forçage du mode relatif M83).
- **Préparation au mélange de couleurs** — l'optimisation sûre *region-collapse* est activée ; les modes expérimentaux restent désactivés par défaut.

### Bouton « Générer la bibliothèque de process »

1. Ouvrez l'onglet **Bibliothèque process**.
2. Cliquez sur **« Générer la bibliothèque de process »**.
3. L'application écrit les **28 profils** dans le dossier `process/` de votre profil utilisateur Snapmaker_Orca et affiche le dossier de destination.

> **Important**
> Si le dossier utilisateur de Snapmaker_Orca n'est pas trouvé, ouvrez OptimusOrca une fois pour qu'il crée votre dossier de profils, puis relancez la génération.

---

## 8. Mises à jour

L'Optimisateur sépare clairement **deux choses** : la **base de données** des filaments (de simples données) et l'**application** elle-même (le binaire).

### Vérification manuelle et automatique

- Une **vérification automatique** a lieu **au lancement** de l'application.
- Vous pouvez aussi lancer une vérification à tout moment avec le bouton **« Rechercher une mise à jour »**.

### Ce qui se passe selon le cas

- **Une nouvelle base de données est publiée** → elle est **téléchargée automatiquement** dans le dossier de données de l'application (c'est juste de la donnée), et sa version est mémorisée. Un message confirme la mise à jour de la base.
- **Une nouvelle version de l'application est publiée** → elle est **proposée au téléchargement** (l'interface affiche la version disponible et un bouton **« Télécharger »** qui ouvre la page de téléchargement). La distribution se fait sous forme d'archive : **le binaire n'est jamais remplacé silencieusement**.
- **Tout est à jour** → un message « Vous avez déjà la dernière version » s'affiche.

> **Note**
> Si la vérification échoue (hors-ligne, serveur indisponible), l'application l'indique sans prétendre qu'une mise à jour est disponible.

---

## 9. Utiliser les profils dans OptimusOrca / Snapmaker_Orca

Une fois les profils générés, ils sont écrits dans le dossier utilisateur du slicer et apparaissent dans ses menus.

### Où apparaissent-ils ?

- Le **profil filament** apparaît dans le **menu Filament** du slicer, dans la section des profils utilisateur.
- Les **profils process** apparaissent dans le **menu Process**, sous le nom du type de projet et de la buse (par exemple `Figurine @U1 (0.4 nozzle)`).

### Comment les sélectionner ?

1. Ouvrez **OptimusOrca / Snapmaker_Orca** (relancez-le s'il était déjà ouvert pendant la génération — voir la [FAQ](#10-dépannage--faq)).
2. Sélectionnez votre **imprimante Snapmaker U1** au bon diamètre de buse.
3. Dans le menu **Filament**, choisissez le profil filament généré pour votre bobine.
4. Dans le menu **Process**, choisissez le profil correspondant à votre **type de projet** et à votre **buse**.

> **Note**
> Le profil process et le profil filament se complètent : le filament porte les températures et le débit, le process porte la géométrie d'impression et les fonctions du fork. Choisissez les deux pour un résultat optimal.

---

## 10. Dépannage / FAQ

### Un profil n'apparaît pas dans le menu du slicer

Le slicer ne lit son dossier de profils utilisateur **qu'au démarrage**. Si vous veniez de générer un profil pendant que le slicer était ouvert, **fermez puis relancez OptimusOrca / Snapmaker_Orca** : le profil apparaîtra alors dans le menu Filament ou Process.

### macOS affiche « application non vérifiée » / « non vérifiée par Apple »

C'est normal : l'application n'est pas signée par un certificat Apple payant. Au lieu de double-cliquer, faites un **clic droit sur l'application > Ouvrir**, puis confirmez. Cette manipulation n'est nécessaire qu'au premier lancement.

### « Dossier utilisateur de Snapmaker_Orca non trouvé »

La génération a besoin que le slicer ait déjà créé son dossier de profils. **Ouvrez OptimusOrca / Snapmaker_Orca au moins une fois**, puis relancez la génération (profil filament ou bibliothèque process).

### La base de données n'est pas trouvée / la mise à jour échoue

La mise à jour de la base nécessite une **connexion Internet**. Si la vérification échoue (hors-ligne, serveur momentanément indisponible), l'application le signale sans bloquer le reste de ses fonctions. Réessayez plus tard avec **« Rechercher une mise à jour »**. Le cœur de l'application (extraction, génération de profils) fonctionne **hors-ligne**.

### Mon PDF est un scan / une image

L'extraction directe du texte fonctionne pour la grande majorité des fiches fabricant (PDF « texte »). Pour un PDF purement image (scan), une reconnaissance de caractères (OCR) est nécessaire ; elle requiert l'installation de **Tesseract** sur le système.

### L'import en ligne d'une TDS ne trouve rien

L'option « chercher la TDS en ligne » dépend des liens présents sur la page du fabricant. Si aucune fiche technique complémentaire n'est trouvée, l'application le mentionne dans le journal et conserve les données déjà extraites du PDF.

---

## 11. Mentions

- **Slicer cible.** Les profils sont destinés à **OptimusOrca / Snapmaker_Orca**, un slicer libre. Le slicer est distribué sous licence **AGPL-3.0-or-later**.
- **Données filaments.** Les données proviennent des **sites officiels des fabricants**. Quand une donnée fabricant existe, elle prime sur toute autre source. L'application **ne réhéberge pas** les PDF des fabricants : elle stocke les faits utiles et, le cas échéant, un lien vers le document d'origine.
- **Marque.** « Optimisateur de filament et de profils d'impression by Maison Drabiec » et le monogramme MD sont la marque de Maison Drabiec.

> Les standards publics (GHS, ISO 11014-1) sont des références publiques ; aucun schéma ou contenu de profil propriétaire n'est reproduit.
