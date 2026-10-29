//! Pile annuler/rétablir (F-35, CdC v0.2 §7.2).

use std::collections::VecDeque;

use crate::model::command::Command;
use crate::model::document::Notebook;
use crate::model::error::{ModelError, Result};

/// Nombre de commandes annulables conservées par défaut.
pub const DEFAULT_LIMIT: usize = 100;

/// Pile annuler/rétablir (F-35).
///
/// Toutes les commandes s'appliquent au même [`Notebook`], dans l'ordre strict
/// où elles ont été exécutées ; annuler et rétablir suivent cet ordre (LIFO).
/// Quand la pile d'annulation atteint `limit`, la commande la plus ancienne est
/// écartée définitivement : la mémoire reste bornée (esprit NF-01).
pub struct UndoStack {
    undo: VecDeque<Box<dyn Command>>,
    redo: VecDeque<Box<dyn Command>>,
    limit: usize,
}

impl UndoStack {
    /// Crée une pile limitée à `limit` commandes annulables (au moins 1).
    pub fn new(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit: limit.max(1),
        }
    }

    /// Applique la commande au cahier et l'empile.
    ///
    /// En cas d'erreur, la commande n'est pas empilée et le cahier est intact
    /// (contrat d'atomicité de `Command`). Toute nouvelle exécution invalide
    /// la pile de rétablissement.
    pub fn execute(
        &mut self,
        mut command: impl Command + 'static,
        notebook: &mut Notebook,
    ) -> Result<()> {
        command.execute(notebook)?;
        self.redo.clear();
        self.undo.push_back(Box::new(command));
        self.trim();
        Ok(())
    }

    /// Annule la commande la plus récente. En cas d'erreur, la commande reste
    /// sur la pile (l'état des piles ne change pas).
    pub fn undo(&mut self, notebook: &mut Notebook) -> Result<()> {
        let Some(mut command) = self.undo.pop_back() else {
            return Err(ModelError::NothingToUndo);
        };
        match command.undo(notebook) {
            Ok(()) => {
                self.redo.push_back(command);
                Ok(())
            }
            Err(error) => {
                self.undo.push_back(command);
                Err(error)
            }
        }
    }

    /// Rétablit la commande annulée la plus récente. En cas d'erreur, la
    /// commande reste sur la pile de rétablissement.
    pub fn redo(&mut self, notebook: &mut Notebook) -> Result<()> {
        let Some(mut command) = self.redo.pop_back() else {
            return Err(ModelError::NothingToRedo);
        };
        match command.execute(notebook) {
            Ok(()) => {
                self.undo.push_back(command);
                self.trim();
                Ok(())
            }
            Err(error) => {
                self.redo.push_back(command);
                Err(error)
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Vide les deux piles (ex. après ouverture d'un document).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Écarte les commandes les plus anciennes au-delà de la limite.
    fn trim(&mut self) {
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(DEFAULT_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::command::{AddObject, RemoveObjects};
    use crate::model::document::{FORMAT_VERSION, Metadata};
    use crate::model::layer::LayerRef;
    use crate::model::object::{Color, Object, Stroke, StrokePoint, StrokeTool};

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
        stroke.push_point(StrokePoint::new(index as f32, 0.0, None).expect("point valide"));
        Object::Stroke(stroke)
    }

    fn target() -> LayerRef {
        LayerRef::new(0, 0)
    }

    fn object_count(notebook: &Notebook) -> usize {
        notebook.layer(target()).expect("calque").object_count()
    }

    #[test]
    fn undo_redo_roundtrip_add_object() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(AddObject::new(target(), stroke(0)), &mut notebook)
            .expect("exécution");
        assert_eq!(object_count(&notebook), 1);

        stack.undo(&mut notebook).expect("annulation");
        assert_eq!(object_count(&notebook), 0);
        assert!(stack.can_redo());

        stack.redo(&mut notebook).expect("rétablissement");
        assert_eq!(object_count(&notebook), 1);
        assert_eq!(
            notebook
                .layer(target())
                .expect("calque")
                .object(0)
                .expect("objet 0"),
            &stroke(0)
        );
    }

    #[test]
    fn undo_redo_roundtrip_remove_objects() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        for index in 0..3 {
            stack
                .execute(AddObject::new(target(), stroke(index)), &mut notebook)
                .expect("exécution");
        }
        stack
            .execute(
                RemoveObjects::new(target(), vec![0, 2]).expect("commande valide"),
                &mut notebook,
            )
            .expect("exécution");
        assert_eq!(object_count(&notebook), 1);

        stack.undo(&mut notebook).expect("annulation");
        assert_eq!(object_count(&notebook), 3);
        for index in 0..3 {
            assert_eq!(
                notebook
                    .layer(target())
                    .expect("calque")
                    .object(index)
                    .expect("objet"),
                &stroke(index)
            );
        }

        stack.redo(&mut notebook).expect("rétablissement");
        assert_eq!(object_count(&notebook), 1);
        assert_eq!(
            notebook
                .layer(target())
                .expect("calque")
                .object(0)
                .expect("objet 0"),
            &stroke(1)
        );
    }

    #[test]
    fn undo_without_command_is_typed_error() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        assert!(matches!(
            stack.undo(&mut notebook),
            Err(ModelError::NothingToUndo)
        ));
        assert!(matches!(
            stack.redo(&mut notebook),
            Err(ModelError::NothingToRedo)
        ));
    }

    #[test]
    fn execute_clears_redo() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(AddObject::new(target(), stroke(0)), &mut notebook)
            .expect("exécution");
        stack.undo(&mut notebook).expect("annulation");
        assert!(stack.can_redo());
        stack
            .execute(AddObject::new(target(), stroke(1)), &mut notebook)
            .expect("exécution");
        assert!(!stack.can_redo());
        assert!(matches!(
            stack.redo(&mut notebook),
            Err(ModelError::NothingToRedo)
        ));
    }

    #[test]
    fn undo_limit_drops_oldest_commands() {
        let mut notebook = notebook();
        let mut stack = UndoStack::new(2);
        for index in 0..3 {
            stack
                .execute(AddObject::new(target(), stroke(index)), &mut notebook)
                .expect("exécution");
        }
        assert_eq!(stack.undo_depth(), 2);
        stack.undo(&mut notebook).expect("annulation");
        stack.undo(&mut notebook).expect("annulation");
        assert!(!stack.can_undo());
        // La commande la plus ancienne (stroke(0)) a été écartée.
        assert_eq!(object_count(&notebook), 1);
        assert_eq!(
            notebook
                .layer(target())
                .expect("calque")
                .object(0)
                .expect("objet 0"),
            &stroke(0)
        );
    }

    #[test]
    fn undo_sequence_is_strictly_lifo() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(AddObject::new(target(), stroke(0)), &mut notebook)
            .expect("exécution");
        stack
            .execute(AddObject::new(target(), stroke(1)), &mut notebook)
            .expect("exécution");
        stack.undo(&mut notebook).expect("annulation");
        stack.undo(&mut notebook).expect("annulation");
        stack.redo(&mut notebook).expect("rétablissement");
        stack.redo(&mut notebook).expect("rétablissement");
        assert_eq!(object_count(&notebook), 2);
        assert_eq!(
            notebook
                .layer(target())
                .expect("calque")
                .object(0)
                .expect("objet 0"),
            &stroke(0)
        );
        assert_eq!(
            notebook
                .layer(target())
                .expect("calque")
                .object(1)
                .expect("objet 1"),
            &stroke(1)
        );
    }

    #[test]
    fn empty_remove_objects_does_not_block_undo_stack() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(AddObject::new(target(), stroke(0)), &mut notebook)
            .expect("exécution");
        stack
            .execute(
                RemoveObjects::new(target(), Vec::new()).expect("commande valide"),
                &mut notebook,
            )
            .expect("exécution");
        assert_eq!(stack.undo_depth(), 2);
        // Régression (B1) : une suppression sans index ne bloque pas la pile.
        stack.undo(&mut notebook).expect("annulation");
        assert_eq!(stack.undo_depth(), 1);
        stack.undo(&mut notebook).expect("annulation");
        assert_eq!(object_count(&notebook), 0);
    }

    #[test]
    fn failed_execute_leaves_stacks_unchanged() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(AddObject::new(target(), stroke(0)), &mut notebook)
            .expect("exécution");
        stack.undo(&mut notebook).expect("annulation");
        assert_eq!(stack.redo_depth(), 1);
        // Exécution en erreur (calque inexistant) : rien n'est empilé et les
        // deux piles restent intactes.
        assert!(matches!(
            stack.execute(
                AddObject::new(LayerRef::new(3, 0), stroke(1)),
                &mut notebook
            ),
            Err(ModelError::PageOutOfRange { .. })
        ));
        assert_eq!(stack.undo_depth(), 0);
        assert_eq!(stack.redo_depth(), 1);
        stack.redo(&mut notebook).expect("rétablissement");
        assert_eq!(object_count(&notebook), 1);
    }

    /// Commande factice dont l'annulation échoue, pour tester le comportement
    /// de la pile face aux erreurs.
    struct FailingUndo;

    impl Command for FailingUndo {
        fn execute(&mut self, _notebook: &mut Notebook) -> Result<()> {
            Ok(())
        }

        fn undo(&mut self, _notebook: &mut Notebook) -> Result<()> {
            Err(ModelError::CommandNotApplied {
                command: self.name(),
            })
        }

        fn name(&self) -> &'static str {
            "failing-undo"
        }
    }

    #[test]
    fn failed_undo_keeps_command_on_stack() {
        let mut notebook = notebook();
        let mut stack = UndoStack::default();
        stack
            .execute(FailingUndo, &mut notebook)
            .expect("exécution");
        assert!(stack.undo(&mut notebook).is_err());
        assert!(stack.can_undo());
        assert_eq!(stack.undo_depth(), 1);
        assert_eq!(stack.redo_depth(), 0);
    }
}
