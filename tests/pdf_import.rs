//! Chaîne complète d'import PDF (F-03, F-04, F-10) et de rendu (F-09).
//!
//! Nécessite la bibliothèque PDFium à l'exécution (voir `pdf::bind`) ;
//! sans elle, le test s'ignore avec un message explicite et la preuve réelle
//! reste documentée dans `docs/mesures.md`. Une seule fonction de test :
//! les liaisons PDFium sont globales (une par processus).

mod common;

use std::path::PathBuf;

use cahierpdf::model::{InsertPages, Notebook, Orientation, PageBackground, UndoStack};

fn metadata() -> cahierpdf::model::Metadata {
    cahierpdf::model::Metadata {
        title: "Test J2".to_string(),
        created_at: 0,
        updated_at: 0,
        format_version: cahierpdf::model::FORMAT_VERSION,
    }
}

#[test]
fn import_render_and_undo_pdf_pages() {
    let Some(pdfium) = (match cahierpdf::pdf::bind() {
        Ok(pdfium) => Some(pdfium),
        Err(error) => {
            eprintln!("[ignore] bibliothèque PDFium absente : {error} (voir docs/mesures.md §J2)");
            None
        }
    }) else {
        return;
    };

    // Fixture reproductible : A4 portrait, letter portrait, A4 paysage.
    let path = std::env::temp_dir().join(format!("cahierpdf-j2-{}.pdf", std::process::id()));
    let expected_sizes = [(595.0f32, 842.0f32), (612.0, 792.0), (842.0, 595.0)];
    let file_size =
        common::write_minimal_pdf(&path, &expected_sizes).expect("écriture du PDF de test");

    // F-03 : l'empreinte identifie la source.
    let fingerprint = cahierpdf::pdf::fingerprint(&path).expect("empreinte");
    assert!(fingerprint.contains(&file_size.to_string()));

    // Import : chaque page source devient une page du cahier (F-03).
    let mut notebook = Notebook::new(metadata());
    let source = notebook.add_source(fingerprint);
    let pages = cahierpdf::pdf::import_pages(&pdfium, &path, source).expect("import");
    assert_eq!(pages.len(), 3);

    // F-10 : orientation conservée par la taille d'origine.
    assert_eq!(pages[0].orientation(), Orientation::Portrait);
    assert_eq!(pages[1].orientation(), Orientation::Portrait);
    assert_eq!(pages[2].orientation(), Orientation::Landscape);

    // F-10 : ajustement proportionnel et centré, sans recadrage.
    let fit_a4 = pages[0].background_fit().expect("ajustement");
    assert!((fit_a4.scale() - 1.0).abs() < 1e-6);
    let fit_letter = pages[1].background_fit().expect("ajustement");
    assert!(fit_letter.scale() > 0.0);
    assert!(612.0 * fit_letter.scale() <= 595.0 + 1e-3);
    assert!(792.0 * fit_letter.scale() <= 842.0 + 1e-3);
    assert!(fit_letter.dx() >= 0.0 && fit_letter.dy() >= 0.0);

    // F-04 : insertion dans un cahier existant à une position choisie,
    // annulable (commande réversible).
    let mut stack = UndoStack::new(10);
    stack
        .execute(InsertPages::new(1, pages), &mut notebook)
        .expect("insertion");
    assert_eq!(notebook.page_count(), 4);
    assert!(matches!(
        notebook.page(1).expect("page 1").background(),
        PageBackground::PdfPage { page_index: 0, .. }
    ));
    stack.undo(&mut notebook).expect("annulation");
    assert_eq!(notebook.page_count(), 1);
    stack.redo(&mut notebook).expect("rétablissement");
    assert_eq!(notebook.page_count(), 4);

    // Rendu (F-09) : dimensions demandées, ratio conservé, RGBA complet.
    let raster = cahierpdf::pdf::render_page(&pdfium, &path, 2, 200, 141).expect("rendu page");
    assert_eq!((raster.width, raster.height), (200, 141));
    assert_eq!(raster.rgba.len(), 200 * 141 * 4);

    let thumbs =
        cahierpdf::pdf::render_thumbnails(&pdfium, &path, &[0, 1, 2], 64).expect("vignettes");
    assert_eq!(thumbs.len(), 3);
    for thumb in &thumbs {
        assert!(thumb.width <= 64 && thumb.height <= 64);
        assert!(thumb.width >= 1 && thumb.height >= 1);
        assert_eq!(thumb.rgba.len(), (thumb.width * thumb.height * 4) as usize);
    }

    std::fs::remove_file(&path).ok();
}

#[test]
fn import_of_missing_file_is_typed_error() {
    let path = PathBuf::from("/non/existant/cahierpdf-test.pdf");
    assert!(cahierpdf::pdf::fingerprint(&path).is_err());
}
