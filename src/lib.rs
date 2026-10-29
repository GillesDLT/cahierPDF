//! cahierPDF — crate principal.
//!
//! J2 — Pages : [`model`] (données pures, commandes réversibles), [`pdf`]
//! (import PDFium, F-03/F-04) et `ui/` (fenêtre Slint, miniatures F-08/F-09).
//! Les couches `render/` et `io/` viendront aux jalons suivants (AGENTS.md).

pub mod model;
pub mod pdf;
