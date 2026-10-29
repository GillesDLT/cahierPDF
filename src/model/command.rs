//! Commandes réversibles (CdC v0.2 §7.2 : toute modification passe par une
//! commande réversible ; F-35 : annuler/rétablir).

use crate::model::document::Notebook;
use crate::model::error::{ModelError, Result};
use crate::model::layer::LayerRef;
use crate::model::object::Object;
use crate::model::page::Page;

/// Une modification réversible du cahier (AGENTS.md).
///
/// Invariants attendus :
/// - `execute` et `undo` sont atomiques : soit ils réussissent, soit ils
///   renvoient une erreur et laissent le cahier intact ;
/// - `execute` → `undo` → `execute` (rétablissement) ramène le cahier à
///   exactement l'état obtenu après la première exécution ;
/// - une commande ne s'applique qu'à un seul et même [`Notebook`], dans
///   l'ordre strict d'un `UndoStack` : les index enregistrés lors de
///   l'exécution ne valent que pour cette séquence (LIFO) ;
/// - `execute` deux fois sans `undo` (ou l'inverse) est refusé par une
///   erreur typée, jamais par une panique.
pub trait Command {
    /// Applique la modification au cahier.
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()>;

    /// Défait exactement la modification.
    fn undo(&mut self, notebook: &mut Notebook) -> Result<()>;

    /// Nom court, pour l'UI et le débogage (ex. « add-object »).
    fn name(&self) -> &'static str;
}

/// Ajoute un objet en fin de calque (ex. un trait à l'écriture, J1).
///
/// L'objet est déplacé dans la commande puis récupéré par `undo`, sans copie.
/// Le rétablissement l'ajoute de nouveau en fin de pile : équivalent à sa
/// place d'origine, puisque l'ordre LIFO du `UndoStack` rejoue la séquence.
pub struct AddObject {
    target: LayerRef,
    /// `Some` quand la commande n'est pas appliquée (objet en attente).
    object: Option<Object>,
    /// Index de l'objet dans le calque, `Some` quand la commande est appliquée.
    inserted_at: Option<usize>,
}

impl AddObject {
    pub fn new(target: LayerRef, object: Object) -> Self {
        Self {
            target,
            object: Some(object),
            inserted_at: None,
        }
    }
}

impl Command for AddObject {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        let layer = notebook.layer_mut(self.target)?;
        let Some(object) = self.object.take() else {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        };
        self.inserted_at = Some(layer.push_object(object));
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        let Some(index) = self.inserted_at else {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        };
        let layer = notebook.layer_mut(self.target)?;
        let object = layer.remove_object(index)?;
        self.object = Some(object);
        self.inserted_at = None;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "add-object"
    }
}

/// Supprime des objets d'un calque (ex. gomme par trait, F-33, J1).
///
/// L'annulation réinsère les objets à leur place d'origine.
pub struct RemoveObjects {
    target: LayerRef,
    /// Index à retirer, triés croissants et distincts (validés à la création).
    indices: Vec<usize>,
    /// (index d'origine, objet) enregistrés à l'exécution, triés croissants ;
    /// renseigné par l'exécution, vidé par l'annulation.
    removed: Vec<(usize, Object)>,
    /// `true` quand la commande est appliquée. Un drapeau explicite, et non
    /// `removed.is_empty()` : la suppression d'une liste d'index vide laisse
    /// `removed` vide dans les deux sens, sans pour autant pouvoir être
    /// exécutée deux fois.
    applied: bool,
}

impl RemoveObjects {
    /// Prépare la suppression des objets aux index donnés, dans un ordre
    /// quelconque. Les doublons sont refusés.
    pub fn new(target: LayerRef, mut indices: Vec<usize>) -> Result<Self> {
        indices.sort_unstable();
        if let Some(duplicate) = indices.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(ModelError::DuplicateObjectIndex(duplicate[0]));
        }
        Ok(Self {
            target,
            indices,
            removed: Vec::new(),
            applied: false,
        })
    }
}

