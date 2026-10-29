//! Générateur de PDF minimal pour les tests d'import (J2).
//!
//! Écrit un PDF 1.4 valide avec une table xref calculée : pages blanches de
//! tailles choisies. Ce module est **réservé aux tests** ; la production
//! n'écrit de PDF qu'en J4 (export vectoriel).

use std::io::Result;
use std::path::Path;

/// Écrit dans `path` un PDF de pages blanches de tailles `page_sizes`
/// (largeur × hauteur en points PDF). Renvoie la taille du fichier.
pub fn write_minimal_pdf(path: &Path, page_sizes: &[(f32, f32)]) -> Result<u64> {
    let count = page_sizes.len();
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(count + 2);

    offsets.push(out.len());
    out.push_str("1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");

    let kids = (0..count)
        .map(|index| format!("{} 0 R", 3 + index))
        .collect::<Vec<_>>()
        .join(" ");
    offsets.push(out.len());
    out.push_str(&format!(
        "2 0 obj << /Type /Pages /Kids [{kids}] /Count {count} >> endobj\n"
    ));

    for (index, (width, height)) in page_sizes.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!(
            "{} 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] >> endobj\n",
            3 + index
        ));
    }

    let xref_offset = out.len();
    let size = count + 3;
    out.push_str(&format!("xref\n0 {size}\n"));
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        // Chaque entrée d'index fait exactement 20 octets (spécification PDF).
        out.push_str(&format!("{offset:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer << /Size {size} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
    ));

    std::fs::write(path, out)?;
    std::fs::metadata(path).map(|metadata| metadata.len())
}
