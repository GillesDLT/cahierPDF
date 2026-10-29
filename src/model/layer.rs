//! Calque : conteneur d'objets, du fond vers le haut (CdC v0.2 §8, D-04).

use crate::model::error::{ModelError, Result};
use crate::model::object::Object;

/// Adresse d'un calque dans le cahier : index de page, puis index de calque.
///
/// Les index ne sont valides que pour la séquence stricte de commandes du
/// `UndoStack` (voir `Command`) : aucune réorganisation des pages n'est
/// prévue en J1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerRef {
    pub page: usize,
    pub layer: usize,
}

impl LayerRef {
    pub const fn new(page: usize, layer: usize) -> Self {
        Self { page, layer }
    }
}

/// Un calque de page (§8) : liste d'objets, index 0 = fond.
///
/// Chaque page possède au moins un calque (D-04) ; l'interface des calques
/// arrive en v2 (F-11), mais le modèle les prévoit dès la v1.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    objects: Vec<Object>,
    visible: bool,
}

impl Layer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn object(&self, index: usize) -> Result<&Object> {
        self.objects.get(index).ok_or(ModelError::ObjectOutOfRange {
            index,
            count: self.objects.len(),
        })
    }

    /// Ajoute un objet en fin de pile (ordre de création = ordre d'affichage).
    pub fn push_object(&mut self, object: Object) -> usize {
        self.objects.push(object);
        self.objects.len() - 1
    }

    /// Insère un objet à `index` (0 = fond) ; utilisé par l'annulation d'une
    /// suppression. Refuse `index > len`.
    pub fn insert_object(&mut self, index: usize, object: Object) -> Result<()> {
        if index > self.objects.len() {
            return Err(ModelError::ObjectOutOfRange {
                index,
                count: self.objects.len(),
            });
        }
        self.objects.insert(index, object);
        Ok(())
    }

    /// Retire et renvoie l'objet à `index`.
    pub fn remove_object(&mut self, index: usize) -> Result<Object> {
        if index >= self.objects.len() {
            return Err(ModelError::ObjectOutOfRange {
                index,
                count: self.objects.len(),
            });
        }
        Ok(self.objects.remove(index))
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            visible: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::object::{Color, Stroke, StrokeTool};

    fn stroke() -> Object {
        Object::Stroke(Stroke::new(StrokeTool::Pen, Color::BLACK, 2.0).expect("épaisseur valide"))
    }

    #[test]
    fn insert_and_remove_object_roundtrip() {
        let mut layer = Layer::new();
        let index = layer.push_object(stroke());
        assert_eq!(index, 0);
        assert_eq!(layer.object_count(), 1);
        let removed = layer.remove_object(0).expect("index valide");
        assert_eq!(layer.object_count(), 0);
        layer.insert_object(0, removed).expect("index valide");
        assert_eq!(layer.object_count(), 1);
    }

    #[test]
    fn out_of_range_access_is_typed_error() {
        let mut layer = Layer::new();
        assert!(matches!(
            layer.remove_object(0),
            Err(ModelError::ObjectOutOfRange { .. })
        ));
        assert!(matches!(
            layer.insert_object(1, stroke()),
            Err(ModelError::ObjectOutOfRange { .. })
        ));
        assert_eq!(layer.object_count(), 0);
    }

    #[test]
    fn default_layer_is_visible() {
        assert!(Layer::default().is_visible());
    }
}
