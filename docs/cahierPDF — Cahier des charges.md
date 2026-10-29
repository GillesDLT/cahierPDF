# cahierPDF — Cahier des charges

Version 0.2 — 29 octobre 2026 — Auteur : Gilles

> **Changements v0.2 (par rapport à v0.1)** — détail dans la section 12.
> - F-01 : pages A4 en portrait **ou** paysage, orientation conservée par page (était : portrait seul).
> - F-10 : passe de S à M ; mise à l'échelle proportionnelle et centrée, sans recadrage (Q-1 tranchée).
> - F-11 : reporté en v2 (calques en interface) ; le modèle les prévoit dès la v1.
> - Q-2 : pas de marge ; l'écriture est bornée au rectangle de la page.
> - NF-08 / Q-4 : licence **MIT OR Apache-2.0**.
> - Q-3 / §7.1 : format de projet **SQLite** (`rusqlite`), implémentation en J4.
> - NF-02 : mesure indicative en J0, validation formelle en J1.
> - §9 : définitions de R-1 et R-4 ajoutées.

## 1. Vision

**cahierPDF** est un logiciel de prise de notes manuscrites sur Linux, centré sur l'annotation de PDF au stylet numérique. Il s'inspire de NoteWise pour l'interface (barres flottantes, esthétique « glassy », épurée) et le concept (un cahier infini de pages A4 où l'on mélange PDF importés, pages blanches, écriture et formes).

Objectifs prioritaires :

- Une **écriture au stylet fluide**, sans latence perceptible.
- Une **gestion des pages souple** : importer un PDF, insérer des pages blanches n'importe où, dupliquer, réordonner.
- Des **formes manipulables immédiatement** (redimensionner, recolorer, tourner).
- Une **empreinte mémoire faible**.
- Un **export PDF** propre et fidèle.

## 2. Plateformes cibles

| Plateforme | Priorité | Remarque |
| --- | --- | --- |
| NixOS + GNOME (Wayland) | Principale (~90 % de l'usage) | Fiabilité exigée, packaging par flake Nix |
| CachyOS + Hyprland (Wayland) | Secondaire | Doit fonctionner correctement, sans développement spécifique |
| X11, Windows, macOS | Hors périmètre | Non testés |

Matériel visé : tablette ou portable avec écran à stylet (protocole Wayland `tablet-v2`, pression, inclinaison si disponible) et écran tactile.

Machine de référence (D-09) : **Dell Latitude 5290 12,3″** pour la RAM et l'ouverture de PDF. Pour la latence, il faut un **écran à stylet confirmé** (la présence d'un stylet sur cette machine est **à vérifier**).

## 3. Périmètre de la version 1

### 3.1 Inclus (v1)

- Document au format **A4** (595 × 842 pt), en portrait ou en paysage, orientation conservée par page.
- Import de PDF, insertion et duplication de pages.
- Dessin au stylet, défilement tactile.
- Formes éditables.
- Sauvegarde du projet et export PDF.

### 3.2 Exclus de la v1

- Calques manipulables dans l'interface (le modèle de données les prévoit dès la v1, l'interface vient en v2).
- Reconnaissance d'écriture (OCR), recherche dans l'écriture manuscrite.
- Synchronisation cloud, collaboration.
- Édition du texte du PDF d'origine.
- Gestion des PDF protégés par mot de passe.

## 4. Exigences fonctionnelles

Priorités : **M** = indispensable (Must), **S** = souhaitable (Should), **C** = optionnel (Could).

### 4.1 Document et pages

| ID | Exigence | Priorité |
| --- | --- | --- |
| F-01 | Toutes les pages sont au format A4, en **portrait ou en paysage**. L'orientation est conservée par page. | M |
| F-02 | Créer un nouveau cahier vide (1 page blanche A4). | M |
| F-03 | Importer un PDF par « Ouvrir », glisser-déposer ou « Ouvrir avec ». Toutes ses pages deviennent des pages du cahier. | M |
| F-04 | Importer un PDF dans un cahier existant, à une position choisie (avant ou après la page N). | M |
| F-05 | Insérer une **page blanche** à n'importe quelle position (par exemple entre les pages 5 et 6). | M |
| F-06 | **Dupliquer** une page, avec ses annotations et son fond PDF éventuel. La copie se place juste après l'originale. | M |
| F-07 | Supprimer une page (avec annulation possible). | M |
| F-08 | Réordonner les pages par glisser-déposer dans un panneau de miniatures. | S |
| F-09 | Panneau de pages miniatures navigable et fluide. | S |
| F-10 | Les pages d'un PDF importé qui ne sont pas au format de la page du cahier sont mises à l'échelle **proportionnellement et centrées**, sans recadrage, en gardant leur orientation. | M |
| F-11 | Jusqu'à 3 calques par page, comme une application de dessin. **Reporté en v2** (interface, cf. §3.2) ; le modèle prévoit les calques dès la v1. | C |

