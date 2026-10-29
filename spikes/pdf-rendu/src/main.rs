//! Spike jetable T2 — rendu d'une page PDF (J0, cf. ../../AGENTS.md).
//!
//! Périmètre strict :
//! - ouverture d'un PDF (de 100 pages pour les mesures NF-01/NF-04) ;
//! - rendu d'une page en bitmap via PDFium (`pdfium-render`) ;
//! - affichage de la page dans une fenêtre Slint, avec repli sur une image
//!   PNG écrite sur disque si l'affichage est impossible (environnement sans
//!   serveur d'affichage) ;
//! - mesures : RSS après ouverture, temps jusqu'à la première page, temps de
//!   rendu d'une page. Sorties brutes sur stdout, consignées dans
//!   ../../docs/mesures.md.
//!
//! Le PDF source n'est jamais modifié (lecture seule, NF-07). Aucun stylet ici
//! (spike `stylo-slint`), aucun choix de moteur de canvas (R-4 tranché ailleurs).
//!
//! Bibliothèque PDFium : chargée à l'exécution par `pdfium-render`
//! (`libloading`), jamais liée à la compilation. Voir `--aide` et
//! ../../docs/mesures.md pour les chemins de recherche.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use pdfium_render::prelude::*;

slint::slint! {
    export component MainWindow inherits Window {
        title: "cahierPDF - spike T2 pdf-rendu";
        width: 800px;
        height: 600px;
        background: #1e1e2e;

        // Page PDF rendue, fournie depuis Rust (slint::Image, buffer RGBA).
        in property <image> page;

        Image {
            source: root.page;
            width: 100%;
            height: 100%;
            image-fit: contain;
        }
    }
}

/// Arguments de la ligne de commande (analyse testable sans PDF ni affichage).
#[derive(Debug, PartialEq)]
struct Args {
    pdf: PathBuf,
    /// Index de page 0-based.
    page: i32,
    /// Pixels par point PDF (1.0 = 72 ppp, 2.0 = 144 ppp).
    echelle: f32,
    png: PathBuf,
    sans_fenetre: bool,
    /// Durée d'affichage de la fenêtre avant fermeture automatique (s).
    duree_fenetre: f64,
    aide: bool,
}

const AIDE: &str = "usage : pdf-rendu <fichier.pdf> [options]
  --page N             index de page 0-based (défaut : 0)
  --echelle S          pixels par point PDF (défaut : 2.0)
  --png CHEMIN         PNG de sortie (défaut : sorties/page.png)
  --sans-fenetre       n'ouvre pas de fenêtre Slint (PNG seul)
  --duree-fenetre S    durée d'affichage de la fenêtre en s (défaut : 4)
  --aide               affiche ce texte

Environnement :
  PDF_RENDU_PDFIUM     chemin explicite de libpdfium.so ;
                       sinon recherche dans ./ puis bibliothèque système.";

fn parse_args<I: IntoIterator<Item = String>>(iter: I) -> Result<Args, String> {
    let mut args = Args {
        pdf: PathBuf::new(),
        page: 0,
        echelle: 2.0,
        png: PathBuf::from("sorties/page.png"),
        sans_fenetre: false,
        duree_fenetre: 4.0,
        aide: false,
    };
    let mut it = iter.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--page" => {
                let v = it.next().ok_or("--page : valeur manquante")?;
                args.page = v
                    .parse()
                    .map_err(|_| format!("--page : valeur invalide {v}"))?;
            }
            "--echelle" => {
                let v = it.next().ok_or("--echelle : valeur manquante")?;
                args.echelle = v
                    .parse()
                    .map_err(|_| format!("--echelle : valeur invalide {v}"))?;
                if args.echelle.is_nan() || args.echelle <= 0.0 {
                    return Err(format!("--echelle : doit être > 0 (reçu {v})"));
                }
            }
            "--png" => {
                let v = it.next().ok_or("--png : valeur manquante")?;
                args.png = PathBuf::from(v);
            }
            "--sans-fenetre" => args.sans_fenetre = true,
            "--duree-fenetre" => {
                let v = it.next().ok_or("--duree-fenetre : valeur manquante")?;
                args.duree_fenetre = v
                    .parse()
                    .map_err(|_| format!("--duree-fenetre : valeur invalide {v}"))?;
            }
            "--aide" | "-h" | "--help" => args.aide = true,
            _ if a.starts_with('-') => return Err(format!("option inconnue {a}")),
            _ => {
                if !args.pdf.as_os_str().is_empty() {
                    return Err(format!("un seul fichier PDF attendu (reçu en plus {a})"));
                }
                args.pdf = PathBuf::from(a);
            }
        }
    }
    if args.aide {
        return Ok(args);
    }
    if args.pdf.as_os_str().is_empty() {
        return Err("fichier PDF manquant".to_string());
    }
    if args.page < 0 {
        return Err(format!("--page : index négatif {}", args.page));
    }
    Ok(args)
}

