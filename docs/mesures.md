# cahierPDF — Mesures

Traces de mesures et de vérifications (cf. AGENTS.md : « consigner les mesures
dans `docs/mesures.md` »).

## T0 — Ouverture de la fenêtre du spike `spikes/stylo-slint/` (Xvfb)

- **Date** : 29 octobre 2026
- **Environnement** : conteneur Debian 13 (trixie), x86_64, sans serveur
  d'affichage réel — rendu sous Xvfb (:99, écran 1280x800x24).
- **Toolchain** : rustc 1.99.1 / cargo 1.99.1 ; slint 1.18.1 (figée dans
  `spikes/stylo-slint/Cargo.lock`).
- **Binaire** : `spikes/stylo-slint/target/debug/stylo-slint` (profil dev).

### Commandes et sorties brutes

```console
$ Xvfb :99 -screen 0 1280x800x24 &

$ DISPLAY=:99 ./target/debug/stylo-slint &

$ ps -p $APP_PID
APP RUNNING (pid 8411)

$ cat /tmp/stylo.log
Error watching for xdg desktop settings: Failed to connect to address `unix:path=/run/user/1000/bus`: No such file or directory (os error 2)

$ DISPLAY=:99 xdotool search --name cahierPDF
2097155

$ DISPLAY=:99 xdotool getwindowgeometry 2097155
Window 2097155
  Position: 0,0 (screen: 0)
  Geometry: 800x600

$ DISPLAY=:99 import -window root docs/mesures-t0-xvfb.png
-rw-r--r-- 1 user user 2437 Oct 29 12:48 /home/user/CahierPDF/docs/mesures-t0-xvfb.png
```

### Observations

- La fenêtre **s'ouvre et reste vivante** : titre `cahierPDF - spike T0
  stylo-slint`, géométrie **800x600** (conforme aux `width: 800px` /
  `height: 600px` déclarés dans le markup Slint), position (0,0) sous Xvfb.
- Affichage (capture ci-dessous) : zone de travail sombre `#1e1e2e`, zone de
  dessin vide blanche **420 x 594 px** centrée, proportions A4 (ratio 842/595),
  en px, sans équivalence en pt. Liseré gris 1 px.
- Avertissement bénin unique : pas de bus D-Bus session dans le conteneur
  (`Error watching for xdg desktop settings`), sans effet sur l'affichage.

Capture : [`mesures-t0-xvfb.png`](./mesures-t0-xvfb.png)

### Historique de la séance (à garder pour le contexte)

- Première tentative : **panique au démarrage** —
  `xkbcommon-dl-0.4.2/src/x11.rs: Library libxkbcommon-x11.so could not be
  loaded` (winit/X11 charge cette bibliothèque par `dlopen`). Corrigé côté
  conteneur par l'installation de `libxkbcommon-x11-0` (Debian 1.7.0-2), puis
  lancement réussit. **TODO laissé en l'état** : comportement équivalent à
  confirmer sous nix (point 7 de la revue 05-relecteur, non traité sans accord).

### Non mesuré ici (hors périmètre de T0)

- RAM et latence stylet (NF-01/NF-02) : objets des spikes suivants de J0.
- Comportement sous Wayland natif et GNOME (cible principale) : machine cible
  requise.

## T1 — non mesuré (pas de stylet)

- **H-1** : « le stylet remonte pression et inclinaison dans Slint ». Elle n’est **pas vérifiée** : aucune session avec stylet/écran à stylet possible dans cet environnement. **R-1 n’est pas levé.** Prototype compilable de journalisation des événements Slint ; aucune pression n’est générée ou substituée pour les tests.
- **API vérifiée** : Slint **1.18.1**, version du `Cargo.lock` de `spikes/stylo-slint`.
  Les chemins ci-dessous sont relatifs à `$CARGO_HOME/registry/src/index.crates.io-1949cf8c6b5b557f/`.

