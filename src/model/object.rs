//! Objets posés sur un calque : traits et formes (CdC v0.2 §8).
//!
//! Les formes stockent leurs paramètres (type, largeur, hauteur, angle),
//! jamais un tracé (AGENTS.md). Les valeurs non finies ou hors bornes sont
//! refusées à la construction : ce sont des types validés.

use crate::model::error::{ModelError, Result};

/// Couleur RGBA (F-34, F-45) : composantes 0-255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Outil ayant produit un trait (F-33).
///
/// La gomme n'est pas un outil de tracé : c'est une action de suppression
/// d'objets, matérialisée par une commande (`RemoveObjects`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeTool {
    /// Stylo : trait opaque.
    #[default]
    Pen,
    /// Surligneur : trait large et semi-transparent (F-33).
    Highlighter,
}

/// Point d'un trait, en points PDF dans le repère de la page (§8).
///
/// La pression est `None` lorsque le matériel ne la fournit pas (R-1) ;
/// lorsqu'elle est fournie, elle est bornée à 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePoint {
    x: f32,
    y: f32,
    pressure: Option<f32>,
}

impl StrokePoint {
    /// Valide les coordonnées (finies) et la pression (0.0..=1.0 si fournie).
    pub fn new(x: f32, y: f32, pressure: Option<f32>) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ModelError::NonFiniteCoordinate { x, y });
        }
        if let Some(value) = pressure
            && !(0.0..=1.0).contains(&value)
        {
            return Err(ModelError::InvalidPressure { value });
        }
        Ok(Self { x, y, pressure })
    }

    pub fn x(self) -> f32 {
        self.x
    }

    pub fn y(self) -> f32 {
        self.y
    }

    pub fn pressure(self) -> Option<f32> {
        self.pressure
    }
}

/// Un trait manuscrit (§8) : points, outil, couleur, épaisseur.
///
/// L'épaisseur est exprimée en points PDF, comme les coordonnées.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    points: Vec<StrokePoint>,
    tool: StrokeTool,
    color: Color,
    width: f32,
}

impl Stroke {
    /// Crée un trait vide ; l'épaisseur doit être finie et strictement positive.
    pub fn new(tool: StrokeTool, color: Color, width: f32) -> Result<Self> {
        if !width.is_finite() || width <= 0.0 {
            return Err(ModelError::InvalidWidth { value: width });
        }
        Ok(Self {
            points: Vec::new(),
            tool,
            color,
            width,
        })
    }

    pub fn push_point(&mut self, point: StrokePoint) {
        self.points.push(point);
    }

    pub fn points(&self) -> &[StrokePoint] {
        &self.points
    }

    pub fn tool(&self) -> StrokeTool {
        self.tool
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn width(&self) -> f32 {
        self.width
    }
}

/// Type de forme (F-40, v1 : ligne, flèche, rectangle, ellipse).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Line,
    Arrow,
    Rectangle,
    Ellipse,
}

/// Boîte englobante d'une forme, avant rotation (§8) : points PDF.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Bounds {
    /// Valide la boîte : coordonnées finies, largeur et hauteur finies > 0.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ModelError::NonFiniteCoordinate { x, y });
        }
        if !width.is_finite() || width <= 0.0 {
            return Err(ModelError::InvalidWidth { value: width });
        }
        if !height.is_finite() || height <= 0.0 {
            return Err(ModelError::InvalidWidth { value: height });
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn x(self) -> f32 {
        self.x
    }

    pub fn y(self) -> f32 {
        self.y
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn height(self) -> f32 {
        self.height
    }
}

/// Style de contour (F-45) : couleur et épaisseur en points PDF.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    color: Color,
    width: f32,
}

impl StrokeStyle {
    pub fn new(color: Color, width: f32) -> Result<Self> {
        if !width.is_finite() || width <= 0.0 {
            return Err(ModelError::InvalidWidth { value: width });
        }
        Ok(Self { color, width })
    }

    pub fn color(self) -> Color {
        self.color
    }

    pub fn width(self) -> f32 {
        self.width
    }
}

/// Forme paramétrique (§8) : type, boîte englobante, rotation, contour,
/// remplissage. Stub pour J3 : aucune transformation n'est implémentée ici.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    kind: ShapeKind,
    bounds: Bounds,
    /// Angle de rotation en radians, sens direct (anti-horaire),
    /// autour du centre de la boîte englobante.
    rotation: f32,
    stroke: StrokeStyle,
    fill: Option<Color>,
}

impl Shape {
    /// Crée une forme ; l'angle doit être fini.
    pub fn new(
        kind: ShapeKind,
        bounds: Bounds,
        rotation: f32,
        stroke: StrokeStyle,
        fill: Option<Color>,
    ) -> Result<Self> {
        if !rotation.is_finite() {
            return Err(ModelError::InvalidAngle { value: rotation });
        }
        Ok(Self {
            kind,
            bounds,
            rotation,
            stroke,
            fill,
        })
    }

    pub fn kind(&self) -> ShapeKind {
        self.kind
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn stroke(&self) -> StrokeStyle {
        self.stroke
    }

    pub fn fill(&self) -> Option<Color> {
        self.fill
    }
}

/// Objet posé sur un calque (§8). `Texte` et `Image` viendront plus tard.
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Stroke(Stroke),
    Shape(Shape),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stroke_point_rejects_pressure_out_of_range() {
        assert!(matches!(
            StrokePoint::new(10.0, 10.0, Some(1.5)),
            Err(ModelError::InvalidPressure { .. })
        ));
        assert!(matches!(
            StrokePoint::new(10.0, 10.0, Some(-0.1)),
            Err(ModelError::InvalidPressure { .. })
        ));
        assert!(StrokePoint::new(10.0, 10.0, Some(0.0)).is_ok());
        assert!(StrokePoint::new(10.0, 10.0, Some(1.0)).is_ok());
        assert!(StrokePoint::new(10.0, 10.0, None).is_ok());
    }

    #[test]
    fn stroke_point_rejects_non_finite_coordinates() {
        assert!(matches!(
            StrokePoint::new(f32::NAN, 0.0, None),
            Err(ModelError::NonFiniteCoordinate { .. })
        ));
        assert!(matches!(
            StrokePoint::new(0.0, f32::INFINITY, None),
            Err(ModelError::NonFiniteCoordinate { .. })
        ));
    }

    #[test]
    fn stroke_rejects_non_positive_width() {
        assert!(matches!(
            Stroke::new(StrokeTool::Pen, Color::BLACK, 0.0),
            Err(ModelError::InvalidWidth { .. })
        ));
        assert!(matches!(
            Stroke::new(StrokeTool::Pen, Color::BLACK, -2.0),
            Err(ModelError::InvalidWidth { .. })
        ));
        assert!(Stroke::new(StrokeTool::Pen, Color::BLACK, 0.5).is_ok());
    }

    #[test]
    fn shape_rejects_invalid_geometry() {
        let stroke = StrokeStyle::new(Color::BLACK, 1.0).expect("épaisseur valide");
        let bounds = Bounds::new(0.0, 0.0, 100.0, 50.0).expect("boîte valide");
        assert!(matches!(
            Shape::new(ShapeKind::Rectangle, bounds, f32::NAN, stroke, None),
            Err(ModelError::InvalidAngle { .. })
        ));
        assert!(matches!(
            Bounds::new(0.0, 0.0, -1.0, 50.0),
            Err(ModelError::InvalidWidth { .. })
        ));
        assert!(
            Shape::new(
                ShapeKind::Rectangle,
                bounds,
                0.0,
                stroke,
                Some(Color::WHITE)
            )
            .is_ok()
        );
    }
}
