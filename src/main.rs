//! Binaire cahierPDF (J2 — Pages).
//!
//! Fenêtre Slint : vue de la page courante, panneau de miniatures (F-08 :
//! réordonner par glisser-déposer, F-09 : naviguer), actions F-03 à F-07,
//! annuler/rétablir (F-35).
//!
//! Limites connues, consignées dans `docs/mesures.md` (§J2) :
//! - le backend winit de Slint 1.18.1 ne relaie pas les dépôts de fichiers
//!   de l'OS : le `DropArea` de F-03 est en place mais n'attend que ce
//!   relais ; « Ouvrir » (champ de chemin) et la ligne de commande couvrent
//!   l'import ;
//! - aucun enregistrement (J4) : fermer la fenêtre perd le travail.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use pdfium_render::prelude::Pdfium;
use slint::{ComponentHandle, ModelRc, SharedPixelBuffer, VecModel};

use cahierpdf::model::{
    DuplicatePage, FORMAT_VERSION, InsertPages, Metadata, ModelError, MovePage, Notebook,
    Orientation, Page, PageBackground, RemovePages, SourceId, UndoStack,
};
use cahierpdf::pdf::{self, Raster};

slint::include_modules!();

/// Échelle d'affichage de la vue courante : 420 px pour 595 pt de largeur A4,
/// soit `DISPLAY_K` pixels par point PDF.
const DISPLAY_K: f32 = 420.0 / 595.0;

/// Vignettes : 96 px au plus, posées dans des cartes de 92 × 130 px.
const THUMB_MAX_PX: u32 = 96;