| Donnée | Ce que le source Slint expose réellement | Source (fichier:ligne) |
|---|---|---|
| Pression | **Non exposée** par `PointerEvent` / `TouchArea` : les champs de la structure sont `button`, `kind`, `modifiers`, `touch_finger_id`, sans pression. | `i-slint-common-1.18.1/builtin_structs.rs:76-88` ; `i-slint-core-1.18.1/items/input_items.rs:131-136,170-185` |
| Inclinaison | **Non exposée** dans cette API (aucun axe tilt dans `PointerEvent`). | `i-slint-common-1.18.1/builtin_structs.rs:79-88` |
| Gomme | **Non exposée/identifiable** dans cette API : pas de type d’outil ; le bouton Slint est limité à `Other/Left/Right/Middle/Back/Forward`, sans type gomme. | `i-slint-common-1.18.1/enums.rs:195-212` |
| Boutons | Événement générique exposé : `button` + `kind` (`Cancel/Down/Up/Move`). Ces boutons ne prouvent pas l’identité d’un bouton physique propre au stylet ; `Other` n’identifie rien de plus. | `i-slint-common-1.18.1/builtin_structs.rs:79-87` ; `i-slint-common-1.18.1/enums.rs:182-212` |
| Coordonnées / doigt | `TouchArea` expose `mouse-x`/`mouse-y` ; `PointerEvent` contient un identifiant tactile, 0 signifiant non tactile. Le prototype relaie les coordonnées locales lors de l’événement. | `i-slint-compiler-1.18.1/builtin_elements.rs:1304-1326` ; `i-slint-common-1.18.1/builtin_structs.rs:86-87` |

Le prototype `spikes/stylo-slint/src/main.rs` relie `TouchArea.pointer-event` à Rust et écrit kind, bouton, id tactile et position dans stdout. Smoke test sous Xvfb avec `xdotool` (souris uniquement, **pas un test stylet**) :

```text
pointer kind=Move button=Other touch_finger_id=0 position=(210.0, 297.0)
pointer kind=Down button=Left touch_finger_id=0 position=(210.0, 297.0)
pointer kind=Up button=Left touch_finger_id=0 position=(210.0, 297.0)
pointer kind=Move button=Other touch_finger_id=0 position=(210.0, 297.0)
pointer kind=Move button=Other touch_finger_id=0 position=(230.0, 317.0)
```

Cela teste uniquement la plomberie Slint sans injecter d’événements ou de valeurs fictifs ; seul un périphérique pourra montrer si les mouvements réels du stylet arrivent et comment ils sont classés.

### Couche fenêtre / alternative Wayland (documentée, non implémentée)

Slint utilise ici le backend winit. Son adaptateur convertit `CursorMoved` en mouvements de souris (et les regroupe, `i-slint-backend-winit-1.18.1/winitwindowadapter.rs:1448-1461`), puis transmet les événements génériques `MouseInput` (`:1486-1522`). Pour `Touch`, il transmet seulement id, position et phase (`:1524-1541`), pas `winit::event::Touch.force`. Bien que winit ait un champ `force` pour certains événements tactiles (`winit-0.30.13/src/event.rs:841-855`), ce n’est ni une API stylet Wayland ni une preuve que Slint le conserve. Le code Wayland de winit 0.30.13 n’expose pas d’événement outil tablet dédié (à revalider dans la version réellement utilisée si cette voie est retenue).

Plan B à prototyper seulement si nécessaire : dessiner dans une zone gérée directement par winit, en amont de la normalisation Slint, ou étendre la couche fenêtre/backend pour préserver les données du stylet ; si cette API ne suffit pas, recevoir Wayland `tablet-v2` sur le `wl_surface` de la fenêtre. **À vérifier** : la version winit inspectée ne présente pas d’événement outil tablet dédié côté Wayland, donc « winit direct » ne garantit pas à lui seul pression/inclinaison/gomme et peut nécessiter une adaptation du backend ou un accès au protocole. Le protocole définit le type d’outil incluant `eraser` (`wayland-protocols-0.32.13/protocols/stable/tablet/tablet-v2.xml:265-283`), pression (`:461-470`), inclinaison (`:483-492`), boutons (`:541-553`) et regroupement par `frame` (`:555-563`). L’accès au `wl_display`/`wl_surface` créé par winit reste lui aussi à vérifier. La réception effective dépendra de l’appareil et du compositeur (Mutter/GNOME prioritaire, Hyprland secondaire). Aucun accès direct ni changement d’architecture n’est fait dans T1.

### Prochaine session avec matériel — tests et mesures à consigner

