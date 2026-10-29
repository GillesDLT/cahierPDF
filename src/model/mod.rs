//! Modèle de données du cahier (CdC v0.2 §8).
//!
//! Pur : ni UI, ni I/O, aucune dépendance matérielle. Toute modification du
//! document passe par une [`command::Command`] appliquée via un
//! [`history::UndoStack`] (AGENTS.md, F-35).
//!
//! Repère : points PDF dans le repère de la page A4 — 595 × 842 en portrait,
//! 842 × 595 en paysage — indépendant du zoom (§8).
#![forbid(unsafe_code)]

pub mod command;
pub mod document;
pub mod error;
pub mod history;
pub mod layer;
pub mod object;
pub mod page;

pub use command::{
    AddObject, Command, DuplicatePage, InsertPages, MovePage, RemoveObjects, RemovePages,
};
pub use document::{FORMAT_VERSION, Metadata, Notebook, PdfSource, SourceId};
pub use error::{ModelError, Result};
pub use history::UndoStack;
pub use layer::{Layer, LayerRef};
pub use object::{
    Bounds, Color, Object, Shape, ShapeKind, Stroke, StrokePoint, StrokeStyle, StrokeTool,
};
pub use page::{A4_HEIGHT_PT, A4_WIDTH_PT, Fit, Orientation, Page, PageBackground, PageSize};
