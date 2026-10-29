//! Import et rendu PDF (F-03, F-04, F-09, F-10) via PDFium.
//!
//! Moteur : `pdfium-render 0.9.4`, bibliothèque PDFium chargée à l'exécution
//! (`libloading`), jamais liée à la compilation. La chaîne de chargement est
//! celle du spike `spikes/pdf-rendu` ; les API utilisées sont vérifiées dans
//! le source figé (`docs/mesures.md`, tableau « Besoin | API réellement
//! utilisée »), aucune n'est devinée.
//!
//! Le PDF source n'est **jamais** modifié (NF-07) : ouvert en lecture seule,
//! ses pages deviennent des [`Page`] dont le fond est une simple référence
//! ([`PageBackground::PdfPage`]). L'ajustement proportionnel et centré des
//! fonds non A4 est porté par le modèle ([`Page::background_fit`], F-10).

use std::path::Path;

use pdfium_render::prelude::*;

use crate::model::document::SourceId;
use crate::model::error::ModelError;
use crate::model::page::{Page, PageBackground, PageSize};

/// Erreurs typées de l'import et du rendu PDF (AGENTS.md : pas de panique).
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("bibliothèque PDFium introuvable : {detail}")]
    Library { detail: String },

    #[error("fichier « {path} » inaccessible : {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error(transparent)]
    Pdfium(#[from] PdfiumError),

    #[error(transparent)]
    Model(#[from] ModelError),
}

/// Résultat d'un rendu : pixels RGBA bruts (le passage en image Slint se fait
/// dans `ui/`, pour que `pdf/` reste indépendant de l'interface).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Charge la bibliothèque PDFium. **Au plus une fois par processus** (les
/// liaisons PDFium sont globales dans `pdfium-render`).
///
/// Chaîne de recherche : variable d'environnement `CAHIERPDF_PDFIUM` (chemin
/// exact de la bibliothèque), puis `./libpdfium.so`, puis bibliothèque système.
pub fn bind() -> Result<Pdfium, PdfError> {
    let bindings = match std::env::var("CAHIERPDF_PDFIUM") {
        Ok(chemin) => Pdfium::bind_to_library(&chemin).map_err(|source| PdfError::Library {
            detail: format!("{chemin} : {source}"),
        })?,
        Err(_) => Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|source| PdfError::Library {
                detail: source.to_string(),
            })?,
    };
    Ok(Pdfium::new(bindings))
}

/// Empreinte d'identification d'un PDF importé (§8).
///
/// J2 : nom de fichier et taille, suffisants pour distinguer les sources ;
/// J4 stocke le binaire et précisera l'empreinte (D-06).
pub fn fingerprint(path: &Path) -> Result<String, PdfError> {
    let metadata = std::fs::metadata(path).map_err(|source| PdfError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(format!("{name}:{}", metadata.len()))
}

/// Importe toutes les pages d'un PDF comme pages du cahier (F-03, F-04).
///
/// Chaque page source devient une [`Page`] A4 : portrait ou paysage selon sa
/// taille d'origine (orientation conservée, F-10), fond référencé par
/// `source` + numéro de page + taille d'origine. Les pages non A4 seront
/// ajustées à l'affichage par [`Page::background_fit`] (F-10).
///
/// `source` doit être l'identifiant retourné par `Notebook::add_source` pour
/// ce même fichier ; cette fonction ne fait aucun I/O sur le cahier.
pub fn import_pages(pdfium: &Pdfium, path: &Path, source: SourceId) -> Result<Vec<Page>, PdfError> {
    let document = pdfium.load_pdf_from_file(path, None)?;
    let count = document.pages().len();
    let mut pages = Vec::with_capacity(count.max(0) as usize);
    for index in 0..count {
        let pdf_page = document.pages().get(index)?;
        let source_size = PageSize::new(pdf_page.width().value, pdf_page.height().value)?;
        pages.push(Page::new(
            source_size.orientation(),
            PageBackground::PdfPage {
                source,
                page_index: index as u32,
                source_size,
            },
        ));
    }
    Ok(pages)
}

/// Rend une page PDF en pixels RGBA aux dimensions exactes demandées.
///
/// Comme l'exige PDFium, l'appelant conserve le ratio de la page
/// (`spikes/pdf-rendu`, `docs/mesures.md`).
pub fn render_page(
    pdfium: &Pdfium,
    path: &Path,
    page_index: u32,
    width_px: u32,
    height_px: u32,
) -> Result<Raster, PdfError> {
    let document = pdfium.load_pdf_from_file(path, None)?;
    // `PdfPageIndex = c_int` et `Pixels = i32` (pdfium-render 0.9.4).
    let page = document.pages().get(page_index as i32)?;
    let bitmap = page.render(width_px as i32, height_px as i32, None)?;
    Ok(Raster {
        width: width_px,
        height: height_px,
        rgba: bitmap.as_rgba_bytes(),
    })
}

/// Rend des pages PDF en vignettes carrées de `max_px`, en conservant le ratio
/// de chaque page (F-09 : panneau de miniatures). Une seule ouverture du
/// document pour tout le lot (esprit NF-04).
pub fn render_thumbnails(
    pdfium: &Pdfium,
    path: &Path,
    page_indices: &[u32],
    max_px: u32,
) -> Result<Vec<Raster>, PdfError> {
    let document = pdfium.load_pdf_from_file(path, None)?;
    let mut rasters = Vec::with_capacity(page_indices.len());
    for &page_index in page_indices {
        let page = document.pages().get(page_index as i32)?;
        let (width, height) = (page.width().value, page.height().value);
        let scale = (max_px as f32 / width).min(max_px as f32 / height);
        let width_px = ((width * scale).round() as u32).max(1);
        let height_px = ((height * scale).round() as u32).max(1);
        let bitmap = page.render(width_px as i32, height_px as i32, None)?;
        rasters.push(Raster {
            width: width_px,
            height: height_px,
            rgba: bitmap.as_rgba_bytes(),
        });
    }
    Ok(rasters)
}
