//! Spike T1 — journalisation générique du pointeur dans le spike Slint de cahierPDF.
//!
//! Périmètre strict de T1 :
//! - la fenêtre T0 avec une zone de dessin vide (un `Rectangle`) ;
//! - journalisation des seuls événements de pointeur fournis par `TouchArea` ;
//! - aucune valeur de pression synthétique, aucun import PDF ;
//! - aucune dépendance hors `slint`.
//!
//! Le choix du moteur de rendu du canvas (tiny-skia ou wgpu + Vello) sera
//! tranché par mesure en J0 (risque R-4), pas ici.

slint::slint! {
    // Fenêtre principale du spike : zone de travail sombre (§6 du cahier des
    // charges) et zone de dessin vide aux proportions A4 (ratio 842/595),
    // en px, sans équivalence en pt.
    export component MainWindow inherits Window {
        title: "cahierPDF - spike T1 stylo-slint";
        width: 800px;
        height: 600px;
        background: #1e1e2e;

        // Zone de dessin T0. Le TouchArea superposé ci-dessous journalise les
        // événements génériques exposés par Slint, pas les données tablet-v2.
        // 420 x 594 px : 594 = arrondi de 420 x 842/595 = 594,4 px (écart de
        // ratio 0,1 %). Valeurs en px, sans équivalence en pt.
        Rectangle {
            width: 420px;
            height: 594px;
            x: (parent.width - self.width) / 2;
            y: (parent.height - self.height) / 2;
            background: #ffffff;
            border-width: 1px;
            border-color: #3a3a4a;
            border-radius: 2px;
        }

        callback pointer-event(event: PointerEvent, x: length, y: length);

        TouchArea {
            width: 420px;
            height: 594px;
            x: (parent.width - self.width) / 2;
            y: (parent.height - self.height) / 2;
            pointer-event(event) => {
                root.pointer-event(event, self.mouse-x, self.mouse-y);
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    window.on_pointer_event(|event, x, y| {
        println!(
            "pointer kind={:?} button={:?} touch_finger_id={} position=({:.1}, {:.1})",
            event.kind, event.button, event.touch_finger_id, x, y
        );
    });
    window.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test structurel hérité de T0 : le type généré par `slint::slint!` existe et
    // implémente `ComponentHandle`. L'instanciation réelle exige un serveur
    // d'affichage ; elle est couverte par le lancement sous Xvfb consigné
    // dans ../../docs/mesures.md, et par les spikes suivants (R-1).
    #[test]
    fn main_window_is_a_component() {
        fn assert_component<T: slint::ComponentHandle>() {}
        assert_component::<MainWindow>();
    }
}
