//! Cahier : métadonnées, sources PDF importées, pages (CdC v0.2 §8).

use crate::model::error::{ModelError, Result};
use crate::model::layer::{Layer, LayerRef};
use crate::model::page::{Orientation, Page};

/// Version du format de données gérée par ce modèle (D-06 : `user_version`).
pub const FORMAT_VERSION: u32 = 1;

/// Identifiant stable d'une source PDF importée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(u64);

impl SourceId {
    /// Construit un identifiant à partir de sa valeur brute.
    ///
    /// Réservé au chargement d'un cahier sauvegardé (J4) et aux tests ;
    /// normalement attribué par [`Notebook::add_source`].
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Source PDF importée : copiée dans le projet et jamais modifiée (NF-07),
/// identifiée par son empreinte (§8). Le contenu binaire reste hors modèle
/// (stocké par `io/` en J4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfSource {
    id: SourceId,
    fingerprint: String,
}

impl PdfSource {
    pub fn new(id: SourceId, fingerprint: impl Into<String>) -> Self {
        Self {
            id,
            fingerprint: fingerprint.into(),
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// Métadonnées du cahier (§8) : titre, dates, version du format.
///
/// Les dates sont en secondes Unix UTC et sont fixées par l'appelant :
/// le modèle ne fait aucun I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub format_version: u32,
}

/// Le cahier complet (§8) : métadonnées, sources PDF, pages.
///
/// Invariants :
/// - au moins une page (F-02 : un nouveau cahier contient une page blanche A4) ;
/// - chaque page contient au moins un calque (D-04, porté par `Page`).
#[derive(Debug, Clone, PartialEq)]
pub struct Notebook {
    meta: Metadata,
    sources: Vec<PdfSource>,
    pages: Vec<Page>,
    next_source_id: u64,
}

impl Notebook {
    /// Nouveau cahier : une page blanche A4 en portrait (F-02).
    pub fn new(meta: Metadata) -> Self {
        Self {
            meta,
            sources: Vec::new(),
            pages: vec![Page::blank(Orientation::Portrait)],
            next_source_id: 0,
        }
    }

    pub fn meta(&self) -> &Metadata {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut Metadata {
        &mut self.meta
    }

    pub fn sources(&self) -> &[PdfSource] {
        &self.sources
    }

    pub fn source(&self, id: SourceId) -> Option<&PdfSource> {
        self.sources.iter().find(|source| source.id() == id)
    }

    /// Enregistre une source PDF importée et lui attribue un identifiant stable.
    pub fn add_source(&mut self, fingerprint: impl Into<String>) -> SourceId {
        let id = SourceId(self.next_source_id);
        self.next_source_id += 1;
        self.sources.push(PdfSource::new(id, fingerprint));
        id
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page(&self, index: usize) -> Result<&Page> {
        self.pages.get(index).ok_or(ModelError::PageOutOfRange {
            index,
            count: self.pages.len(),
        })
    }

    pub fn page_mut(&mut self, index: usize) -> Result<&mut Page> {
        let count = self.pages.len();
        self.pages
            .get_mut(index)
            .ok_or(ModelError::PageOutOfRange { index, count })
    }

    /// Insère les pages `pages` à partir de l'index `at` (0 = avant la
    /// première page, `page_count` = en fin de cahier) (F-04, F-05).
    ///
    /// Atomicité : si `at` est hors limites, le cahier n'est pas modifié et
    /// `pages` est rendu intact (aucune page perdue).
    pub fn insert_pages(&mut self, at: usize, pages: &mut Vec<Page>) -> Result<()> {
        let count = self.pages.len();
        if at > count {
            return Err(ModelError::PageOutOfRange { index: at, count });
        }
        let inserted = std::mem::take(pages);
        self.pages.splice(at..at, inserted);
        Ok(())
    }

    /// Insère une page à `index` (0 = avant la première, `page_count` = en
    /// fin de cahier). En cas d'erreur, le cahier n'est pas modifié.
    pub fn insert_page(&mut self, index: usize, page: Page) -> Result<()> {
        let count = self.pages.len();
        if index > count {
            return Err(ModelError::PageOutOfRange { index, count });
        }
        self.pages.insert(index, page);
        Ok(())
    }

    /// Retire et renvoie les `count` pages contiguës à partir de `at`.
    ///
    /// Refuse de retirer la dernière page du cahier (invariant « au moins une
    /// page », F-02). En cas d'erreur, le cahier n'est pas modifié.
    pub fn remove_pages(&mut self, at: usize, count: usize) -> Result<Vec<Page>> {
        let total = self.pages.len();
        if at + count > total {
            return Err(ModelError::PageOutOfRange {
                index: at + count,
                count: total,
            });
        }
        if total - count == 0 {
            return Err(ModelError::LastPage);
        }
        Ok(self.pages.drain(at..at + count).collect())
    }

    /// Retire et renvoie la page à `index` (F-07). Refuse de retirer la
    /// dernière page du cahier (invariant « au moins une page », F-02).
    pub fn remove_page(&mut self, index: usize) -> Result<Page> {
        let count = self.pages.len();
        if index >= count {
            return Err(ModelError::PageOutOfRange { index, count });
        }
        if count <= 1 {
            return Err(ModelError::LastPage);
        }
        Ok(self.pages.remove(index))
    }

    /// Déplace la page `from` à l'index final `to` (F-08).
    ///
    /// Les index sont validés avant toute mutation ; `from == to` est un no-op.
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<()> {
        let count = self.pages.len();
        if from >= count {
            return Err(ModelError::PageOutOfRange { index: from, count });
        }
        if to >= count {
            return Err(ModelError::PageOutOfRange { index: to, count });
        }
        if from == to {
            return Ok(());
        }
        let page = self.pages.remove(from);
        self.pages.insert(to, page);
        Ok(())
    }

    pub fn layer(&self, target: LayerRef) -> Result<&Layer> {
        self.page(target.page)?.layer(target.layer)
    }

    pub fn layer_mut(&mut self, target: LayerRef) -> Result<&mut Layer> {
        self.page_mut(target.page)?.layer_mut(target.layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Metadata {
        Metadata {
            title: "Test".to_string(),
            created_at: 0,
            updated_at: 0,
            format_version: FORMAT_VERSION,
        }
    }

    #[test]
    fn new_notebook_has_one_blank_page() {
        let notebook = Notebook::new(metadata());
        assert_eq!(notebook.page_count(), 1);
        let page = notebook.page(0).expect("page 0");
        assert_eq!(page.orientation(), Orientation::Portrait);
        assert_eq!(page.size(), (595.0, 842.0));
        assert!(matches!(
            page.background(),
            crate::model::page::PageBackground::Blank
        ));
    }

    #[test]
    fn add_source_assigns_stable_ids() {
        let mut notebook = Notebook::new(metadata());
        let first = notebook.add_source("aaaa");
        let second = notebook.add_source("bbbb");
        assert_ne!(first, second);
        assert_eq!(
            notebook.source(first).expect("source 1").fingerprint(),
            "aaaa"
        );
        assert_eq!(
            notebook.source(second).expect("source 2").fingerprint(),
            "bbbb"
        );
        assert_eq!(notebook.sources().len(), 2);
    }

    #[test]
    fn page_out_of_range_is_typed_error() {
        let notebook = Notebook::new(metadata());
        assert!(matches!(
            notebook.page(1),
            Err(ModelError::PageOutOfRange { .. })
        ));
    }

    #[test]
    fn insert_pages_at_any_position() {
        let mut notebook = Notebook::new(metadata());
        let mut pages = vec![Page::blank(Orientation::Landscape)];
        notebook.insert_pages(0, &mut pages).expect("insertion");
        assert!(
            pages.is_empty(),
            "les pages sont consommées par l'insertion"
        );
        assert_eq!(notebook.page_count(), 2);
        assert_eq!(
            notebook.page(0).expect("page 0").orientation(),
            Orientation::Landscape
        );
        assert_eq!(
            notebook.page(1).expect("page 1").orientation(),
            Orientation::Portrait
        );
    }

    #[test]
    fn insert_pages_out_of_range_keeps_pages() {
        let mut notebook = Notebook::new(metadata());
        let mut pages = vec![Page::blank(Orientation::Portrait)];
        assert!(matches!(
            notebook.insert_pages(5, &mut pages),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(pages.len(), 1, "aucune page n'est perdue");
        assert_eq!(notebook.page_count(), 1);
    }

    #[test]
    fn cannot_remove_the_last_page() {
        let mut notebook = Notebook::new(metadata());
        assert!(matches!(notebook.remove_page(0), Err(ModelError::LastPage)));
        assert!(matches!(
            notebook.remove_pages(0, 1),
            Err(ModelError::LastPage)
        ));
        assert_eq!(notebook.page_count(), 1);
    }

    #[test]
    fn remove_pages_returns_them_in_order() {
        let mut notebook = Notebook::new(metadata());
        let mut extra = vec![
            Page::blank(Orientation::Landscape),
            Page::blank(Orientation::Portrait),
        ];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        let removed = notebook.remove_pages(1, 2).expect("retrait");
        assert_eq!(removed.len(), 2);
        assert_eq!(removed[0].orientation(), Orientation::Landscape);
        assert_eq!(removed[1].orientation(), Orientation::Portrait);
        assert_eq!(notebook.page_count(), 1);
    }

    #[test]
    fn move_page_puts_it_at_target_index() {
        let mut notebook = Notebook::new(metadata());
        let mut extra = vec![
            Page::blank(Orientation::Landscape),
            Page::blank(Orientation::Landscape),
        ];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        // [portrait, paysage, paysage] -> [portrait, paysage, portrait]...
        notebook.move_page(0, 2).expect("déplacement");
        assert_eq!(
            notebook.page(2).expect("page 2").orientation(),
            Orientation::Portrait
        );
        assert_eq!(
            notebook.page(0).expect("page 0").orientation(),
            Orientation::Landscape
        );
        // bornes validées avant mutation
        let mut notebook = Notebook::new(metadata());
        assert!(matches!(
            notebook.move_page(0, 3),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert!(matches!(
            notebook.move_page(2, 0),
            Err(ModelError::PageOutOfRange { .. })
        ));
    }
}