/// Dimensions en pixels cibles pour une page de `largeur_pt` x `hauteur_pt`,
/// en conservant le ratio (pdfium-render exige que l'appelant le fasse).
fn pixels_cibles(largeur_pt: f32, hauteur_pt: f32, echelle: f32) -> (i32, i32) {
    let w = (largeur_pt * echelle).round().max(1.0) as i32;
    let h = (hauteur_pt * echelle).round().max(1.0) as i32;
    (w, h)
}

/// Extrait une valeur en kio d'une ligne `VmRSS:` ou `VmHWM:` de /proc/self/status.
fn extraire_kib(contenu: &str, cle: &str) -> Option<u64> {
    contenu
        .lines()
        .find(|l| l.starts_with(cle))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// RSS courant du processus, en kio (source : /proc/self/status).
fn rss_kib() -> Option<u64> {
    let contenu = fs::read_to_string("/proc/self/status").ok()?;
    extraire_kib(&contenu, "VmRSS:")
}

/// RSS maximal du processus depuis son démarrage, en kio.
fn rss_hwm_kib() -> Option<u64> {
    let contenu = fs::read_to_string("/proc/self/status").ok()?;
    extraire_kib(&contenu, "VmHWM:")
}

/// Affiche un point de mesure brut (format libre, consigné tel quel).
fn mesure(nom: &str, valeur: f64, unite: &str) {
    println!("[mesure] {nom}={valeur:.1} {unite}");
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = match parse_args(env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("erreur : {e}\n\n{AIDE}");
            std::process::exit(2);
        }
    };
    if args.aide {
        println!("{AIDE}");
        return Ok(());
    }

    // t0 : départ du processus utile, après analyse de la ligne de commande.
    let t0 = Instant::now();
    let etape = |nom: &str| {
        mesure(nom, t0.elapsed().as_secs_f64() * 1000.0, "ms depuis t0");
    };

    println!("[info] fichier_pdf={}", args.pdf.display());
    println!("[info] page_index={}", args.page);
    println!("[info] echelle_px_par_pt={}", args.echelle);
    println!("[info] pdfium_render=0.9.4 (figé dans Cargo.lock)");
    mesure("rss_debut", rss_kib().unwrap_or(0) as f64, "kio");

    // --- Liaison à la bibliothèque PDFium (chargée à l'exécution) -----------
    let t_bind = Instant::now();
    let bindings = match env::var("PDF_RENDU_PDFIUM") {
        Ok(chemin) => Pdfium::bind_to_library(&chemin)
            .map_err(|e| format!("liaison impossible à PDF_RENDU_PDFIUM={chemin} : {e}"))?,
        Err(_) => Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| format!("liaison impossible à libpdfium.so (./ puis système) : {e}"))?,
    };
    mesure(
        "t_bind_pdfium",
        t_bind.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );
    let pdfium = Pdfium::new(bindings);

    // --- Ouverture du document (lecture seule, paresseuse) -----------------
    let t_ouverture = Instant::now();
    let document = pdfium.load_pdf_from_file(&args.pdf, None)?;
    mesure(
        "t_ouverture_document",
        t_ouverture.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );
    println!("[info] nb_pages={}", document.pages().len());
    mesure("rss_apres_ouverture", rss_kib().unwrap_or(0) as f64, "kio");
    etape("t_depuis_t0_apres_ouverture");

    // --- Rendu de la première page (proxy de NF-04 : première page prête) --
    let t_premiere = Instant::now();
    let page_0 = document.pages().first()?;
    let (w0, h0) = pixels_cibles(page_0.width().value, page_0.height().value, args.echelle);
    let bitmap_0 = page_0.render(w0, h0, None)?;
    mesure(
        "t_premiere_page_rendue",
        t_premiere.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );
    drop(bitmap_0);
    etape("t_depuis_t0_premiere_page_rendue");

    // --- Rendu de la page demandée (mesure de rendu d'une page) -----------
    let page = document.pages().get(args.page)?;
    println!(
        "[info] page_taille_pt={}x{}",
        page.width().value,
        page.height().value
    );
    let (w_px, h_px) = pixels_cibles(page.width().value, page.height().value, args.echelle);
    println!("[info] page_cible_px={w_px}x{h_px}");

    let t_rendu = Instant::now();
    let bitmap = page.render(w_px, h_px, None)?;
    mesure(
        "t_rendu_page",
        t_rendu.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );

    // Second rendu de la même page : coût marginal après initialisation
    // (polices, caches internes de PDFium).
    let t_rendu2 = Instant::now();
    let bitmap2 = page.render(w_px, h_px, None)?;
    mesure(
        "t_rendu_page_2e",
        t_rendu2.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );
    drop(bitmap2);
    mesure("rss_apres_rendu", rss_kib().unwrap_or(0) as f64, "kio");

    // --- PNG sur disque (repli d'affichage, et preuve du rendu) -----------
    let t_png = Instant::now();
    if let Some(parent) = args.png.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    bitmap
        .as_image()?
        .save(&args.png)
        .map_err(|e| format!("écriture PNG {} : {e}", args.png.display()))?;
    println!("[info] png_ecrit={}", args.png.display());
    mesure(
        "t_ecriture_png",
        t_png.elapsed().as_secs_f64() * 1000.0,
        "ms",
    );

    // --- Fenêtre Slint ----------------------------------------------------
    if args.sans_fenetre {
        println!("[info] fenetre=desactivee (--sans-fenetre)");
    } else {
        // Buffer RGBA pour Slint (copie normalisée du bitmap PDFium).
        let rgba = bitmap.as_rgba_bytes();
        let mut pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(
            bitmap.width() as u32,
            bitmap.height() as u32,
        );
        let (paquets, _reste) = rgba.as_chunks::<4>();
        for (dst, src) in pixels.make_mut_slice().iter_mut().zip(paquets) {
            *dst = slint::Rgba8Pixel {
                r: src[0],
                g: src[1],
                b: src[2],
                a: src[3],
            };
        }
        let image = slint::Image::from_rgba8(pixels);

        match MainWindow::new() {
            Ok(window) => {
                window.set_page(image);
                let t_fenetre = Instant::now();
                window.show()?;
                mesure(
                    "t_fenetre_show",
                    t_fenetre.elapsed().as_secs_f64() * 1000.0,
                    "ms",
                );
                etape("t_depuis_t0_fenetre_visible");
                println!(
                    "[info] fenetre=ouverte fermeture_auto_s={}",
                    args.duree_fenetre
                );
                // Laisse le temps à une capture extérieure (import/xwd) puis
                // ferme la boucle d'événements. La présentation effective de la
                // première trame par le compositeur n'est pas observable depuis
                // Slint (à vérifier) : t_fenetre_show est un proxy.
                let duree = Duration::from_secs_f64(args.duree_fenetre);
                slint::Timer::single_shot(duree, || {
                    let _ = slint::quit_event_loop();
                });
                window.run()?;
            }
            Err(e) => {
                println!("[avertissement] fenetre=impossible ({e}) ; PNG seul");
            }
        }
    }

    mesure("t_total", t0.elapsed().as_secs_f64() * 1000.0, "ms");
    mesure("rss_fin", rss_kib().unwrap_or(0) as f64, "kio");
    mesure("rss_hwm", rss_hwm_kib().unwrap_or(0) as f64, "kio");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(liste: &[&str]) -> Result<Args, String> {
        parse_args(liste.iter().map(|s| s.to_string()))
    }

    // Analyse de la ligne de commande : aucun PDF, aucun affichage requis.
    #[test]
    fn parse_args_valeurs_par_defaut() {
        let a = args(&["doc.pdf"]).unwrap();
        assert_eq!(a.pdf, PathBuf::from("doc.pdf"));
        assert_eq!(a.page, 0);
        assert_eq!(a.echelle, 2.0);
        assert_eq!(a.png, PathBuf::from("sorties/page.png"));
        assert!(!a.sans_fenetre);
    }

    #[test]
    fn parse_args_options_et_erreurs() {
        let a = args(&[
            "doc.pdf",
            "--page",
            "42",
            "--echelle",
            "1.5",
            "--sans-fenetre",
        ])
        .unwrap();
        assert_eq!(a.page, 42);
        assert_eq!(a.echelle, 1.5);
        assert!(a.sans_fenetre);
        assert!(args(&[]).is_err());
        assert!(args(&["a.pdf", "b.pdf"]).is_err());
        assert!(args(&["a.pdf", "--page", "-1"]).is_err());
        assert!(args(&["a.pdf", "--echelle", "0"]).is_err());
        assert!(args(&["a.pdf", "--inconnue"]).is_err());
    }

    // Lecture de /proc/self/status : test sur contenu figé (pas de /proc requis).
    #[test]
    fn extraire_kib_depuis_status() {
        let contenu = "Name:\tpdf-rendu\nVmHWM:\t  123456 kB\nVmRSS:\t   23456 kB\n";
        assert_eq!(extraire_kib(contenu, "VmRSS:"), Some(23456));
        assert_eq!(extraire_kib(contenu, "VmHWM:"), Some(123456));
        assert_eq!(extraire_kib(contenu, "VmSwap:"), None);
    }

    // Dimensions cibles : ratio conservé, minimum 1 px.
    #[test]
    fn pixels_cibles_conservent_le_ratio() {
        assert_eq!(pixels_cibles(595.276, 841.89, 2.0), (1191, 1684));
        assert_eq!(pixels_cibles(10.0, 10.0, 0.001), (1, 1));
    }
}