impl Command for RemoveObjects {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        if self.applied {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        }
        let layer = notebook.layer_mut(self.target)?;
        let count = layer.object_count();
        // Vérifie tous les index avant toute mutation (atomicité).
        if let Some(&index) = self.indices.iter().find(|&&index| index >= count) {
            return Err(ModelError::ObjectOutOfRange { index, count });
        }
        // Retire du plus grand index au plus petit pour ne pas décaler les
        // index restants. Infaillible après la vérification ci-dessus (index
        // valides et distincts) ; par défense, un échec remet les objets déjà
        // retirés à leur place avant de renvoyer l'erreur, pour que le calque
        // reste intact et qu'aucun objet ne soit perdu.
        for &index in self.indices.iter().rev() {
            let object = match layer.remove_object(index) {
                Ok(object) => object,
                Err(error) => {
                    // `removed` est en ordre décroissant : la réinsertion se
                    // fait en ordre croissant (positions d'origine). L'index
                    // est borné par la longueur courante : l'insertion ne
                    // peut pas échouer, donc aucun objet n'est perdu.
                    for (index, object) in std::mem::take(&mut self.removed).into_iter().rev() {
                        let _ = layer.insert_object(index.min(layer.object_count()), object);
                    }
                    return Err(error);
                }
            };
            self.removed.push((index, object));
        }
        self.removed.reverse();
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        if !self.applied {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        }
        let layer = notebook.layer_mut(self.target)?;
        // Vérifie la réinsertion complète avant toute mutation (atomicité) :
        // à l'étape k (index croissants), l'objet est inséré dans un calque de
        // longueur `count + k`, d'où `index <= count + k`. Si le calque a été
        // modifié hors pile, l'erreur est renvoyée sans rien consommer : la
        // commande garde tous ses objets et le calque reste intact.
        let count = layer.object_count();
        for (offset, &(index, _)) in self.removed.iter().enumerate() {
            if index > count + offset {
                return Err(ModelError::ObjectOutOfRange {
                    index,
                    count: count + offset,
                });
            }
        }
        // Réinsertion par index croissant : chaque objet retrouve sa place
        // d'origine. Infaillible après la vérification ci-dessus ; l'index est
        // néanmoins borné par défense pour qu'aucun objet ne puisse être perdu.
        // `removed` n'est consommé qu'ici, après le succès de la validation.
        for (index, object) in std::mem::take(&mut self.removed) {
            let _ = layer.insert_object(index.min(layer.object_count()), object);
        }
        self.applied = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "remove-objects"
    }
}

/// Insère des pages à une position (F-04 : import à une position choisie,
/// F-05 : page blanche à n'importe quelle position).
///
/// L'annulation retire exactement le bloc inséré ; les pages circulent entre
/// la commande et le cahier sans copie (comme [`AddObject`]).
pub struct InsertPages {
    at: usize,
    /// Pages en attente d'insertion (vides tant que la commande est appliquée).
    pages: Vec<Page>,
    /// Nombre de pages insérées, mémorisé pour l'annulation.
    inserted: usize,
    applied: bool,
}

impl InsertPages {
    /// Prépare l'insertion de `pages` à partir de l'index `at` (0 = avant la
    /// première page, `page_count` = en fin de cahier).
    pub fn new(at: usize, pages: Vec<Page>) -> Self {
        Self {
            at,
            pages,
            inserted: 0,
            applied: false,
        }
    }
}

