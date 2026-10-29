# AGENTS.md — cahierPDF

Contexte permanent pour tout agent de code travaillant sur ce dépôt.
Le cahier des charges complet (exigences F-xx / NF-xx, risques R-x, jalons J0–J5) fait foi : `docs/cahierPDF — Cahier des charges.md`.

## Projet
Logiciel Linux de prise de notes au stylet sur pages A4, avec import/annotation/export de PDF.
Cibles : NixOS + GNOME (Wayland, prioritaire), CachyOS + Hyprland (secondaire).

## Stack
- Rust (édition courante stable), interface **Slint**, rendu PDF **pdfium-render**.
- Géométrie : `kurbo` ; index spatial : `rstar` ; export PDF : `pdf-writer` ou `lopdf`.
- Packaging : flake Nix avec `devShell`.
- **Interdit** : webview, Electron, moteur JS, toute dépendance lourde sans accord explicite.

## Contraintes qui dictent le code
- RAM < 150 Mo avec un PDF de 100 pages ouvert.
- Latence stylet ressentie < 20 ms, 60 images/s.
- Le PDF source n'est **jamais** modifié ; les annotations sont stockées à part.
- Coordonnées en points PDF dans le repère de la page A4 (595 × 842), indépendantes du zoom.
- Les formes stockent leurs paramètres (type, largeur, hauteur, angle), pas un tracé.
- Toute modification du document passe par une **commande réversible** (annuler/rétablir).
- Le modèle prévoit déjà des calques (≥ 1 par page), même si l'UI arrive plus tard.

## Architecture (modules)
`model/` (document, pages, objets, commandes) — pur, sans UI ni I/O, testé unitairement.
`render/` (tuiles, cache, traits, formes) — dépend de `model`.
`pdf/` (import via PDFium, export vectoriel) — dépend de `model`.
`io/` (sauvegarde `.cahierpdf`, écriture atomique, sauvegarde auto).
`ui/` (fenêtre Slint, barres flottantes, gestion des événements stylet/doigt).

## Règles de code
- `cargo fmt`, et `cargo clippy --all-targets -- -D warnings` propres avant tout commit.
- Pas de `unwrap()`/`expect()` hors tests et initialisation ; erreurs typées (`thiserror`), `anyhow` seulement dans `main`.
- `unsafe` interdit sauf FFI justifiée par un commentaire `// SAFETY:`.
- Pas d'allocation dans la boucle de rendu des traits ; réutiliser les tampons.
- Mesurer (RAM, latence) avant d'optimiser ; consigner les mesures dans `docs/mesures.md`.
- Commits petits, un par changement logique, message à l'impératif.

## Boucle de vérification (à exécuter soi-même, sans attendre qu'on le demande)
1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. Si l'UI est touchée : lancer l'app et décrire ce qui a été observé.
Ne jamais déclarer une tâche terminée sans avoir lu la sortie de ces commandes.

## Méthode de travail
- Travailler **un jalon à la fois** (voir le cahier des charges). Ne pas anticiper le jalon suivant.
- Avant de coder : plan en 3 à 6 lignes, critère de réussite, fichiers touchés.
- **Ne pas inventer d'API** (Slint, pdfium-render, winit…) : lire la documentation de la version figée dans `Cargo.lock`. En cas de doute, le dire explicitement et proposer un mini-test pour vérifier.
- Si une exigence du cahier des charges est ambiguë, poser la question au lieu de deviner.
- Les risques R-1 (stylet dans Slint) et R-2 (effet « verre ») se règlent par des **prototypes jetables** dans `spikes/`, pas dans le code principal.

## État courant
Jalon actif : **J2 — Pages** — implémenté (F-03 à F-09 : import PDF,
commandes de pages réversibles, miniatures) et vérifié sous Xvfb
(`docs/mesures.md` §J2). Reste ouvert : test matériel stylet (R-1, NF-02 —
J0/J1 non clos côté validation), glisser-déposer de fichiers (relais backend
winit), licence PDFium (D-05), validation sur machine réelle.
Prochain jalon : **J3 — Formes**.