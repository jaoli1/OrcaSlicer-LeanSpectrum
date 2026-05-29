# Optimisateur de filament et de profils d'impression by Maison Drabiec — Manuel utilisateur

> **Version du logiciel :** 0.4.0 · **Langue de ce document :** Français ([English version](WIKI_EN.md))
>
> Application de bureau qui s'appuie sur une **base de données de filaments** (construite à partir des fiches officielles des fabricants) pour générer, en un clic, des profils **filament** et **process** optimisés pour le slicer **OptimusOrca / Snapmaker_Orca**.

---

## Table des matières

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Premier lancement & interface](#3-premier-lancement--interface)
4. [La Bibliothèque Filament (mode principal)](#4-la-bibliothèque-filament-mode-principal)
5. [Le sélecteur d'imprimante global](#5-le-sélecteur-dimprimante-global)
6. [PDF unique (mode de secours)](#6-pdf-unique-mode-de-secours)
7. [La bibliothèque de profils PROCESS](#7-la-bibliothèque-de-profils-process)
8. [Mises à jour](#8-mises-à-jour)
9. [Utiliser les profils dans OptimusOrca / Snapmaker_Orca](#9-utiliser-les-profils-dans-optimusorca--snapmaker_orca)
10. [Dépannage / FAQ](#10-dépannage--faq)
11. [Mentions](#11-mentions)

---

## 1. Introduction

L'**Optimisateur de filament et de profils d'impression by Maison Drabiec** (en abrégé : *Optimisateur MD*) est un utilitaire de bureau qui supprime l'étape fastidieuse du réglage manuel d'un nouveau filament.

Au cœur du logiciel se trouve désormais une **base de données de filaments**. Construite à partir des **fiches officielles des fabricants** (TDS — fiche technique, SDS / MSDS — fiches de sécurité, RoHS), elle réunit **709 matériaux** de **122 marques**, avec leurs températures, leurs couleurs et les liens vers les documents d'origine. Elle est **embarquée hors-ligne** (un instantané est posé dans l'application à la première utilisation), puis **rafraîchie depuis le serveur Maison Drabiec** quand vous cliquez sur « Rechercher une mise à jour ».

Le principe d'utilisation est simple :

1. vous choisissez votre **slicer** dans le sélecteur en haut de la fenêtre (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, ou un dossier personnalisé) — c'est là que les profils seront écrits ;
2. vous choisissez votre **imprimante** (marque → modèle → buse) ;
3. vous **recherchez un matériau** dans la base (par marque, nom ou famille : PLA, PETG…), avec la possibilité d'en **cocher plusieurs** ;
4. l'application génère, **en un seul clic**, le profil filament **et** les sept profils process par type de projet, calibrés pour cette imprimante.

En retour, le logiciel écrit pour vous :

- un **profil filament** `.json` (températures de buse et de plateau, densité, débit volumétrique, fournisseur…), rendu compatible avec l'imprimante choisie ;
- une **bibliothèque de profils process** `.json` par type de projet et par diamètre de buse.

Ces profils apparaissent ensuite directement dans les menus du slicer **OptimusOrca / Snapmaker_Orca**.

Pour un filament qui ne figure pas encore dans la base, un mode de secours **« PDF unique »** permet d'importer une fiche fabricant (SDS / TDS) au format PDF.

### Pour qui ?

- Les **possesseurs de Snapmaker U1** : les profils sont calibrés pour cette machine et ses quatre buses (0.2 / 0.4 / 0.6 / 0.8 mm).
- Plus largement, les **imprimeurs FDM** qui utilisent le slicer Snapmaker_Orca / OptimusOrca (famille OrcaSlicer) et veulent partir de réglages fiables, issus des données officielles du fabricant, plutôt que de valeurs glanées sur des forums.

> **Note**
> Le logiciel ne réhéberge jamais les PDF des fabricants. La base stocke les **faits utiles** (températures, densité, séchage, couleurs…) et un **lien direct** vers le document d'origine sur le site du fabricant.

---

## 2. Installation

L'application est distribuée sous forme de binaire léger (technologie Tauri) pour chaque système. La **release est une archive ZIP unique** contenant trois dossiers : `Windows/` (`.exe`), `MacOS/` (`.dmg`) et `Linux/` (`.AppImage`). Décompressez l'archive, puis ouvrez le dossier correspondant à votre système.

### Windows (`.exe`)

1. Ouvrez le dossier `Windows/` de l'archive et lancez le fichier `.exe`.
2. Suivez les étapes de l'installateur.
3. Démarrez l'application depuis le menu Démarrer.

### macOS (`.dmg` — application non signée)

1. Ouvrez le fichier `.dmg` situé dans le dossier `MacOS/` de l'archive.
2. Glissez l'application dans le dossier **Applications**.
3. **Au premier lancement**, ne double-cliquez pas : faites un **clic droit sur l'application > Ouvrir**, puis confirmez dans la boîte de dialogue.

> **Important**
> L'application n'est **pas signée** par un certificat de développeur Apple payant. macOS affichera donc un avertissement « application non vérifiée » au premier démarrage. Le contournement par **clic droit > Ouvrir** est normal et n'est nécessaire qu'une seule fois. Voir aussi la [FAQ](#10-dépannage--faq).

### Linux (`.AppImage`)

1. Récupérez le fichier `.AppImage` dans le dossier `Linux/` de l'archive.
2. Rendez-le exécutable :

   ```bash
   chmod +x Optimisateur-MD-*.AppImage
   ```

3. Lancez-le par double-clic ou en ligne de commande :

   ```bash
   ./Optimisateur-MD-*.AppImage
   ```

### Où l'application écrit-elle les profils ?

L'Optimisateur écrit ses profils dans le **dossier utilisateur du slicer choisi** (voir le [sélecteur de slicer](#3-premier-lancement--interface)), là où le slicer va les lire. Pour SnapmakerOrca / OptimusOrca, par exemple :

- les **profils filament** vont dans le sous-dossier `filament/` de votre profil utilisateur ;
- les **profils process** vont dans le sous-dossier `process/`.

Les autres slicers de la famille OrcaSlicer (OrcaSlicer, Bambu Studio, Creality Print) suivent la même organisation, dans leur propre dossier de préréglages ; avec l'option « dossier personnalisé », vous pointez directement l'emplacement voulu.

> **Important**
> Le dossier utilisateur du slicer choisi doit déjà exister. Si ce n'est pas le cas, **ouvrez le slicer au moins une fois** pour qu'il crée son dossier de profils, puis relancez la génération.

L'application tient aussi un **journal** dans son propre dossier de données système (utile pour le support en cas de problème).

---

## 3. Premier lancement & interface

Au premier démarrage, l'interface s'ouvre dans la langue du système (français ou anglais), puis mémorise votre choix.

### Sélecteur de langue (FR / EN)

En haut à droite, deux boutons **EN** et **FR** permettent de basculer l'interface. Le choix est conservé d'une session à l'autre.

### Bouton « Rechercher une mise à jour »

Sous l'en-tête se trouve un bouton **« Rechercher une mise à jour »**, accompagné d'une zone de statut. C'est lui qui installe et met à jour la **base de filaments** : au premier usage il télécharge la base courante, puis il récupère une version plus récente si le serveur en publie une. Voir la section [Mises à jour](#8-mises-à-jour).

### Sélecteur de slicer

Tout en haut de la fenêtre, un sélecteur **« Slicer »** indique dans quel slicer écrire les profils générés : **OrcaSlicer**, **Bambu Studio**, **Creality Print**, **SnapmakerOrca / OptimusOrca**, ou un **dossier personnalisé** que vous désignez vous-même. L'application résout automatiquement le dossier de préréglages utilisateur du slicer choisi selon votre système (Windows / macOS / Linux), et **mémorise** ce choix d'une session à l'autre. Voir aussi [Le sélecteur d'imprimante global](#5-le-sélecteur-dimprimante-global).

### Sélecteur d'imprimante global

Juste **au-dessus des onglets**, un sélecteur **« Imprimante »** permet de choisir **Marque → Modèle → Buse**, avec une option **« Toutes les buses »** pour traiter toutes les buses de la machine d'un coup. Ce sélecteur est **partagé par les deux bibliothèques** (Filament et process) : l'imprimante choisie ici sert aussi bien à la génération en un clic qu'à la génération des profils process seuls. Il couvre toute la **famille OrcaSlicer** (57 marques / 326 modèles). Voir la section [Le sélecteur d'imprimante global](#5-le-sélecteur-dimprimante-global).

### Les 3 onglets

Sous le sélecteur d'imprimante, l'interface s'organise en trois onglets, dans cet ordre :

| Onglet | Rôle |
|---|---|
| **Bibliothèque Filament** | Rechercher un matériau dans la base de filaments, puis générer en un clic le profil filament **et** ses profils process pour l'imprimante choisie. |
| **Bibliothèque process** | Générer le jeu de profils process par type de projet (pour l'imprimante choisie, ou le jeu complet Snapmaker U1). |
| **PDF unique** | Mode de secours : importer une seule fiche fabricant PDF pour un filament pas encore dans la base. |

---

## 4. La Bibliothèque Filament (mode principal)

C'est le mode central de l'application, dans l'onglet **Bibliothèque Filament**. Il relie la **base de données de filaments** au générateur de profils : vous choisissez un matériau et une imprimante, et l'application écrit le profil filament **et** ses profils process en un seul clic.

> Cet onglet remplace l'ancienne « Base de données locale ».

### Étapes

1. **Choisissez votre slicer** puis votre **imprimante** dans les sélecteurs en haut de la fenêtre (voir [Le sélecteur d'imprimante global](#5-le-sélecteur-dimprimante-global)).
2. **Recherchez un filament**. Vous pouvez d'abord restreindre la liste avec le menu déroulant **« Marque »** placé au-dessus de la recherche, puis affiner dans le champ de texte en tapant un **nom de produit** ou une **famille** (PLA, PETG, ABS…). Le filtre par marque et la recherche texte se combinent ; la liste se filtre au fur et à mesure.
3. **Sélectionnez un ou plusieurs matériaux** dans la liste (cochez-en plusieurs pour les traiter en une fois). Vous voyez les informations issues des fiches fabricant : famille de polymère, plage de **températures** (buse / plateau), **densité**, **couleurs** disponibles et liens vers les documents d'origine.
4. Cliquez sur **« Générer filament + process »**.
5. L'application écrit un **profil filament** par matière sélectionnée et un **jeu commun de 7 profils process** par type de projet, puis affiche un récapitulatif (profils filament créés, nombre de profils process, imprimante visée) et un **journal**.

### Un clic = filament + process

Depuis le matériau choisi et l'imprimante du sélecteur global, l'application génère **ensemble** :

- le **profil filament** (températures, débit, rétraction issus de la matière), rendu compatible avec l'imprimante choisie ;
- les **7 profils process** par type de projet (voir la section [La bibliothèque de profils PROCESS](#7-la-bibliothèque-de-profils-process)).

Le partage des réglages reste « **un jeu de process partagé + un réglage matière** » : le réglage propre au filament (températures, débit, rétraction) vit sur le **profil filament**, tandis que les profils process portent la géométrie d'impression, le cornering / la résonance et les fonctions du fork.

> **Note — option « Toutes les buses »**
> Si vous avez coché **« Toutes les buses »** dans le sélecteur, l'application génère les **7 profils process par buse** de la machine (par exemple ×4 pour le Snapmaker U1, soit 28 profils), en plus du ou des profils filament.

> **Note — multi-sélection et nom des profils**
> Si vous cochez **plusieurs matériaux**, l'application crée **un profil filament par matière** mais **un seul jeu commun** des 7 profils process (un jeu de process partagé ne peut pas être propre à chaque matière). Chaque profil filament est nommé « **Marque Matière** » (par exemple « Eryone PLA+ ») ; les caractères de nom de fichier légaux comme « + » sont conservés.

### Ce que contiennent les informations affichées

Les données proviennent des fiches officielles du fabricant et alimentent le profil :

- la **plage de température de buse** (et la valeur retenue, au milieu de la plage) ;
- la **température de plateau** ;
- les **conditions de séchage** et la **densité** ;
- les **couleurs** (avec leur code couleur) ;
- les **liens** vers les fiches d'origine (TDS / SDS…).

Les champs qui ont dû être **estimés** (faute de valeur dans la fiche) sont complétés à partir de valeurs par défaut propres à la famille de polymère, puis signalés : le profil peut alors porter un badge **« À vérifier »** pour vous inviter à contrôler avant une impression critique. Un profil complet porte le badge **« Prêt »**.

---

## 5. Le sélecteur d'imprimante global

En haut de la fenêtre, **au-dessus des onglets**, le sélecteur **« Imprimante »** détermine la machine pour laquelle les profils seront générés. Il est **partagé par la Bibliothèque Filament et la Bibliothèque process**. Il fonctionne en tandem avec le **sélecteur de slicer** (voir la [section 3](#sélecteur-de-slicer)), qui décide *dans quel slicer* les profils sont écrits.

### Choisir sa machine

1. Choisissez la **Marque** (par exemple Snapmaker, Creality, Bambu Lab, Prusa, Anycubic…).
2. Choisissez le **Modèle**.
3. Choisissez la **Buse**, ou sélectionnez **« Toutes les buses »** pour générer les profils pour toutes les buses de la machine en une fois.

Le catalogue couvre toute la **famille OrcaSlicer** : **57 marques** et **326 modèles**.

### Multi-imprimante : un profil filament correct

Le profil filament généré est rendu **compatible avec l'imprimante choisie** :

- pour le **Snapmaker U1**, il hérite du parent réglé pour la U1 (chaîne « @U1 ») ;
- pour **toute autre imprimante** de la famille OrcaSlicer, il hérite du profil de série **« Generic &lt;polymère&gt; »** (par exemple Generic PLA, Generic PETG…).

Ainsi, le filament apparaît bien dans le menu du slicer pour la machine sélectionnée, et part de réglages de base cohérents avec elle.

---

## 6. PDF unique (mode de secours)

Pour un filament qui ne figure **pas encore dans la base de données**, l'onglet **PDF unique** permet de générer un profil à partir d'une fiche fabricant que vous fournissez vous-même. C'est le dernier onglet de l'interface.

### Étapes

1. **Déposez un PDF** dans la zone prévue (« Glisser un .pdf ici, ou cliquer pour choisir un fichier »), ou cliquez pour ouvrir le sélecteur de fichiers.
2. Laissez cochée (ou non) l'option **« Chercher aussi la TDS du fabricant en ligne (recommandé) »**. Si votre PDF est une simple fiche de sécurité (SDS) sans données d'impression, cette option permet d'aller chercher la fiche technique (TDS) correspondante sur le site du fabricant pour compléter le profil.
3. Laissez cochée (ou non) l'option **« Partager cette fiche (anonyme) avec la base communautaire »**, **cochée par défaut**. Après un import réussi, elle envoie les seuls **faits fabricant** de la fiche pour enrichir la base partagée (voir la sous-section [Contributions communautaires](#contributions-communautaires-anonymes-et-facultatives) ci-dessous). Elle est entièrement facultative et ne bloque jamais l'import.
4. Cliquez sur **« Créer le profil filament »**.
5. L'application affiche le résultat (champs extraits, éventuels badges) et un **journal** détaillant ce qui a été fait.

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

### Contributions communautaires (anonymes et facultatives)

La base de filaments **s'enrichit au fil du temps grâce aux imports partagés**. Sur l'onglet **PDF unique**, la case **« Partager cette fiche (anonyme) avec la base communautaire »** est **cochée par défaut**, mais vous pouvez la décocher à tout moment.

Concrètement, après un **import réussi** :

- l'application envoie **uniquement les faits fabricant** extraits de la fiche : marque, matière, type de base, fenêtre de température (buse / plateau), densité, lien vers la fiche et date de révision ;
- ces données rejoignent la **file de modération** de Maison Drabiec ; après relecture, les entrées validées intègrent la base partagée distribuée aux utilisateurs ;
- la **donnée fabricant garde toujours la priorité** sur toute autre source.

Ce qui **n'est jamais** envoyé : le **PDF** lui-même, vos **chemins de fichiers**, et toute **donnée personnelle ou liée à votre machine**. Le serveur ne conserve qu'un **identifiant d'IP haché** pour limiter les abus.

> **Note**
> Le partage est **entièrement optionnel** et **ne bloque jamais** l'import : si vous décochez la case, le profil est généré exactement de la même façon, simplement sans contribution.

---

## 7. La bibliothèque de profils PROCESS

L'onglet **Bibliothèque process** génère des profils de process par **type de projet** pour l'imprimante choisie dans le [sélecteur global](#5-le-sélecteur-dimprimante-global) (toute la famille OrcaSlicer : Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…) : l'application produit les 7 profils calibrés pour cette imprimante précise. Un bouton génère aussi, en un clic, le **jeu complet Snapmaker U1** (7 types × 4 buses = 28 profils).

Ce sont **les mêmes 7 profils process** que produit la génération en un clic de la [Bibliothèque Filament](#4-la-bibliothèque-filament-mode-principal) ; cet onglet sert à les régénérer seuls, sans repasser par un matériau.

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

### Supports & adhérence (anti-warping)

Les profils process intègrent des réglages pensés pour que **les supports tiennent au plateau mais se détachent proprement du modèle**, et pour **limiter le warping et le décollement** de la pièce :

- **Détachement support → modèle** : jeu vertical au-dessus du support (`support_top_z_distance` 0,2 mm), interface rectiligne espacée (`support_interface_spacing` 0,5 mm, `support_interface_pattern` rectilinear), distance XY 0,35 mm — le support s'enlève à la main sans arracher la surface. (Ce réglage est **inchangé** quel que soit le type de projet.)
- **Anti-warping par type de projet** : comme les profils process forment un **jeu partagé** (non lié à la matière), l'adhérence au plateau se règle selon le **type de projet**. Les types **fonctionnels à plus grande empreinte** — **Objet du quotidien, Jouet, Pièce mécanique** — reçoivent un **brim extérieur modéré** (`brim_type` outer_only) pour limiter le warping et le décollement. Les types **esthétiques** — **Figurine, Vase, Décoration** — et le **Prototype rapide** n'en reçoivent pas, pour ne pas alourdir le nettoyage ni marquer la pièce.

> Ces réglages ne s'activent que lorsque la découpe génère des supports / un brim ; ce sont des valeurs de départ prudentes, ajustables ensuite. (Recherche : `data/RESEARCH_supports_adhesion.md`.)

### Générer les profils

**Pour n'importe quelle imprimante** (génération à la demande) :
1. Choisissez votre **marque**, votre **modèle** et votre **buse** dans le [sélecteur d'imprimante global](#5-le-sélecteur-dimprimante-global), en haut de la fenêtre. (Avec **« Toutes les buses »**, les profils sont générés pour chaque buse de la machine.)
2. Ouvrez l'onglet **Bibliothèque process**.
3. Cliquez sur **« Générer les process pour l'imprimante choisie »** — l'application écrit les **7 profils** (un par type de projet) dans le dossier `process/` de votre profil utilisateur, en héritant du process de base de cette imprimante (chaîne OrcaSlicer → SnapmakerOrca).

**Raccourci Snapmaker U1** : le bouton **« Générer le jeu Snapmaker U1 »** produit directement les **28 profils** (7 types × 4 buses) pour la U1.

> **Important**
> Si le dossier utilisateur de Snapmaker_Orca n'est pas trouvé, ouvrez OptimusOrca une fois pour qu'il crée votre dossier de profils, puis relancez la génération.

---

## 8. Mises à jour

L'Optimisateur sépare clairement **deux choses** : la **base de données** des filaments (de simples données) et l'**application** elle-même (le binaire).

La base est **embarquée** : un instantané hors-ligne est posé dans le dossier de données de l'application à la première utilisation, de sorte que la Bibliothèque Filament fonctionne sans connexion. Au **premier usage** du bouton, l'application télécharge la base courante depuis le serveur ; par la suite, elle ne récupère une nouvelle base que si le serveur en publie une plus récente.

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
2. Sélectionnez **l'imprimante** (et le diamètre de buse) que vous aviez choisie dans le sélecteur global au moment de la génération.
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

- **Licences.** L'**Optimisateur de filament et de profils d'impression by Maison Drabiec** est un logiciel **propriétaire**, réservé à un usage strictement **personnel et privé** (voir `LICENSE.md`). Le slicer cible **OptimusOrca / Snapmaker_Orca**, lui, reste un logiciel **libre** distribué sous licence **AGPL-3.0-or-later** : les deux licences sont distinctes.
- **Données filaments.** Les données proviennent des **sites officiels des fabricants**. Quand une donnée fabricant existe, elle prime sur toute autre source. L'application **ne réhéberge pas** les PDF des fabricants : elle stocke les faits utiles et, le cas échéant, un lien vers le document d'origine. Les **contributions communautaires** éventuelles (voir [section 6](#contributions-communautaires-anonymes-et-facultatives)) sont anonymes et limitées aux faits fabricant.
- **Marque.** « Optimisateur de filament et de profils d'impression by Maison Drabiec » et le monogramme MD sont la marque de Maison Drabiec.

> Les standards publics (GHS, ISO 11014-1) sont des références publiques ; aucun schéma ou contenu de profil propriétaire n'est reproduit.