impl Command for InsertPages {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        if self.applied {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        }
        let count = notebook.page_count();
        if self.at > count {
            return Err(ModelError::PageOutOfRange {
                index: self.at,
                count,
            });
        }
        let mut pending = std::mem::take(&mut self.pages);
        self.inserted = pending.len();
        if let Err(error) = notebook.insert_pages(self.at, &mut pending) {
            // `insert_pages` rend `pending` intact en cas d'erreur : aucune
            // page n'est perdue. Impossble après la validation ci-dessus.
            self.pages = pending;
            self.inserted = 0;
            return Err(error);
        }
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        if !self.applied {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        }
        // Bornes validées par `remove_pages` avant toute mutation : en cas
        // d'échec, le cahier est intact et la commande garde ses pages.
        self.pages = notebook.remove_pages(self.at, self.inserted)?;
        self.inserted = 0;
        self.applied = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "insert-pages"
    }
}

/// Supprime des pages (F-07, avec annulation).
///
/// L'annulation restaure les pages à leur place d'origine, annotations et
/// fond PDF compris.
pub struct RemovePages {
    /// Index à retirer, triés croissants et distincts (validés à la création).
    indices: Vec<usize>,
    /// (index d'origine, page) en ordre croissant après exécution.
    removed: Vec<(usize, Page)>,
    applied: bool,
}

impl RemovePages {
    /// Prépare la suppression des pages aux index donnés, ordre quelconque.
    /// Les doublons sont refusés.
    pub fn new(mut indices: Vec<usize>) -> Result<Self> {
        indices.sort_unstable();
        if let Some(duplicate) = indices.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(ModelError::DuplicatePageIndex(duplicate[0]));
        }
        Ok(Self {
            indices,
            removed: Vec::new(),
            applied: false,
        })
    }
}

impl Command for RemovePages {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        if self.applied {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        }
        let count = notebook.page_count();
        // Bornes et invariant « au moins une page » validés avant toute
        // mutation (atomicité).
        if let Some(&index) = self.indices.iter().find(|&&index| index >= count) {
            return Err(ModelError::PageOutOfRange { index, count });
        }
        if count - self.indices.len() == 0 {
            return Err(ModelError::LastPage);
        }
        // Retrait du plus grand index au plus petit pour ne pas décaler les
        // index restants. Infaillible après la vérification ci-dessus ; par
        // défense, un échec remet les pages déjà retirées à leur place avant
        // de renvoyer l'erreur, pour qu'aucune page ne soit perdue.
        for &index in self.indices.iter().rev() {
            let page = match notebook.remove_page(index) {
                Ok(page) => page,
                Err(error) => {
                    for (index, page) in std::mem::take(&mut self.removed).into_iter().rev() {
                        let _ = notebook.insert_page(index.min(notebook.page_count()), page);
                    }
                    return Err(error);
                }
            };
            self.removed.push((index, page));
        }
        self.removed.reverse();
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        if !self.applied {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        }
        // Vérifie la réinsertion complète avant toute mutation (atomicité) :
        // à l'étape k (index croissants), la page revient dans un cahier de
        // longueur `count + k`, d'où `index <= count + k`. Si le cahier a été
        // modifié hors pile, l'erreur est renvoyée sans rien consommer.
        let count = notebook.page_count();
        for (offset, &(index, _)) in self.removed.iter().enumerate() {
            if index > count + offset {
                return Err(ModelError::PageOutOfRange {
                    index,
                    count: count + offset,
                });
            }
        }
        // Réinsertion par index croissant : chaque page retrouve sa place
        // d'origine. Infaillible après la vérification ; l'index est néanmoins
        // borné par défense pour qu'aucune page ne puisse être perdue.
        for (index, page) in std::mem::take(&mut self.removed) {
            let _ = notebook.insert_page(index.min(notebook.page_count()), page);
        }
        self.applied = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "remove-pages"
    }
}

/// Duplique une page, annotations et fond PDF compris ; la copie se place
/// juste après l'originale (F-06).
///
/// La copie est mémorisée dans la commande : le rétablissement réinsère
/// exactement la même page, et l'annulation ne retire que la copie.
pub struct DuplicatePage {
    index: usize,
    /// Copie mémorisée ; `None` avant la première exécution.
    copy: Option<Page>,
    applied: bool,
}