1. **Pression** : confirmer si les événements arrivent ; relever les valeurs brutes et leur plage min/max, vérifier variation progressive à plusieurs forces, repos et contact. Comparer Slint et, si Slint ne fournit rien, le prototype de couche fenêtre/tablet-v2. Ne pas supposer une plage sans la mesurer.
2. **Inclinaison** : stylet vertical puis incliné dans plusieurs directions ; vérifier axes, signe, unité/plage et continuité.
3. **Gomme et boutons** : inverser le stylet, tester chaque bouton latéral et vérifier type d’outil, transitions et absence de confusion avec les boutons souris.
4. **Rejet de la paume / distinction doigt-stylet** : stylet seul, doigt seul, paume seule, puis stylet + paume/doigt simultanés ; consigner événements parasites, traits involontaires et pertes d’événements.
5. **Latence réelle** : mesurer événement→pixel au ralenti avec caméra haute vitesse (240 fps = 4,17 ms/image, résolution temporelle à déclarer), répéter au moins 30 traits, rapporter médiane/P95/max et la méthode. Mesurer aussi les temps de trame (objectif 60 Hz : budget 16,67 ms, rapporter médiane/P95/max) avant/après tout changement de pipeline ; latence ressentie cible < 20 ms.
6. **RAM** : mesurer RSS au repos et en charge avec le PDF de référence de 100 pages, via `/usr/bin/time -v` ou `/proc/<pid>/smaps_rollup`, avant/après tout changement ; comparer à la cible < 150 Mo.

Pour chaque essai, consigner appareil/modèle, noyau, version du compositeur, backend/chemin d’événements, fréquence d’échantillonnage si observable et sorties brutes. À ce stade, pression, inclinaison, gomme, rejet de la paume, latence réelle, temps de trame sous stylet et RSS sous charge sont **non mesurés** ; la conformité à < 20 ms / 60 images/s / < 150 Mo n’est pas établie par T1.

## T2 — Rendu PDF (spike `spikes/pdf-rendu/`)

- **Date** : 29 octobre 2026
- **Environnement** : conteneur Debian 13 (trixie), x86_64, 4 cœurs ; affichage
  sous Xvfb (`:99`, 1280x800x24) pour la fenêtre ; mesures sans fenêtre en
  l'absence de serveur d'affichage réel.
- **Toolchain** : rustc 1.99.1 / cargo 1.99.1, profil **release**.
- **Versions figées** (`spikes/pdf-rendu/Cargo.lock`) : `pdfium-render 0.9.4`,
  `slint 1.18.1`, `image 0.25.10`, `libloading 0.9.0`.
- **Binaire** : `spikes/pdf-rendu/target/release/pdf-rendu`.
- **Périmètre** : ouverture d'un PDF de 100 pages, rendu d'une page, affichage
  dans une fenêtre Slint avec repli PNG sur disque, mesures RSS/temps.
  Aucun stylet, aucun choix de moteur de canvas (R-4 : la décision de rendu du
  canvas se prend avec ces chiffres, pas par avance).

### Comment la bibliothèque PDFium est obtenue

- `pdfium-render 0.9.4` **ne lie pas** PDFium à la compilation : il charge une
  bibliothèque partagée **à l'exécution** via `libloading`
  (`Pdfium::bind_to_library(chemin)` / `Pdfium::bind_to_system_library()`,
  source figée : `pdfium-render-0.9.4/src/pdfium.rs:105-260`). La feature
  `static` existe mais n'est pas utilisée ici.
- La bibliothèque utilisée pour ces mesures est le binaire précompilé
  **`pdfium-linux-x64.tgz` de la release `chromium/7881`** de
  <https://github.com/bblanchon/pdfium-binaries> (VERSION : `151.0.7881.0`),
  extrait dans `spikes/pdf-rendu/.pdfium/` (ignoré par git). Ce numéro de
  release est celui que la crate elle-même vise : la feature par défaut de
  `pdfium-render 0.9.4` est `pdfium_7881` (`pdfium_latest` = `pdfium_7881`,
  README §Features).
- Chemins de recherche du spike, dans l'ordre : variable
  **`PDF_RENDU_PDFIUM`** (chemin explicite du `.so`, utilisée pour toutes les
  mesures ci-dessous), sinon `./libpdfium.so`, sinon bibliothèque système
  (`dlopen("libpdfium.so")`, résolution via `ldconfig`/`LD_LIBRARY_PATH`).
  **À noter** : `pdfium-render 0.9.4` n'implémente **aucune** variable
  d'environnement de recherche (pas de `PDFIUM_DYNAMIC_LIBRARY_PATH` dans le
  source figé) ; `PDF_RENDU_PDFIUM` est une variable propre au spike.