### 4.2 Navigation et affichage

| ID | Exigence | Priorité |
| --- | --- | --- |
| F-20 | Défilement des pages une par une pour rester économique en RAM et en fluidité de zoom et d'écriture. | S |
| F-21 | **Défilement au doigt** (glisser) et zoom par pincement. Le stylet ne fait jamais défiler. | M |
| F-22 | Zoom et déplacement à la molette/pavé tactile et au clavier. | M |
| F-23 | Rendu net à tout niveau de zoom (fond PDF re-rendu, traits vectoriels). | M |

### 4.3 Dessin au stylet

| ID | Exigence | Priorité |
| --- | --- | --- |
| F-31 | Lissage des traits et prédiction de trajectoire pour réduire la latence apparente. (Toggle) | M |
| F-32 | **Rejet de la paume** : seuls le stylet et le doigt (pour le défilement) sont interprétés, pas de trace involontaire. (toggle) | S |
| F-33 | Outils : stylo, surligneur (semi-transparent), gomme (par trait ou exactement sous la gomme). | M |
| F-34 | Palette de couleurs rapide et choix d'épaisseur. | M |
| F-35 | Annuler / rétablir. | S |
| F-36 | Bouton du stylet et extrémité « gomme » reconnus comme raccourcis. | S |
| F-37 | Lasso : sélectionner, déplacer, recolorer des traits. | S |
| F-38 | Option « le doigt dessine » (désactivée par défaut). | C |

### 4.4 Formes

| ID | Exigence | Priorité |
| --- | --- | --- |
| F-40 | Formes : ligne, flèche, rectangle, ellipse. | M |
| F-41 | Création par outil dédié, et par **reconnaissance « maintenir en fin de tracé »** (dessiner puis tenir le stylet pour redresser). | C |
| F-42 | Sélection d'une forme : **poignées** de redimensionnement, déplacement. | M |
| F-43 | **Changement de proportions immédiat** : transformer un rectangle en carré (verrouillage du ratio, magnétisme sur le ratio 1:1 pendant le glissement, ou saisie numérique des dimensions). | M |
| F-44 | **Rotation** par poignée, libre, avec magnétisme léger (par pas de 15° ou retour à 0°) pour permettre les petites rotations. | M |
| F-45 | Couleur du contour, couleur de remplissage (avec transparence), épaisseur, trait plein ou pointillé, modifiables **immédiatement** via une barre contextuelle flottante apparaissant à la sélection. | M |
| F-46 | Dupliquer, supprimer, mettre au premier ou à l'arrière-plan. | M |
| F-47 | Coins arrondis du rectangle. | C |
| F-48 | Un slider pour gérer le nombre de côtés de la forme « polygone » (par défaut 4 pour le rectangle) : de 3 (triangle) à 10 (décagone). | C |
| F-49 | Insertion d'image, gestion de leurs tailles et possibilité de rogner. | C |

### 4.5 Sauvegarde et export

| ID | Exigence | Priorité |
| --- | --- | --- |
| F-50 | Sauvegarde du cahier dans un fichier projet (`.cahierpdf`), avec sauvegarde automatique régulière. | M |
| F-51 | **Export PDF final** : pages dans l'ordre du cahier, annotations et formes en **vectoriel**, fond PDF conservé. | M |
| F-52 | Export d'une sélection de pages. | S |
| F-53 | Texte et liens du PDF d'origine conservés à l'export quand c'est possible. | S |
| F-54 | Export PNG d'une page. | C |

## 5. Exigences non fonctionnelles

| ID | Exigence | Cible |
| --- | --- | --- |
| NF-01 | Mémoire (RSS) avec un PDF de 100 pages ouvert | < 150 Mo, objectif 100 Mo |
| NF-02 | Latence stylet (événement → pixel) | < 20 ms ressentis, avec prédiction. Mesure **indicative** en J0 ; validation **formelle** en J1 sur un écran à stylet confirmé |
| NF-03 | Fluidité | 60 images/s au défilement et à l'écriture |
| NF-04 | Ouverture d'un PDF de 100 pages | < 2 s jusqu'à la première page affichée |
| NF-05 | Démarrage à froid | < 2 s |
| NF-06 | Fiabilité | Aucune perte de données en cas de plantage (sauvegarde auto + transactions de la base, cf. §7.1) |
| NF-07 | Le PDF d'origine n'est jamais modifié | Obligatoire |
| NF-08 | Licence | **MIT OR Apache-2.0** (double licence, convention Rust). Dépôt public sur GitHub. |

## 6. Interface (inspirée de NoteWise)

