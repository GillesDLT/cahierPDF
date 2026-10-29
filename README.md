# cahierPDF

Prise de notes au stylet sur pages A4, avec import/annotation/export de PDF.
Rust + Slint, cibles Linux : **NixOS + GNOME** (prioritaire) et
**CachyOS + Hyprland** (secondaire).

- Le PDF source n'est **jamais** modifié : les annotations sont stockées à part.
- Toute modification du document passe par une commande réversible
  (annuler/rétablir).
- Projet conduit jalon par jalon (cf. `docs/cahierPDF — Cahier des charges.md`
  et `AGENTS.md`).

## État actuel

**J2 — Pages** : ouverture/import de PDF (F-03, F-04), pages blanches (F-05),
duplication (F-06), suppression annulable (F-07), réorganisation par
glisser-déposer et panneau de miniatures (F-08, F-09). Vérifications et
observations : `docs/mesures.md` §J2.

À venir : J3 (formes), J4 (sauvegarde SQLite, export PDF), J5 (finition,
packaging). Non couvert pour l'instant : écriture au stylet (J1, en attente de
validation matérielle), enregistrement, zoom/défilement.

## Construire

Prérequis : Rust stable (≥ 1.92, édition 2024) et les dépendances système de
Slint 1.18 (backend winit + rendu femtovg) :

**CachyOS / Arch :**

```sh
sudo pacman -S --needed base-devel rustup fontconfig freetype2 libxkbcommon \
    libxkbcommon-x11 wayland libx11 libxcb mesa
rustup default stable
```

**Debian / Ubuntu :**

```sh
sudo apt install build-essential cargo fontconfig libfontconfig-dev \
    libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev libxcb1-dev \
    libgl1-mesa-dev
```

> Note (retour d'expérience, `spikes/stylo-slint/flake.nix`) : winit charge
> `libxkbcommon-x11.so` par `dlopen` — si elle manque au démarrage, installer
> `libxkbcommon-x11` ou régler `LD_LIBRARY_PATH`.

```sh
git clone <ce-dépôt> && cd cahierPDF
cargo build --release
```

## Bibliothèque PDFium

`pdfium-render` charge PDFium **à l'exécution** (jamais à la compilation).
Chemin de recherche : variable `CAHIERPDF_PDFIUM`, puis `./libpdfium.so`,
puis bibliothèque système. Obtenir la version validée (chromium/7881) :

```sh
mkdir -p .pdfium && cd .pdfium
curl -sL -o pdfium-linux-x64.tgz \
  https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7881/pdfium-linux-x64.tgz
tar xzf pdfium-linux-x64.tgz && cd ..
export CAHIERPDF_PDFIUM=$PWD/.pdfium/lib/libpdfium.so
```

## Lancer

```sh
./target/release/cahierpdf            # cahier vierge (1 page A4)
./target/release/cahierpdf doc.pdf    # « Ouvrir avec » : les pages du PDF
```

Interface : champ de chemin + boutons « Ouvrir » / « Insérer » (avant/après la
page courante), miniatures cliquables et déplaçables par glisser-déposer,
annuler/rétablir. Limite connue : le backend winit de Slint 1.18.1 ne relaie
pas les dépôts de fichiers de l'OS (glisser-déposer depuis le gestionnaire de
fichiers) — voir `docs/mesures.md` §J2.

## Tests

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
CAHIERPDF_PDFIUM=$PWD/.pdfium/lib/libpdfium.so cargo test   # + intégration PDF
```

## Licence

Double licence **MIT OR Apache-2.0** (D-05, NF-08) — voir `LICENSE-MIT` et
`LICENSE-APACHE`.

La bibliothèque PDFium chargée à l'exécution n'est pas incluse dans ce dépôt ;
sa licence propre est documentée par son distributeur
(<https://github.com/bblanchon/pdfium-binaries>, `LICENSE` et `licenses/`).
Compatibilité avec la double licence : **à vérifier** avant distribution de
binaires (D-05).