- Commande d'obtention reproductible :

```console
$ cd spikes/pdf-rendu && mkdir -p .pdfium && cd .pdfium
$ curl -sL -o pdfium-linux-x64.tgz \
    https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F7881/pdfium-linux-x64.tgz
$ tar xzf pdfium-linux-x64.tgz && cat VERSION
MAJOR=151
MINOR=0
BUILD=7881
PATCH=0
```

### PDF de test de 100 pages

Aucun PDF réel de 100 pages n'existe sur la machine (le plus long document
vrai : `Guide_Abaqus_CAE_pas_a_pas.pdf`, 28 pages A4). Le PDF de référence
`spikes/pdf-rendu/sorties/cent-pages.pdf` est donc **assemblé au `pdfunite`**
à partir de documents réels de l'utilisateur, 100 pages exactement,
2 618 028 octets, contenu A4 (texte LaTeX, tableaux, encadrés colorés) :

```console
$ pdfunite <documents personnels de l'utilisateur, noms retirés>.pdf \
         sorties/cent-pages.pdf    # 100 pages au total
$ pdfinfo sorties/cent-pages.pdf | grep -E "^(Pages|File size)"
Pages:           100
File size:       2618028 bytes
```

### API utilisées (lues dans les sources figées, pas devinées)

| Besoin | API réellement utilisée | Source (fichier:ligne) |
|---|---|---|
| Liaison PDFium | `Pdfium::bind_to_library(path)` → `Pdfium::new(bindings)` | `pdfium-render-0.9.4/src/pdfium.rs:173,212` |
| Ouverture | `pdfium.load_pdf_from_file(path, None)` (chargement **paresseux** via `FPDF_LoadCustomDocument`) | `pdfium-render-0.9.4/src/pdfium.rs:371` |
| Pages | `document.pages().len()`, `.first()`, `.get(i)` | `pdfium-render-0.9.4/src/pdf/document/pages.rs:97,124,192` |
| Taille page | `page.width()/height()` → `PdfPoints { value: f32 }` | `pdfium-render-0.9.4/src/pdf/document/page.rs:248-255` |
| Rendu | `page.render(largeur_px, hauteur_px, None)` → `PdfBitmap` (l'appelant doit conserver le ratio) | `pdfium-render-0.9.4/src/pdf/document/page.rs:541` |
| Pixels | `PdfBitmap::as_rgba_bytes()` (copie normalisée RGBA), `as_image()` (feature `image_api`) | `pdfium-render-0.9.4/src/pdf/bitmap.rs:294,329` |
| PNG | `DynamicImage::save()` sur le résultat de `as_image()` | pattern README `pdfium-render-0.9.4/README.md` |
| Fenêtre | `slint::Image::from_rgba8(SharedPixelBuffer<Rgba8Pixel>)`, `slint::Timer::single_shot`, `slint::quit_event_loop` | `i-slint-core-1.18.1/graphics/image.rs:842`, `i-slint-core-1.18.1/timers.rs:148` |

### Commandes et sorties brutes

Méthode de mesure : RSS lu dans `/proc/self/status` (`VmRSS`, `VmHWM`, en kio)
par le binaire lui-même ; temps en ms par `Instant::std::time` depuis `t0`
(départ du processus utile, après analyse de la ligne de commande). Le conteneur
ne fournit pas `/usr/bin/time` (non installé) : la contre-mesure externe prévue
par la séance T1 n'a pas pu être faite ; `VmHWM` sert de pic de référence.

```console
$ export PDF_RENDU_PDFIUM=.pdfium/lib/libpdfium.so

$ # RUN 1 : PDF de 100 pages, page 0, sans fenêtre
$ ./target/release/pdf-rendu sorties/cent-pages.pdf --sans-fenetre --png sorties/cent-pages-p0.png
[info] fichier_pdf=sorties/cent-pages.pdf
[info] page_index=0
[info] echelle_px_par_pt=2
[info] pdfium_render=0.9.4 (figé dans Cargo.lock)
[mesure] rss_debut=6344.0 kio
[mesure] t_bind_pdfium=3.9 ms
[mesure] t_ouverture_document=4.6 ms
[info] nb_pages=100
[mesure] rss_apres_ouverture=8884.0 kio
[mesure] t_depuis_t0_apres_ouverture=13.8 ms depuis t0
[mesure] t_premiere_page_rendue=24.8 ms
[mesure] t_depuis_t0_premiere_page_rendue=39.0 ms depuis t0
[info] page_taille_pt=595.276x841.89
[info] page_cible_px=1191x1684
[mesure] t_rendu_page=5.0 ms
[mesure] t_rendu_page_2e=5.1 ms
[mesure] rss_apres_rendu=27088.0 kio
[info] png_ecrit=sorties/cent-pages-p0.png
[mesure] t_ecriture_png=4.0 ms
[info] fenetre=desactivee (--sans-fenetre)
[mesure] t_total=53.4 ms
[mesure] rss_fin=27332.0 kio
[mesure] rss_hwm=27332.0 kio

$ # RUN 2 : PDF de 100 pages, page 50 (milieu)
$ ./target/release/pdf-rendu sorties/cent-pages.pdf --sans-fenetre --page 50 --png sorties/cent-pages-p50.png
[info] fichier_pdf=sorties/cent-pages.pdf
[info] page_index=50
[info] echelle_px_par_pt=2
[info] pdfium_render=0.9.4 (figé dans Cargo.lock)
[mesure] rss_debut=6388.0 kio
[mesure] t_bind_pdfium=0.4 ms
[mesure] t_ouverture_document=0.3 ms
[info] nb_pages=100
[mesure] rss_apres_ouverture=9180.0 kio
[mesure] t_depuis_t0_apres_ouverture=1.0 ms depuis t0
[mesure] t_premiere_page_rendue=12.0 ms
[mesure] t_depuis_t0_premiere_page_rendue=13.6 ms depuis t0
[info] page_taille_pt=595.276x841.89
[info] page_cible_px=1191x1684
[mesure] t_rendu_page=11.6 ms
[mesure] t_rendu_page_2e=6.4 ms
[mesure] rss_apres_rendu=28744.0 kio
[info] png_ecrit=sorties/cent-pages-p50.png
[mesure] t_ecriture_png=5.7 ms
[info] fenetre=desactivee (--sans-fenetre)
[mesure] t_total=47.7 ms
[mesure] rss_fin=29280.0 kio
[mesure] rss_hwm=29280.0 kio

$ # RUN 3 : PDF réel seul (Guide_Abaqus, 28 pages A4), page 0
$ ./target/release/pdf-rendu documents/guide-exemple.pdf --sans-fenetre --png sorties/abaqus-p0.png
[info] fichier_pdf=documents/guide-exemple.pdf
[info] page_index=0
[info] echelle_px_par_pt=2
[info] pdfium_render=0.9.4 (figé dans Cargo.lock)
[mesure] rss_debut=6272.0 kio
[mesure] t_bind_pdfium=0.3 ms
[mesure] t_ouverture_document=1.5 ms
[info] nb_pages=28
[mesure] rss_apres_ouverture=9220.0 kio
[mesure] t_depuis_t0_apres_ouverture=2.0 ms depuis t0
[mesure] t_premiere_page_rendue=16.7 ms
[mesure] t_depuis_t0_premiere_page_rendue=19.1 ms depuis t0
[info] page_taille_pt=595.276x841.89
[info] page_cible_px=1191x1684
[mesure] t_rendu_page=5.1 ms
[mesure] t_rendu_page_2e=5.1 ms
[mesure] rss_apres_rendu=27064.0 kio
[info] png_ecrit=sorties/abaqus-p0.png
[mesure] t_ecriture_png=3.9 ms
[info] fenetre=desactivee (--sans-fenetre)
[mesure] t_total=33.5 ms
[mesure] rss_fin=27408.0 kio
[mesure] rss_hwm=27408.0 kio
```

RUN 4 — fenêtre Slint sous Xvfb (script jetable
`spikes/pdf-rendu/sorties/test-fenetre.sh`, capture
[`mesures-t2-fenetre-xvfb.png`](./mesures-t2-fenetre-xvfb.png)) :

```console
$ sh spikes/pdf-rendu/sorties/test-fenetre.sh
$ xdotool search --name "cahierPDF"
2097155
$ xdotool getwindowgeometry 2097155
Window 2097155
  Position: 0,0 (screen: 0)
  Geometry: 800x600
$ # sortie du spike (sorties/fenetre.log) :
[info] fichier_pdf=.../sorties/cent-pages.pdf
[info] page_index=3
[info] echelle_px_par_pt=2
[mesure] rss_debut=6460.0 kio
[mesure] t_bind_pdfium=0.3 ms
[mesure] t_ouverture_document=0.3 ms
[info] nb_pages=100
[mesure] rss_apres_ouverture=9312.0 kio
[mesure] t_depuis_t0_apres_ouverture=0.8 ms depuis t0
[mesure] t_premiere_page_rendue=11.2 ms
[mesure] t_depuis_t0_premiere_page_rendue=12.4 ms depuis t0
[info] page_taille_pt=595.276x841.89
[info] page_cible_px=1191x1684
[mesure] t_rendu_page=10.0 ms
[mesure] t_rendu_page_2e=6.1 ms
[mesure] rss_apres_rendu=27860.0 kio
[info] png_ecrit=.../sorties/fenetre-p3.png
[mesure] t_ecriture_png=5.2 ms
[mesure] t_fenetre_show=0.0 ms
[mesure] t_depuis_t0_fenetre_visible=93.1 ms depuis t0
[info] fenetre=ouverte fermeture_auto_s=8
Error watching for xdg desktop settings: Failed to connect to address `unix:path=/run/user/1000/bus`: No such file or directory (os error 2)
[mesure] t_total=8094.4 ms
[mesure] rss_fin=157672.0 kio
[mesure] rss_hwm=157672.0 kio
```

### Synthèse

| Mesure | RUN 1 (100 p, p0) | RUN 2 (100 p, p50) | RUN 3 (28 p réel) | RUN 4 (100 p, fenêtre) | Cible |
|---|---|---|---|---|---|
| RSS après ouverture (NF-01) | 8,7 Mo | 9,0 Mo | 9,0 Mo | 9,1 Mo | < 150 Mo (obj. 100) |
| RSS fin / pic (`VmHWM`) | 26,7 Mo | 28,6 Mo | 26,8 Mo | **154,0 Mo** | < 150 Mo |
| Première page rendue depuis `t0` (NF-04) | 39,0 ms | 13,6 ms | 19,1 ms | 12,4 ms | < 2 s |
| Rendu d'une page (1191×1684 px) | 5,0 ms | 11,6 ms | 5,1 ms | 10,0 ms | — |
| 2e rendu même page | 5,1 ms | 6,4 ms | 5,1 ms | 6,1 ms | — |

### Observations

- **NF-04 largement tenue** : la première page est rendue en **< 40 ms** après
  démarrage du processus, soit ~50 fois mieux que les 2 s visés, même en
  comptant le chargement de `libpdfium.so`. L'ouverture seule du document est
  paresseuse (`FPDF_LoadCustomDocument`) : 0,3 à 4,6 ms.
- **NF-01 tenue côté PDFium** : un PDF de 100 pages ouvert coûte **~9 Mo** de
  RSS ; après rendu d'une page à 144 ppp (2×), **~27 Mo**. PDFium ne charge que
  les portions nécessaires.
- **NF-01 mise en défaut par la fenêtre Slint (RUN 4) : pic à 154 Mo.** Ce n'est
  pas le PDF : le spike cumule alors le renderer Slint (fenêtre 800×600) et
  **trois copies RGBA** de la page rendue (bitmap PDFium ~7,7 Mo +
  `as_rgba_bytes()` ~7,7 Mo + `SharedPixelBuffer` Slint ~7,7 Mo, plus une copie
  `DynamicImage` pour le PNG). Des marges d'optimisation nettes existent :
  rendre directement dans le tampon Slint (`PdfBitmap::from_bytes`), supprimer
  la copie `as_rgba_bytes`, borner le cache. **À re-mesurer après optimisation
  avant de conclure sur R-4.**
- **Fidélité du rendu vérifiée visuellement** (PNG et capture Xvfb) : polices
  LaTeX sérif/gras/chasse fixe correctes, accents français, tableaux, encadrés
  colorés (« ASTUCE », « ATTENTION »), numéros de page. Aucune police manquante
  sur ces documents ; `as_rgba_bytes()` restitue les couleurs correctement.
- La fenêtre Slint s'ouvre sous Xvfb (800×600, position 0,0), affiche la page en
  `contain`, et se ferme toute seule ; code de sortie 0. Même avertissement
  bénin que T0 (pas de bus D-Bus session).
- `t_fenetre_show` vaut 0,0 ms : `show()` est asynchrone. La présentation de la
  première trame par le compositeur **n'est pas observable** depuis Slint
  1.18.1 (aucun retour « frame presented ») : `t_depuis_t0_fenetre_visible` est
  un **proxy** (fin de `show()`). À vérifier si NF-04 doit un jour être mesurée
  à l'écran réel (caméra, cf. séance T1).

### À vérifier (non établi par T2)

1. **Compatibilité PDFium ≠ 7881** : seule la release `chromium/7881` (celle
   visée par la feature par défaut de la crate) a été testée. Un `.so` plus
   récent (ex. `chromium/8125`) ou plus ancien n'a **pas** été testé. Mini-test :
   relancer RUN 1 avec `PDF_RENDU_PDFIUM` pointant sur un autre build et
   comparer les sorties + absence de crash.
2. **PDF chiffrés** : hors périmètre v1 (CdC §3.2), non testés. L'API prend
   `password: Option<&str>` ; comportement sur un PDF protégé à vérifier le jour
   où il entrera en périmètre.
3. **Pages tournées, pages de tailles non A4, PDF malformés** : non couverts par
   ce spike (toutes les pages de test sont A4 portrait). Cas limite à tester en
   J2/J4 avec un jeu dédié (ex. un jeu de test de 10 pages **letter**,
   paysage, déjà disponible sur la machine).
4. **Backend/renderer Slint réellement actif sous Xvfb** (femtovg ou skia selon
   les features compilées) : non instrumenté ; le pic de 154 Mo doit être
   redécomposé par backend avant de trancher R-4 (tiny-skia vs wgpu+Vello).
5. **Contre-mesure RSS externe** : `/usr/bin/time -v` absent du conteneur ;
   refaire avec cet outil ou `smaps_rollup` sur la machine cible.
6. **Démarrage à froid (NF-05)** : les mesures partent de `t0` après analyse des
   arguments ; elles n'incluent pas le dynamic linker ni (pour la fenêtre) la
   mise en place X11 avant `main`. NF-05 n'est pas validée ici.

### Rejouer les mesures

```console
cd spikes/pdf-rendu
PDF_RENDU_PDFIUM=.pdfium/lib/libpdfium.so ./target/release/pdf-rendu sorties/cent-pages.pdf --sans-fenetre
sh sorties/test-fenetre.sh   # fenêtre + capture sous Xvfb
```

Vérifications exécutées sur ce spike (sorties lues) : `cargo fmt --check` OK,
`cargo clippy --all-targets -- -D warnings` OK, `cargo test` **4 tests OK**,
`cargo check --locked` OK. Aucun commit effectué.

---

# J2 — Pages : vérifications et observations

Date : 2026-10-29. Environnement : conteneur de développement, Xvfb :99
(1400×1000×24), `xdotool` pour les événements synthétiques, `import`
(ImageMagick) pour les captures. Aucune donnée personnelle : le PDF de test
est généré (voir ci-dessous).

## Boucle de vérification (sorties lues)

| Commande | Résultat |
| --- | --- |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets -- -D warnings` | OK |
| `cargo test` | **58 tests OK** (56 unitaires modèle + 2 intégration PDF) |
| `cargo check --locked` | OK |

## Test d'intégration PDF (F-03, F-04, F-10)

Fixture reproductible : `tests/common/mod.rs` génère un PDF minimal (xref
calculé, pages de tailles choisies : A4 portrait, letter portrait, A4
paysage). Chaîne réelle exécutée avec la bibliothèque PDFium du spike :

```
$ CAHIERPDF_PDFIUM=spikes/pdf-rendu/.pdfium/lib/libpdfium.so cargo test --test pdf_import
test import_render_and_undo_pdf_pages ... ok
    test import_of_missing_file_is_typed_error ... ok   [2 tests OK]
```

Vérifié : import des 3 pages (orientations conservées, F-10), ajustement
proportionnel et centré (valeurs exactes), insertion à une position choisie
avec annulation/rétablissement (F-04), rendu page et vignettes (dimensions
exactes, RGBA complet). Sans la bibliothèque, le test s'ignore avec un
message `[ignore]` (le code de production exige la bibliothèque au démarrage).

## Recette observée sous Xvfb (F-03 à F-09)

PDF de test : `/tmp/cahierpdf-j2-test.pdf`, 3 pages A4 avec textes et
formule, **généré** par `pdflatex` depuis `/tmp/cahierpdf-j2-test.tex`
(contenu de démonstration, non versionné).

```
$ Xvfb :99 -screen 0 1400x1000x24 &
$ CAHIERPDF_PDFIUM=spikes/pdf-rendu/.pdfium/lib/libpdfium.so \
    ./target/debug/cahierpdf /tmp/cahierpdf-j2-test.pdf
[info] cahierPDF J2 — PDFium lié
[ui] ouvert : /tmp/cahierpdf-j2-test.pdf (3 page(s))     # F-03 « Ouvrir avec »
[ui] page 3 dupliquée                                    # F-06 (bouton)
[ui] page 4 supprimée (annulable)                        # F-07 (bouton)
[ui] page blanche insérée en 3                           # F-05 (avant/après page N)
[ui] page déplacée en position 3                         # F-08 (bouton ↑)
[ui] page déplacée en position 1                         # F-08 (glisser-déposer souris)
[ui] annulé / [ui] rétabli                               # F-35 (boutons)
[ui] inséré : /tmp/cahierpdf-j2-test.pdf (3 page(s))     # F-04 (champ + Insérer)
```

Captures : `docs/mesures-j2-1-ouverture.png` (ouverture : 3 vignettes
rendues, page 1 ajustée et centrée F-10, « page 1 / 3 ») ;
`docs/mesures-j2-2-insertion.png` (après insertion F-04 : 7 pages,
miniatures défilantes, « page 2 / 7 »). Le glisser-déposer de F-08 a été
effectué par une séquence `xdotool mousedown/mousemove/mouseup` réelle (1
pression, 8 mouvements, relâchement), et la navigation F-09 par clic sur
vignette. Un chemin vide saisi dans le champ ressort en erreur typée dans la
barre d'état, sans plantage.

| Exigence | Statut | Preuve |
| --- | --- | --- |
| F-03 | Validée **hors glisser-déposer** | « Ouvrir avec » (CLI) et « Ouvrir » (champ de chemin), log + captures |
| F-04 | Validée | insertion avant/après la page courante, annulable, log + capture |
| F-05 | Validée | page blanche à la position voulue, log |
| F-06 | Validée | copie identique (annotations et fond) juste après l'originale, log + capture |
| F-07 | Validée | suppression annulable (bouton + Annuler), log |
| F-08 | Validée **sous Xvfb** | glisser-déposer de vignette et boutons ↑/↓, log |
| F-09 | Validée **sous Xvfb** | panneau fluide (ListView), vignettes rendues, clic = navigation, captures |

## Limites et points à vérifier sur machine réelle

- **F-03 glisser-déposer de fichiers** : le backend winit de Slint 1.18.1 ne
  relaie pas les dépôts de fichiers de l'OS (vérifié dans le source figé :
  aucun traitement de `DroppedFile`/`HoveredFile` dans
  `i-slint-backend-winit-1.18.1`). Le `DropArea` de `ui/main.slint` est en
  place et devient actif dès que le backend relaiera ces événements. En
  attendant, « Ouvrir » et la ligne de commande couvrent l'import.
- **F-03 « Ouvrir »** : pas de dialogue de fichier natif (aucune dépendance
  de ce type sans accord, AGENTS.md) — champ de chemin à valider
  ergonomiquement sur machine réelle.
- **F-08** : le geste a été déclenché par événements synthétiques ; à
  retester à la souris/au doigt réels (le seuil de glisser est de 4 px).
- **NF-01 (RAM) et NF-03 (fluidité)** : non mesurés pour ce jalon. Les
  vignettes sont rendues à 96 px avec cache par (source, page) ; la vue
  courante est rendue à la demande en 420 px de large. Le zoom et le
  défilement (F-20 à F-23) sont hors J2.
- **R-1 / NF-02** (stylet) : inchangés, toujours en attente du test matériel
  (D-07) — J0 et J1 ne sont pas clos côté validation.
- **Aucune sauvegarde** (J4) : fermer la fenêtre perd le travail.
- **PDFium** : chargé au démarrage (`CAHIERPDF_PDFIUM` → `./libpdfium.so` →
  bibliothèque système). La compatibilité de licence avec MIT OR Apache-2.0
  reste **à vérifier** (D-05). Observé sous Xvfb : l'absence de D-Bus
  produit un avertissement bénin du backend winit
  (« Error watching for xdg desktop settings »), sans effet sur le fonctionnement.