- **Barre d'outils flottante** en haut, semi-transparente, avec effet « verre » : outils (stylo, surligneur, gomme, lasso, formes, texte plus tard), couleurs, épaisseurs.
- **Barre contextuelle** qui apparaît au-dessus d'une sélection (couleur, remplissage, épaisseur, dupliquer, supprimer).
- **Zone de travail** sombre, fond de page A4 avec ombre légère, grille de points optionnelle.
- **Panneau latéral** escamotable : miniatures des pages, menu d'insertion (page blanche, importer un PDF, dupliquer).
- Thème clair et sombre, suivant le thème Ayu.
- Interface plein écran par défaut sur un nouveau projet vide, sans barre de titre encombrante.

## 7. Architecture technique

### 7.1 Choix

| Couche | Choix | Note |
| --- | --- | --- |
| Langage | **Rust** | Mémoire maîtrisée, pas de ramasse-miettes |
| Interface | **Slint** | Léger, déclaratif, bon pour le style « glassy » |
| Rendu du canvas | À trancher après prototype : rendu logiciel optimisé (tiny-skia) ou GPU (wgpu + Vello) | Voir risques R-1 et R-4 |
| Rendu PDF | **pdfium-render** (PDFium) | MuPDF écarté (AGPL, incompatible avec MIT OR Apache-2.0). Compatibilité des licences PDFium / pdfium-render : **à vérifier** |
| Export PDF | `pdf-writer` ou `lopdf`, avec import des pages d'origine comme objets réutilisables | Annotations en vectoriel |
| Géométrie | `kurbo` (courbes, transformations), `rstar` (index spatial) | |
| Sauvegarde | **SQLite** via `rusqlite` : un fichier `.cahierpdf` ; une transaction par modification ou sauvegarde auto ; PDF sources en BLOB ; version du format par `user_version` | Décidé en v0.2 (D-06). **Implémentation en J4 uniquement.** |
| Packaging | Flake Nix (NixOS) ; paquet AUR ou Flatpak plus tard (CachyOS) | |

### 7.2 Principes

1. **Séparation stricte** entre le modèle (documents, pages, objets), le moteur de rendu et l'interface.
2. **Rendu par tuiles visibles uniquement**, avec cache limité en taille.
3. **Pages chargées à la demande**, les pages éloignées sont déchargées.
4. Toutes les modifications passent par des **commandes** réversibles (annuler/rétablir).

## 8. Modèle de données

```
Cahier
 ├─ métadonnées (titre, dates, version du format)
 ├─ sources[]        PDF importés (copiés dans le projet, identifiés par empreinte)
 └─ pages[]
     ├─ format : A4 (595 × 842 pt) ; orientation : Portrait | Paysage
     ├─ fond : Blanc | PagePDF(source, numéro)
     └─ calques[]    (au moins un par page en v1 ; interface en v2)
         └─ objets[]
             ├─ Trait   : points (x, y, pression), outil, couleur, épaisseur
             ├─ Forme   : type, boîte englobante, rotation, contour, remplissage
             └─ (plus tard) Texte, Image
```

- Coordonnées en **points PDF** dans le repère de la page, indépendantes du zoom. Le repère est de 595 × 842 en portrait et de 842 × 595 en paysage.
- L'orientation est une propriété de **chaque page** (F-01) : une page paysage a son propre repère.
- Les **formes** stockent leurs paramètres (type, largeur, hauteur, angle) et non des tracés, d'où la modification instantanée (rectangle → carré).
- Les calques existent dans le modèle dès la v1 pour éviter une migration de format plus tard.

## 9. Risques et points à valider (prototypes)

| ID | Risque | Parade |
| --- | --- | --- |
| R-1 | **Stylet dans Slint** : la chaîne pression / inclinaison / gomme (protocole Wayland `tablet-v2`) n'est pas garantie de bout en bout dans Slint. Risque de ne pas recevoir ces événements, ou de les recevoir avec retard. | Spike J0 : vérifier quelles données de stylet Slint (ou la couche fenêtre sous-jacente) expose, et mesurer le délai. API exacte **à vérifier par prototype**. |
| R-2 | **Effet « verre »** : le flou de ce qui se trouve derrière un élément n'est pas natif dans Slint. | Simuler avec transparence, dégradés et bordures lumineuses ; flou calculé uniquement sur des zones fixes si nécessaire. |
| R-3 | **Fiabilité sous GNOME** : décorations de fenêtre côté client et intégration au thème. | Tester tôt sous GNOME ; choisir une interface sans barre de titre classique. |
| R-4 | **Performance du rendu** : coût RAM et fluidité du rendu PDF + canvas (100 pages, 60 images/s, ouverture < 2 s). Définition ajoutée en v0.2, **à valider**. | Rendu par tuiles et cache borné ; choix tiny-skia ou wgpu + Vello **par mesure** en J0, pas par avance. |
| R-5 | **Export PDF fidèle** (polices, liens, pages à rotation ou tailles non standard). | S'appuyer sur la réutilisation des pages d'origine plutôt que leur reconstruction. |
| R-6 | **Rejet de la paume** variable selon le matériel. | Tests sur la tablette cible ; réglages exposés. |

