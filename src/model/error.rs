//! Erreurs typées du modèle (AGENTS.md : pas de panique hors tests).

use std::result;

/// Résultat des opérations du modèle.
pub type Result<T> = result::Result<T, ModelError>;

/// Erreurs typées du modèle.
///
/// Aucune variante ne doit être produite par un `unwrap()`/`expect()` :
/// elles sont retournées par des constructeurs ou des opérations validées.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("page {index} hors limites ({count} page(s))")]
    PageOutOfRange { index: usize, count: usize },

    #[error("calque {index} hors limites ({count} calque(s))")]
    LayerOutOfRange { index: usize, count: usize },

    #[error("objet {index} hors limites ({count} objet(s))")]
    ObjectOutOfRange { index: usize, count: usize },

    #[error("index d'objet dupliqué dans la commande : {0}")]
    DuplicateObjectIndex(usize),

    #[error("index de page dupliqué dans la commande : {0}")]
    DuplicatePageIndex(usize),

    #[error("un cahier contient au moins une page : la dernière page ne peut pas être supprimée")]
    LastPage,

    #[error(
        "taille de page ({width} × {height}) invalide : attendu des nombres finis strictement positifs"
    )]
    InvalidPageSize { width: f32, height: f32 },

    #[error("commande « {command} » : le cahier ne correspond pas à l'état attendu")]
    CommandStateMismatch { command: &'static str },

    #[error("pression {value} hors bornes (attendu 0.0..=1.0)")]
    InvalidPressure { value: f32 },

    #[error("épaisseur ou dimension {value} invalide (attendu un nombre fini strictement positif)")]
    InvalidWidth { value: f32 },

    #[error("coordonnée non finie ({x}, {y})")]
    NonFiniteCoordinate { x: f32, y: f32 },

    #[error("angle {value} non fini")]
    InvalidAngle { value: f32 },

    #[error("un calque ne peut pas être supprimé : une page contient au moins un calque")]
    LastLayer,

    #[error("commande « {command} » annulée ou rétablie avant d'avoir été appliquée")]
    CommandNotApplied { command: &'static str },

    #[error("commande « {command} » appliquée deux fois sans annulation")]
    CommandAlreadyApplied { command: &'static str },

    #[error("aucune commande à annuler")]
    NothingToUndo,

    #[error("aucune commande à rétablir")]
    NothingToRedo,
}
