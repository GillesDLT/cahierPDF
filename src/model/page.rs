//! Page A4 : format, orientation, fond (CdC v0.2 §8, F-01, D-01, F-10).

use crate::model::document::SourceId;
use crate::model::error::{ModelError, Result};
use crate::model::layer::Layer;

/// Largeur A4 en points PDF, repère portrait.
pub const A4_WIDTH_PT: f32 = 595.0;

/// Hauteur A4 en points PDF, repère portrait.
pub const A4_HEIGHT_PT: f32 = 842.0;

/// Orientation d'une page (F-01, D-01) : conservée par page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// Taille d'une page source PDF, en points PDF (F-10).
///
/// Valeurs finies et strictement positives, validées à la construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    width: f32,
    height: f32,
}

impl PageSize {
    pub fn new(width: f32, height: f32) -> Result<Self> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(ModelError::InvalidPageSize { width, height });
        }
        Ok(Self { width, height })
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn height(self) -> f32 {
        self.height
    }

    /// Orientation d'après cette taille (F-10 : l'orientation est conservée).
    pub fn orientation(self) -> Orientation {
        if self.width > self.height {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }
}

/// Ajustement d'un fond PDF dans la page (F-10) : mise à l'échelle
/// **proportionnelle et centrée**, sans recadrage (D-02).
///
/// Position dans le repère de la page (§8, origine en haut à gauche) :
/// `x_page = dx + scale × x_source`, `y_page = dy + scale × y_source`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    scale: f32,
    dx: f32,
    dy: f32,
}

impl Fit {
    pub fn scale(self) -> f32 {
        self.scale
    }

    pub fn dx(self) -> f32 {
        self.dx
    }

    pub fn dy(self) -> f32 {
        self.dy
    }
}

/// Fond d'une page (§8). Le PDF source n'est jamais modifié (NF-07) :
/// le fond n'est qu'une référence.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PageBackground {
    /// Page blanche.
    #[default]
    Blank,
    /// Page d'un PDF importé : source, numéro de page (index 0-based) et
    /// taille d'origine, nécessaire à l'ajustement F-10.
    PdfPage {
        source: SourceId,
        page_index: u32,
        source_size: PageSize,
    },
}

/// Une page A4 du cahier (F-01).
///
/// Invariants :
/// - le format est toujours A4 ; seule l'orientation change (§8) ;
/// - la page contient au moins un calque (D-04).
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    orientation: Orientation,
    background: PageBackground,
    layers: Vec<Layer>,
}

impl Page {
    /// Crée une page avec son calque par défaut (D-04).
    pub fn new(orientation: Orientation, background: PageBackground) -> Self {
        Self {
            orientation,
            background,
            layers: vec![Layer::default()],
        }
    }