## 10. Jalons

| Jalon | Contenu | Critère de réussite |
| --- | --- | --- |
| J0 — Spikes | Fenêtre Slint + canvas avec pression ; affichage d'une page PDF ; mesures RAM et latence (latence **indicative**) | R-1 et R-4 levés, choix du rendu fait |
| J1 — Écriture | Traits lissés, gomme, couleurs, annuler/rétablir ; **validation formelle de NF-02** sur écran à stylet confirmé | Écrire confortablement sur une page |
| J2 — Pages | Document multipages, import PDF, insertion de pages blanches, duplication, suppression, miniatures | F-03 à F-09 validées |
| J3 — Formes | Outils de formes, sélection, poignées, rotation, barre contextuelle | F-40 à F-46 validées |
| J4 — Fichiers | Sauvegarde SQLite, sauvegarde automatique, export PDF vectoriel | F-50 à F-52 validées |
| J5 — Finition | Style « glassy », thèmes, packaging Nix, tests sous Hyprland | Utilisable au quotidien |

## 11. Questions ouvertes

Questions tranchées en v0.2 (voir section 12) :

- **Q-1** (mise à l'échelle) : tranchée, proportionnelle et centrée, orientation conservée (D-02).
- **Q-2** (marges) : tranchée, pas de marge, écriture bornée au rectangle de la page (D-03).
- **Q-3** (format de fichier) : tranchée, format SQLite documenté, implémentation en J4 (D-06).
- **Q-4** (licence) : tranchée, MIT OR Apache-2.0 (D-05).
- **Q-5** (orientation paysage) : tranchée, oui, par page (D-01).

Questions restant ouvertes :

- **Q-6** (machine de référence) : Dell Latitude 5290 retenue pour RAM et ouverture. **Présence d'un stylet à vérifier** ; sans écran à stylet, la latence ne peut pas être validée.
- **F-11 / §3.2** : F-11 est reporté en v2 (D-04). Si vous voulez des calques en v1, il faudra revoir §3.2 et le périmètre de J3.

## 12. Journal des décisions (v0.2)

| ID | Décision | Raison | Statut |
| --- | --- | --- | --- |
| D-01 | F-01 : pages A4 en portrait ou paysage, orientation par page. Le modèle stocke l'orientation par page. | Q-5 = oui ; une page paysage a un repère différent. | Décidé |
| D-02 | Q-1 : mise à l'échelle proportionnelle, centrée, sans recadrage, orientation gardée. F-10 passe de S à M. | Sans cela, un PDF non A4 est inutilisable. | Décidé |
| D-03 | Q-2 : pas de marge. Écriture bornée au rectangle de la page. | La contradiction de v0.1 venait d'une formulation ambiguë. | Décidé |
| D-04 | F-11 reporté en v2 ; calques dans le modèle dès la v1 (≥ 1 par page). | Évite un conflit avec §3.2 ; le modèle reste prêt. | Décidé (à confirmer par le propriétaire si besoin) |
| D-05 | Licence **MIT OR Apache-2.0**. MuPDF écarté. | Standard Rust ; l'AGPL de MuPDF est incompatible. | Décidé ; compatibilité PDFium **à vérifier** |
| D-06 | Format `.cahierpdf` = **SQLite** (`rusqlite`), transactions, PDF sources en BLOB, `user_version`. | Atomicité (NF-06) sans réécrire le fichier ; lisible avec un outil SQLite standard. Le ZIP est écarté : chaque sauvegarde réécrirait l'archive entière. | Décidé ; implémentation en J4 |
| D-07 | NF-02 : mesure indicative en J0 (résultat non retenu comme preuve) ; validation formelle en J1. | R-1 doit être levé en J0, mais la latence réelle exige un écran à stylet. | Décidé |
| D-08 | R-1 et R-4 définis en §9 (absents de v0.1). | Le critère de sortie de J0 n'était pas vérifiable. | Décidé ; à valider par le propriétaire |
| D-09 | Machine de référence : Dell Latitude 5290 pour RAM et ouverture ; écran à stylet requis pour la latence. | Q-6 ; présence du stylet non confirmée. | Partiel ; **à vérifier** |

**Restent à traiter hors du cahier** (pas des décisions du cahier) : AGENTS.md renvoie à `docs/cahier-des-charges.md`, alors que le fichier s'appelle `docs/cahierPDF — Cahier des charges.md` ; à corriger dans AGENTS.md.