impl DuplicatePage {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            copy: None,
            applied: false,
        }
    }
}

impl Command for DuplicatePage {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        if self.applied {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        }
        // Première exécution : la copie est clonée depuis l'originale et
        // mémorisée ; rétablissement : la même copie est réinsérée.
        let copy = match self.copy.clone() {
            Some(copy) => copy,
            None => notebook.page(self.index)?.clone(),
        };
        self.copy = Some(copy.clone());
        let mut pages = vec![copy];
        // `insert_pages` rend les pages intactes en cas d'erreur ; la copie
        // mémorisée dans la commande n'est en aucun cas perdue.
        notebook.insert_pages(self.index + 1, &mut pages)?;
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        if !self.applied {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        }
        // Défense hors pile : ne retire la page que si c'est bien la copie,
        // sinon une page innocente disparaîtrait.
        match notebook.page(self.index + 1) {
            Ok(actual) if self.copy.as_ref() == Some(actual) => {}
            Ok(_) => {
                return Err(ModelError::CommandStateMismatch {
                    command: self.name(),
                });
            }
            Err(error) => return Err(error),
        }
        // La copie retirée n'est pas perdue : son contenu reste dans `copy`.
        let _ = notebook.remove_page(self.index + 1)?;
        self.applied = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "duplicate-page"
    }
}

/// Déplace une page (F-08 : réordonner par glisser-déposer).
///
/// La page occupe exactement l'index `to` après exécution ; l'annulation la
/// ramène à l'index `from`.
pub struct MovePage {
    from: usize,
    to: usize,
    applied: bool,
}

impl MovePage {
    pub fn new(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            applied: false,
        }
    }
}

impl Command for MovePage {
    fn execute(&mut self, notebook: &mut Notebook) -> Result<()> {
        if self.applied {
            return Err(ModelError::CommandAlreadyApplied {
                command: self.name(),
            });
        }
        // `move_page` valide les bornes avant toute mutation (atomicité).
        notebook.move_page(self.from, self.to)?;
        self.applied = true;
        Ok(())
    }

    fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        if !self.applied {
            return Err(ModelError::CommandNotApplied {
                command: self.name(),
            });
        }
        notebook.move_page(self.to, self.from)?;
        self.applied = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "move-page"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::document::{FORMAT_VERSION, Metadata, SourceId};
    use crate::model::history::UndoStack;
    use crate::model::object::{Color, Object, Stroke, StrokeTool};
    use crate::model::page::{Orientation, PageBackground, PageSize};

    fn notebook() -> Notebook {
        Notebook::new(Metadata {
            title: "Test".to_string(),
            created_at: 0,
            updated_at: 0,
            format_version: FORMAT_VERSION,
        })
    }

    fn stroke(index: usize) -> Object {
        let mut stroke = Stroke::new(StrokeTool::Pen, Color::BLACK, 2.0).expect("épaisseur valide");
        stroke.push_point(
            crate::model::object::StrokePoint::new(index as f32, 0.0, None).expect("point valide"),
        );
        Object::Stroke(stroke)
    }

    fn target() -> LayerRef {
        LayerRef::new(0, 0)
    }