    /// Page blanche A4 (F-05).
    pub fn blank(orientation: Orientation) -> Self {
        Self::new(orientation, PageBackground::Blank)
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    pub fn background(&self) -> &PageBackground {
        &self.background
    }

    /// Taille du repère de la page, en points PDF (§8) :
    /// 595 × 842 en portrait, 842 × 595 en paysage.
    pub fn size(&self) -> (f32, f32) {
        match self.orientation {
            Orientation::Portrait => (A4_WIDTH_PT, A4_HEIGHT_PT),
            Orientation::Landscape => (A4_HEIGHT_PT, A4_WIDTH_PT),
        }
    }

    /// Ajustement du fond dans la page (F-10) ; `None` pour une page blanche.
    pub fn background_fit(&self) -> Option<Fit> {
        let PageBackground::PdfPage { source_size, .. } = self.background else {
            return None;
        };
        let (target_w, target_h) = self.size();
        let scale = (target_w / source_size.width()).min(target_h / source_size.height());
        let dx = (target_w - source_size.width() * scale) / 2.0;
        let dy = (target_h - source_size.height() * scale) / 2.0;
        Some(Fit { scale, dx, dy })
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn layer(&self, index: usize) -> Result<&Layer> {
        self.layers.get(index).ok_or(ModelError::LayerOutOfRange {
            index,
            count: self.layers.len(),
        })
    }

    pub fn layer_mut(&mut self, index: usize) -> Result<&mut Layer> {
        let count = self.layers.len();
        self.layers
            .get_mut(index)
            .ok_or(ModelError::LayerOutOfRange { index, count })
    }

    /// Ajoute un calque (F-11 en v2 pour l'UI ; le modèle les prévoit dès la v1).
    /// Renvoie son index.
    pub fn add_layer(&mut self) -> usize {
        self.layers.push(Layer::default());
        self.layers.len() - 1
    }

    /// Supprime un calque et le renvoie ; refuse de supprimer le dernier (D-04).
    pub fn remove_layer(&mut self, index: usize) -> Result<Layer> {
        if self.layers.len() <= 1 {
            return Err(ModelError::LastLayer);
        }
        let count = self.layers.len();
        if index >= count {
            return Err(ModelError::LayerOutOfRange { index, count });
        }
        Ok(self.layers.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_size_matches_orientation() {
        let portrait = Page::blank(Orientation::Portrait);
        assert_eq!(portrait.size(), (A4_WIDTH_PT, A4_HEIGHT_PT));
        let landscape = Page::blank(Orientation::Landscape);
        assert_eq!(landscape.size(), (A4_HEIGHT_PT, A4_WIDTH_PT));
    }

    #[test]
    fn new_page_has_one_layer() {
        let page = Page::blank(Orientation::Portrait);
        assert_eq!(page.layer_count(), 1);
    }

    #[test]
    fn cannot_remove_last_layer() {
        let mut page = Page::blank(Orientation::Portrait);
        assert!(matches!(page.remove_layer(0), Err(ModelError::LastLayer)));
        assert_eq!(page.layer_count(), 1);
    }

    #[test]
    fn layer_bounds_are_typed_errors() {
        let page = Page::blank(Orientation::Portrait);
        assert!(matches!(
            page.layer(1),
            Err(ModelError::LayerOutOfRange { .. })
        ));
    }

    #[test]
    fn page_size_rejects_invalid_dimensions() {
        for (width, height) in [
            (0.0, 10.0),
            (10.0, -1.0),
            (f32::NAN, 10.0),
            (10.0, f32::INFINITY),
        ] {
            assert!(matches!(
                PageSize::new(width, height),
                Err(ModelError::InvalidPageSize { .. })
            ));
        }
    }

    #[test]
    fn page_size_orientation_follows_aspect() {
        assert_eq!(
            PageSize::new(A4_WIDTH_PT, A4_HEIGHT_PT)
                .expect("taille valide")
                .orientation(),
            Orientation::Portrait
        );
        assert_eq!(
            PageSize::new(A4_HEIGHT_PT, A4_WIDTH_PT)
                .expect("taille valide")
                .orientation(),
            Orientation::Landscape
        );
    }

    fn pdf_background(source_size: PageSize) -> PageBackground {
        PageBackground::PdfPage {
            source: crate::model::document::SourceId::new(0),
            page_index: 0,
            source_size,
        }
    }

    #[test]
    fn blank_page_has_no_background_fit() {
        assert_eq!(Page::blank(Orientation::Portrait).background_fit(), None);
    }

    #[test]
    fn background_fit_is_identity_for_same_size() {
        let source = PageSize::new(A4_WIDTH_PT, A4_HEIGHT_PT).expect("taille valide");
        let page = Page::new(Orientation::Portrait, pdf_background(source));
        let fit = page.background_fit().expect("ajustement");
        assert!((fit.scale() - 1.0).abs() < 1e-6);
        assert!(fit.dx().abs() < 1e-6);
        assert!(fit.dy().abs() < 1e-6);
    }

    #[test]
    fn background_fit_is_proportional_and_centered() {
        // Source 400 × 800 dans A4 portrait (595 × 842) : l'échelle est bornée
        // par la hauteur (842/800 = 1,0525), donc centrée horizontalement.
        let source = PageSize::new(400.0, 800.0).expect("taille valide");
        let page = Page::new(Orientation::Portrait, pdf_background(source));
        let fit = page.background_fit().expect("ajustement");
        assert!((fit.scale() - 842.0 / 800.0).abs() < 1e-6);
        let rendered_w = 400.0 * fit.scale();
        assert!((fit.dx() - (A4_WIDTH_PT - rendered_w) / 2.0).abs() < 1e-3);
        assert!(fit.dy().abs() < 1e-3);
        // Sans recadrage : la source entière tient dans la page (D-02).
        assert!(rendered_w <= A4_WIDTH_PT + 1e-3);
    }

    #[test]
    fn background_fit_keeps_source_inside_landscape_page() {
        let source = PageSize::new(600.0, 300.0).expect("taille valide");
        let page = Page::new(Orientation::Landscape, pdf_background(source));
        let fit = page.background_fit().expect("ajustement");
        let rendered_w = 600.0 * fit.scale();
        let rendered_h = 300.0 * fit.scale();
        assert!(fit.dx() >= 0.0 && fit.dy() >= 0.0);
        assert!(rendered_w <= A4_HEIGHT_PT + 1e-3);
        assert!(rendered_h <= A4_WIDTH_PT + 1e-3);
    }
}