/// État applicatif : le document, son historique et les rendus mis en cache.
struct App {
    notebook: Notebook,
    history: UndoStack,
    /// Page courante, index 0-based ; borné par `refresh`.
    current: usize,
    /// Chemins des PDF importés (J4 stockera les binaires, le chemin devient
    /// inutile).
    sources: HashMap<SourceId, PathBuf>,
    /// Vignettes déjà rendues, par (identifiant de source, page source).
    thumbs: HashMap<(u64, u32), slint::Image>,
    pdfium: Pdfium,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

fn set_status(ui: &MainWindow, message: impl Into<slint::SharedString>) {
    let message = message.into();
    println!("[ui] {message}");
    ui.set_status(message);
}

/// Convertit un rendu RGBA en image Slint (dimensions exactes).
fn raster_to_image(raster: &Raster) -> slint::Image {
    let mut buffer = SharedPixelBuffer::<slint::Rgba8Pixel>::new(raster.width, raster.height);
    let pixels = buffer.make_mut_bytes();
    let copied = pixels.len().min(raster.rgba.len());
    pixels[..copied].copy_from_slice(&raster.rgba[..copied]);
    slint::Image::from_rgba8(buffer)
}

/// Enregistre la source et importe les pages d'un PDF dans `notebook`
/// (F-03, F-04). Les pages sont liées à la source enregistrée ; le PDF
/// n'est jamais modifié (NF-07).
fn import_pdf(
    pdfium: &Pdfium,
    notebook: &mut Notebook,
    sources: &mut HashMap<SourceId, PathBuf>,
    path: &Path,
) -> Result<Vec<Page>, pdf::PdfError> {
    let fingerprint = pdf::fingerprint(path)?;
    let source = notebook.add_source(fingerprint);
    let pages = pdf::import_pages(pdfium, path, source)?;
    sources.insert(source, path.to_path_buf());
    Ok(pages)
}

/// Rafraîchit toute l'interface depuis l'état : miniatures (avec cache),
/// vue courante (fond ajusté F-10), barre d'état.
fn refresh(ui: &MainWindow, app: &mut App) {
    let count = app.notebook.page_count();
    if app.current >= count {
        app.current = count - 1;
    }

    // Miniatures (F-09) : description des pages, puis rendu des vignettes
    // manquantes, un seul ouvrage de document par source.
    let mut rows = Vec::with_capacity(count);
    let mut by_source: HashMap<u64, Vec<u32>> = HashMap::new();
    for index in 0..count {
        let Ok(page) = app.notebook.page(index) else {
            continue;
        };
        let landscape = page.orientation() == Orientation::Landscape;
        let label: slint::SharedString = format!("Page {}", index + 1).into();
        match page.background() {
            PageBackground::PdfPage {
                source, page_index, ..
            } => {
                rows.push((label, landscape, true, Some((source.get(), *page_index))));
                by_source.entry(source.get()).or_default().push(*page_index);
            }
            PageBackground::Blank => rows.push((label, landscape, false, None)),
        }
    }
    for (source_raw, indices) in &by_source {
        let missing: Vec<u32> = indices
            .iter()
            .copied()
            .filter(|page_index| !app.thumbs.contains_key(&(*source_raw, *page_index)))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let Some(path) = app.sources.get(&SourceId::new(*source_raw)).cloned() else {
            continue;
        };
        match pdf::render_thumbnails(&app.pdfium, &path, &missing, THUMB_MAX_PX) {
            Ok(rasters) => {
                for (page_index, raster) in missing.iter().zip(rasters) {
                    app.thumbs
                        .insert((*source_raw, *page_index), raster_to_image(&raster));
                }
            }
            Err(error) => set_status(ui, format!("vignettes indisponibles : {error}")),
        }
    }
    let items: Vec<PageThumb> = rows
        .into_iter()
        .map(|(label, landscape, has_pdf, key)| PageThumb {
            label,
            thumb: key
                .and_then(|key| app.thumbs.get(&key).cloned())
                .unwrap_or_default(),
            has_pdf,
            landscape,
        })
        .collect();
    ui.set_pages(ModelRc::new(VecModel::from(items)));

    // Vue courante (F-10 : fond ajusté proportionnellement et centré).
    if let Ok(page) = app.notebook.page(app.current) {
        let (width_pt, height_pt) = page.size();
        ui.set_page_w_px(width_pt * DISPLAY_K);
        ui.set_page_h_px(height_pt * DISPLAY_K);
        ui.set_current_landscape(page.orientation() == Orientation::Landscape);
        let fit = page.background_fit();
        let mut image = slint::Image::default();
        let (mut offset_x, mut offset_y) = (0.0f32, 0.0f32);
        if let (
            Some(fit),
            PageBackground::PdfPage {
                source,
                page_index,
                source_size,
            },
        ) = (fit, page.background())
        {
            if let Some(path) = app.sources.get(source) {
                let width_px = ((source_size.width() * fit.scale()) * DISPLAY_K)
                    .round()
                    .max(1.0) as u32;
                let height_px = ((source_size.height() * fit.scale()) * DISPLAY_K)
                    .round()
                    .max(1.0) as u32;
                match pdf::render_page(&app.pdfium, path, *page_index, width_px, height_px) {
                    Ok(raster) => {
                        image = raster_to_image(&raster);
                        offset_x = fit.dx() * DISPLAY_K;
                        offset_y = fit.dy() * DISPLAY_K;
                    }
                    Err(error) => set_status(ui, format!("fond indisponible : {error}")),
                }
            }
        }
        ui.set_page_image(image);
        ui.set_page_img_x_px(offset_x);
        ui.set_page_img_y_px(offset_y);
    }

    ui.set_current_page(app.current as i32);
    ui.set_can_undo(app.history.can_undo());
    ui.set_can_redo(app.history.can_redo());
}

/// F-03 : « Ouvrir » (ou glisser-déposer, ou « Ouvrir avec » en ligne de
/// commande) — toutes les pages du PDF deviennent les pages du cahier.
fn open_pdf(ui: &MainWindow, app: &mut App, path_str: &str) {
    let path = Path::new(path_str);
    let mut notebook = Notebook::new(Metadata {
        title: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sans titre".to_string()),
        created_at: now(),
        updated_at: now(),
        format_version: FORMAT_VERSION,
    });
    match import_pdf(&app.pdfium, &mut notebook, &mut app.sources, path) {
        Ok(mut pages) => {
            let imported = pages.len();
            if notebook.insert_pages(0, &mut pages).is_err() {
                set_status(ui, format!("« {path_str} » : insertion impossible"));
                return;
            }
            if imported > 0 {
                // La page blanche initiale (F-02) est remplacée par l'import.
                let _ = notebook.remove_page(imported);
            }
            app.notebook = notebook;
            app.history.clear();
            app.thumbs.clear();
            app.current = 0;
            set_status(
                ui,
                format!("ouvert : {} ({imported} page(s))", path.display()),
            );
        }
        Err(error) => set_status(ui, format!("ouverture impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-04 : insérer un PDF dans le cahier existant, avant ou après la page
/// courante (commande réversible).
fn insert_pdf(ui: &MainWindow, app: &mut App, path_str: &str, after: bool) {
    let path = Path::new(path_str);
    let at = app.current + usize::from(after);
    match import_pdf(&app.pdfium, &mut app.notebook, &mut app.sources, path) {
        Ok(pages) => {
            let imported = pages.len();
            match app
                .history
                .execute(InsertPages::new(at, pages), &mut app.notebook)
            {
                Ok(()) => {
                    if imported > 0 {
                        app.current = at;
                    }
                    set_status(
                        ui,
                        format!("inséré : {} ({imported} page(s))", path.display()),
                    );
                }
                Err(error) => set_status(ui, format!("insertion impossible : {error}")),
            }
        }
        Err(error) => set_status(ui, format!("import impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-05 : insérer une page blanche (même orientation que la page courante).
fn add_blank_page(ui: &MainWindow, app: &mut App, after: bool) {
    let at = app.current + usize::from(after);
    let orientation = app
        .notebook
        .page(app.current)
        .map(|page| page.orientation())
        .unwrap_or(Orientation::Portrait);
    match app.history.execute(
        InsertPages::new(at, vec![Page::blank(orientation)]),
        &mut app.notebook,
    ) {
        Ok(()) => {
            app.current = at;
            set_status(ui, format!("page blanche insérée en {at}"));
        }
        Err(error) => set_status(ui, format!("insertion impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-06 : dupliquer la page courante (annotations et fond compris).
fn duplicate_page(ui: &MainWindow, app: &mut App) {
    match app
        .history
        .execute(DuplicatePage::new(app.current), &mut app.notebook)
    {
        Ok(()) => {
            app.current += 1;
            set_status(ui, format!("page {} dupliquée", app.current));
        }
        Err(error) => set_status(ui, format!("duplication impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-07 : supprimer la page courante, annulable.
fn delete_page(ui: &MainWindow, app: &mut App) {
    match RemovePages::new(vec![app.current]) {
        Ok(command) => match app.history.execute(command, &mut app.notebook) {
            Ok(()) => set_status(
                ui,
                format!("page {} supprimée (annulable)", app.current + 1),
            ),
            Err(ModelError::LastPage) => {
                set_status(ui, "impossible : le cahier garde au moins une page")
            }
            Err(error) => set_status(ui, format!("suppression impossible : {error}")),
        },
        Err(error) => set_status(ui, format!("suppression impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-08 : déplacer une page (glisser-déposer des miniatures ou ↑/↓).
fn move_page(ui: &MainWindow, app: &mut App, from: i32, to: i32) {
    let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
        return;
    };
    let count = app.notebook.page_count();
    let to = to.min(count - 1);
    if from >= count || from == to {
        return;
    }
    match app
        .history
        .execute(MovePage::new(from, to), &mut app.notebook)
    {
        Ok(()) => {
            app.current = to;
            set_status(ui, format!("page déplacée en position {}", to + 1));
        }
        Err(error) => set_status(ui, format!("déplacement impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-09 : aller à une page (clic sur une miniature).
fn goto_page(ui: &MainWindow, app: &mut App, index: i32) {
    let Ok(index) = usize::try_from(index) else {
        return;
    };
    app.current = index.min(app.notebook.page_count() - 1);
    refresh(ui, app);
}

fn undo(ui: &MainWindow, app: &mut App) {
    match app.history.undo(&mut app.notebook) {
        Ok(()) => set_status(ui, "annulé"),
        Err(ModelError::NothingToUndo) => set_status(ui, "rien à annuler"),
        Err(error) => set_status(ui, format!("annulation impossible : {error}")),
    }
    refresh(ui, app);
}

fn redo(ui: &MainWindow, app: &mut App) {
    match app.history.redo(&mut app.notebook) {
        Ok(()) => set_status(ui, "rétabli"),
        Err(ModelError::NothingToRedo) => set_status(ui, "rien à rétablir"),
        Err(error) => set_status(ui, format!("rétablissement impossible : {error}")),
    }
    refresh(ui, app);
}

/// F-03 : glisser-déposer d'un PDF (reçu du `DropArea`). Le backend winit de
/// Slint 1.18.1 ne relaie pas encore les dépôts de l'OS : ce traitement
/// devient actif dès ce relais.
fn file_dropped(ui: &MainWindow, app: &mut App, data: &slint::DataTransfer) {
    if !data.has_file_paths() {
        set_status(ui, "dépôt ignoré : aucun fichier reçu");
        return;
    }
    let Ok(paths) = data.file_paths() else {
        set_status(ui, "dépôt illisible");
        return;
    };
    for path in paths {
        let is_pdf = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
        if is_pdf {
            let owned = path.to_string_lossy().into_owned();
            open_pdf(ui, app, &owned);
            return;
        }
    }
    set_status(ui, "dépôt ignoré : aucun PDF parmi les fichiers");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_pdf = std::env::args().nth(1).map(PathBuf::from);

    // Bibliothèque PDFium : chargée une seule fois par processus.
    let pdfium = pdf::bind()?;
    println!("[info] cahierPDF J2 — PDFium lié");

    let app = Rc::new(RefCell::new(App {
        notebook: Notebook::new(Metadata {
            title: "Sans titre".to_string(),
            created_at: now(),
            updated_at: now(),
            format_version: FORMAT_VERSION,
        }),
        history: UndoStack::new(100),
        current: 0,
        sources: HashMap::new(),
        thumbs: HashMap::new(),
        pdfium,
    }));

    let ui = MainWindow::new()?;

    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_open_pdf(move |path| {
            let Some(ui) = ui_weak.upgrade() else { return };
            open_pdf(&ui, &mut app.borrow_mut(), path.as_str());
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_insert_pdf(move |path, after| {
            let Some(ui) = ui_weak.upgrade() else { return };
            insert_pdf(&ui, &mut app.borrow_mut(), path.as_str(), after);
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_add_blank_page(move |after| {
            let Some(ui) = ui_weak.upgrade() else { return };
            add_blank_page(&ui, &mut app.borrow_mut(), after);
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_duplicate_page(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            duplicate_page(&ui, &mut app.borrow_mut());
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_delete_page(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            delete_page(&ui, &mut app.borrow_mut());
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_move_page(move |from, to| {
            let Some(ui) = ui_weak.upgrade() else { return };
            move_page(&ui, &mut app.borrow_mut(), from, to);
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_goto_page(move |index| {
            let Some(ui) = ui_weak.upgrade() else { return };
            goto_page(&ui, &mut app.borrow_mut(), index);
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_undo(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            undo(&ui, &mut app.borrow_mut());
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_redo(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            redo(&ui, &mut app.borrow_mut());
        });
    }
    {
        let app = Rc::clone(&app);
        let ui_weak = ui.as_weak();
        ui.on_file_dropped(move |data| {
            let Some(ui) = ui_weak.upgrade() else { return };
            file_dropped(&ui, &mut app.borrow_mut(), &data);
        });
    }

    // « Ouvrir avec » (F-03) : un PDF passé en ligne de commande.
    match cli_pdf {
        Some(path) => {
            let owned = path.to_string_lossy().into_owned();
            open_pdf(&ui, &mut app.borrow_mut(), &owned);
        }
        None => {
            let mut app = app.borrow_mut();
            set_status(&ui, "Prêt. Ouvrez un PDF, ou insérez des pages.");
            refresh(&ui, &mut app);
        }
    }

    ui.run()?;
    Ok(())
}