    #[test]
    fn add_object_appends_in_order_and_undoes() {
        let mut notebook = notebook();
        let mut add_first = AddObject::new(target(), stroke(0));
        let mut add_second = AddObject::new(target(), stroke(1));
        add_first.execute(&mut notebook).expect("exécution");
        add_second.execute(&mut notebook).expect("exécution");
        let layer = notebook.layer(target()).expect("calque");
        assert_eq!(layer.object_count(), 2);
        assert_eq!(layer.object(0).expect("objet 0"), &stroke(0));
        assert_eq!(layer.object(1).expect("objet 1"), &stroke(1));

        add_second.undo(&mut notebook).expect("annulation");
        add_first.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 0);
    }

    #[test]
    fn add_object_rejects_double_execute_and_unapplied_undo() {
        let mut notebook = notebook();
        let mut add = AddObject::new(target(), stroke(0));
        add.undo(&mut notebook)
            .expect_err("undo avant exécution impossible");
        assert!(matches!(
            add.undo(&mut notebook),
            Err(ModelError::CommandNotApplied { .. })
        ));
        add.execute(&mut notebook).expect("exécution");
        assert!(matches!(
            add.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 1);
    }

    #[test]
    fn remove_objects_restores_original_positions() {
        let mut notebook = notebook();
        for index in 0..3 {
            AddObject::new(target(), stroke(index))
                .execute(&mut notebook)
                .expect("exécution");
        }
        let mut remove = RemoveObjects::new(target(), vec![2, 0]).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        let layer = notebook.layer(target()).expect("calque");
        assert_eq!(layer.object_count(), 1);
        assert_eq!(layer.object(0).expect("objet 0"), &stroke(1));

        remove.undo(&mut notebook).expect("annulation");
        let layer = notebook.layer(target()).expect("calque");
        assert_eq!(layer.object_count(), 3);
        assert_eq!(layer.object(0).expect("objet 0"), &stroke(0));
        assert_eq!(layer.object(1).expect("objet 1"), &stroke(1));
        assert_eq!(layer.object(2).expect("objet 2"), &stroke(2));

        remove.execute(&mut notebook).expect("rétablissement");
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 1);
    }

    #[test]
    fn remove_objects_rejects_duplicate_indices() {
        assert!(matches!(
            RemoveObjects::new(target(), vec![1, 1]),
            Err(ModelError::DuplicateObjectIndex(1))
        ));
    }

    #[test]
    fn remove_objects_out_of_range_leaves_document_untouched() {
        let mut notebook = notebook();
        AddObject::new(target(), stroke(0))
            .execute(&mut notebook)
            .expect("exécution");
        let mut remove = RemoveObjects::new(target(), vec![0, 5]).expect("commande valide");
        assert!(matches!(
            remove.execute(&mut notebook),
            Err(ModelError::ObjectOutOfRange { .. })
        ));
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 1);
    }

    #[test]
    fn remove_objects_empty_indices_is_reversible_noop() {
        let mut notebook = notebook();
        AddObject::new(target(), stroke(0))
            .execute(&mut notebook)
            .expect("exécution");
        let mut remove = RemoveObjects::new(target(), Vec::new()).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 1);
        // Régression (B1) : une seconde exécution est refusée même si
        // `removed` reste vide après une suppression sans index.
        assert!(matches!(
            remove.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        // Et l'annulation fonctionne une fois, puis est refusée.
        remove.undo(&mut notebook).expect("annulation");
        assert!(matches!(
            remove.undo(&mut notebook),
            Err(ModelError::CommandNotApplied { .. })
        ));
    }

    #[test]
    fn remove_objects_rejects_double_execute() {
        let mut notebook = notebook();
        AddObject::new(target(), stroke(0))
            .execute(&mut notebook)
            .expect("exécution");
        let mut remove = RemoveObjects::new(target(), vec![0]).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        assert!(matches!(
            remove.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 0);
    }

    #[test]
    fn remove_objects_undo_failure_keeps_objects() {
        let mut notebook = notebook();
        for index in 0..3 {
            AddObject::new(target(), stroke(index))
                .execute(&mut notebook)
                .expect("exécution");
        }
        let mut remove = RemoveObjects::new(target(), vec![2]).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        // Modifie le calque hors pile : les index d'origine ne valent plus.
        let layer = notebook.layer_mut(target()).expect("calque");
        while layer.object_count() > 0 {
            layer.remove_object(0).expect("retrait");
        }
        // Régression (B2) : l'échec est typé, sans rien consommer ni perdre.
        assert!(matches!(
            remove.undo(&mut notebook),
            Err(ModelError::ObjectOutOfRange { .. })
        ));
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 0);
        // La commande garde l'objet : il revient dès que la place existe.
        let layer = notebook.layer_mut(target()).expect("calque");
        layer.push_object(stroke(7));
        layer.push_object(stroke(8));
        remove.undo(&mut notebook).expect("annulation");
        let layer = notebook.layer(target()).expect("calque");
        assert_eq!(layer.object_count(), 3);
        assert_eq!(layer.object(2).expect("objet 2"), &stroke(2));
    }

    #[test]
    fn bad_layer_ref_fails_without_mutation() {
        let mut notebook = notebook();
        let mut add = AddObject::new(LayerRef::new(3, 0), stroke(0));
        assert!(matches!(
            add.execute(&mut notebook),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(notebook.layer(target()).expect("calque").object_count(), 0);
    }

    fn pdf_page(orientation: Orientation, page_index: u32) -> Page {
        Page::new(
            orientation,
            PageBackground::PdfPage {
                source: SourceId::new(7),
                page_index,
                source_size: PageSize::new(600.0, 300.0).expect("taille valide"),
            },
        )
    }

    #[test]
    fn insert_pages_inserts_block_and_undoes() {
        let mut notebook = notebook();
        let mut insert = InsertPages::new(
            0,
            vec![
                pdf_page(Orientation::Landscape, 0),
                pdf_page(Orientation::Landscape, 1),
            ],
        );
        insert.execute(&mut notebook).expect("exécution");
        assert_eq!(notebook.page_count(), 3);
        assert!(matches!(
            notebook.page(0).expect("page 0").background(),
            PageBackground::PdfPage { page_index: 0, .. }
        ));
        assert!(matches!(
            notebook.page(1).expect("page 1").background(),
            PageBackground::PdfPage { page_index: 1, .. }
        ));

        insert.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook.page_count(), 1);
        insert.execute(&mut notebook).expect("rétablissement");
        assert_eq!(notebook.page_count(), 3);
    }

    #[test]
    fn insert_pages_rejects_double_execute_and_unapplied_undo() {
        let mut notebook = notebook();
        let mut insert = InsertPages::new(1, vec![Page::blank(Orientation::Landscape)]);
        insert
            .undo(&mut notebook)
            .expect_err("undo avant exécution");
        assert!(matches!(
            insert.undo(&mut notebook),
            Err(ModelError::CommandNotApplied { .. })
        ));
        insert.execute(&mut notebook).expect("exécution");
        assert!(matches!(
            insert.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        assert_eq!(notebook.page_count(), 2);
    }

    #[test]
    fn insert_pages_out_of_range_keeps_all_pages() {
        let mut notebook = notebook();
        let mut insert = InsertPages::new(4, vec![Page::blank(Orientation::Landscape)]);
        assert!(matches!(
            insert.execute(&mut notebook),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(notebook.page_count(), 1);
        // La commande garde sa page (aucune fuite) mais n'est pas bloquée :
        // elle reste dans l'état « non appliquée ».
        assert!(matches!(
            insert.undo(&mut notebook),
            Err(ModelError::CommandNotApplied { .. })
        ));
        let mut retry = InsertPages::new(1, vec![Page::blank(Orientation::Landscape)]);
        retry.execute(&mut notebook).expect("exécution");
        assert_eq!(notebook.page_count(), 2);
    }

    #[test]
    fn remove_pages_restores_pages_exactly() {
        let mut notebook = notebook();
        let mut extra = vec![
            pdf_page(Orientation::Landscape, 3),
            Page::blank(Orientation::Landscape),
        ];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        // Une annotation sur la page 1, pour vérifier qu'elle survit.
        notebook
            .layer_mut(LayerRef::new(1, 0))
            .expect("calque")
            .push_object(stroke(9));
        let before = notebook.clone();

        let mut remove = RemovePages::new(vec![2, 0]).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        assert_eq!(notebook.page_count(), 1);
        assert_eq!(
            notebook.page(0).expect("page 0"),
            before.page(1).expect("page 1")
        );

        remove.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook, before, "les pages sont restaurées à l'identique");
    }

    #[test]
    fn remove_pages_cannot_empty_the_notebook() {
        let mut notebook = notebook();
        let mut remove = RemovePages::new(vec![0]).expect("commande valide");
        assert!(matches!(
            remove.execute(&mut notebook),
            Err(ModelError::LastPage)
        ));
        assert_eq!(notebook.page_count(), 1);

        let mut extra = vec![Page::blank(Orientation::Landscape)];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        let mut remove_all = RemovePages::new(vec![0, 1]).expect("commande valide");
        assert!(matches!(
            remove_all.execute(&mut notebook),
            Err(ModelError::LastPage)
        ));
        assert_eq!(notebook.page_count(), 2, "rien n'a été supprimé");
    }

    #[test]
    fn remove_pages_rejects_duplicate_indices() {
        assert!(matches!(
            RemovePages::new(vec![1, 1]),
            Err(ModelError::DuplicatePageIndex(1))
        ));
    }

    #[test]
    fn remove_pages_undo_failure_keeps_pages() {
        let mut notebook = notebook();
        let mut extra = vec![
            Page::blank(Orientation::Landscape),
            pdf_page(Orientation::Landscape, 5),
        ];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        let mut remove = RemovePages::new(vec![2]).expect("commande valide");
        remove.execute(&mut notebook).expect("exécution");
        // Modifie le cahier hors pile : l'index d'origine ne vaut plus.
        notebook.remove_page(1).expect("retrait hors pile");
        // L'annulation échoue sans rien consommer ni perdre.
        assert!(matches!(
            remove.undo(&mut notebook),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(notebook.page_count(), 1);
        // Dès que la place existe à nouveau, l'annulation fonctionne.
        let mut back = vec![Page::blank(Orientation::Landscape)];
        notebook.insert_pages(1, &mut back).expect("insertion");
        notebook.insert_pages(1, &mut back).expect("insertion");
        remove.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook.page_count(), 3);
        assert!(matches!(
            notebook.page(2).expect("page 2").background(),
            PageBackground::PdfPage { page_index: 5, .. }
        ));
    }

    #[test]
    fn duplicate_page_copies_annotations_and_background() {
        let mut notebook = notebook();
        let mut extra = vec![pdf_page(Orientation::Landscape, 8)];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        notebook
            .layer_mut(LayerRef::new(1, 0))
            .expect("calque")
            .push_object(stroke(4));
        let original = notebook.page(1).expect("page 1").clone();

        let mut duplicate = DuplicatePage::new(1);
        duplicate.execute(&mut notebook).expect("exécution");
        assert_eq!(notebook.page_count(), 3);
        assert_eq!(
            notebook.page(2).expect("page 2"),
            &original,
            "la copie, juste après l'originale, est identique (F-06)"
        );
        assert_eq!(notebook.page(1).expect("page 1"), &original);

        duplicate.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook.page_count(), 2);
        duplicate.execute(&mut notebook).expect("rétablissement");
        assert_eq!(notebook.page(2).expect("page 2"), &original);
    }

    #[test]
    fn duplicate_page_undo_refuses_when_copy_is_not_there() {
        let mut notebook = notebook();
        let mut duplicate = DuplicatePage::new(0);
        duplicate.execute(&mut notebook).expect("exécution");
        // Hors pile : la copie est remplacée par une autre page.
        notebook.remove_page(1).expect("retrait hors pile");
        let mut other = vec![Page::blank(Orientation::Landscape)];
        notebook.insert_pages(1, &mut other).expect("insertion");
        // L'annulation refuse de dévorer une page innocente.
        assert!(matches!(
            duplicate.undo(&mut notebook),
            Err(ModelError::CommandStateMismatch { .. })
        ));
        assert_eq!(notebook.page_count(), 2, "aucune page n'a disparu");
    }

    #[test]
    fn duplicate_page_rejects_double_execute_and_out_of_range() {
        let mut notebook = notebook();
        let mut duplicate = DuplicatePage::new(3);
        assert!(matches!(
            duplicate.execute(&mut notebook),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(notebook.page_count(), 1);
        let mut duplicate = DuplicatePage::new(0);
        duplicate.execute(&mut notebook).expect("exécution");
        assert!(matches!(
            duplicate.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        duplicate.undo(&mut notebook).expect("annulation");
        assert!(matches!(
            duplicate.undo(&mut notebook),
            Err(ModelError::CommandNotApplied { .. })
        ));
    }

    #[test]
    fn move_page_roundtrip_and_guards() {
        let mut notebook = notebook();
        let mut extra = vec![
            Page::blank(Orientation::Landscape),
            pdf_page(Orientation::Landscape, 2),
        ];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        let before = notebook.clone();

        let mut move_page = MovePage::new(0, 2);
        move_page.execute(&mut notebook).expect("exécution");
        assert_eq!(
            notebook.page(2).expect("page 2"),
            before.page(0).expect("page 0")
        );
        move_page.undo(&mut notebook).expect("annulation");
        assert_eq!(notebook, before);
        move_page.execute(&mut notebook).expect("rétablissement");
        assert_eq!(
            notebook.page(2).expect("page 2"),
            before.page(0).expect("page 0")
        );

        assert!(matches!(
            move_page.execute(&mut notebook),
            Err(ModelError::CommandAlreadyApplied { .. })
        ));
        let mut bad = MovePage::new(0, 9);
        assert!(matches!(
            bad.execute(&mut notebook),
            Err(ModelError::PageOutOfRange { .. })
        ));
    }

    #[test]
    fn page_commands_leave_stack_consistent_after_error() {
        let mut notebook = notebook();
        let mut stack = UndoStack::new(10);
        let mut bad = RemovePages::new(vec![5]).expect("commande valide");
        assert!(bad.execute(&mut notebook).is_err());
        assert_eq!(stack.undo_depth(), 0, "la pile n'avale pas les échecs");
        stack
            .execute(
                InsertPages::new(1, vec![Page::blank(Orientation::Landscape)]),
                &mut notebook,
            )
            .expect("exécution");
        assert_eq!(stack.undo_depth(), 1);
    }

    #[test]
    fn mixed_page_commands_roundtrip_through_undo_stack() {
        let mut notebook = notebook();
        let mut extra = vec![pdf_page(Orientation::Landscape, 1)];
        notebook.insert_pages(1, &mut extra).expect("insertion");
        let before = notebook.clone();

        let mut stack = UndoStack::new(10);
        stack
            .execute(
                InsertPages::new(0, vec![Page::blank(Orientation::Landscape)]),
                &mut notebook,
            )
            .expect("insertion");
        stack
            .execute(
                AddObject::new(LayerRef::new(2, 0), stroke(1)),
                &mut notebook,
            )
            .expect("annotation");
        stack
            .execute(MovePage::new(0, 2), &mut notebook)
            .expect("déplacement");
        stack
            .execute(DuplicatePage::new(1), &mut notebook)
            .expect("duplication");
        stack
            .execute(
                RemovePages::new(vec![0]).expect("commande valide"),
                &mut notebook,
            )
            .expect("suppression");

        while stack.can_undo() {
            stack.undo(&mut notebook).expect("annulation");
        }
        assert_eq!(notebook, before, "retour à l'état initial");
        while stack.can_redo() {
            stack.redo(&mut notebook).expect("rétablissement");
        }
        // État final des 5 commandes : [pdf, copie, paysage].
        assert_eq!(notebook.page_count(), 3);
        assert_eq!(
            notebook.page(0).expect("page 0"),
            notebook.page(1).expect("page 1")
        );
    }
}
