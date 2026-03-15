use crate::{editor::document::Document, model::overlay::OverlayObject};
use image::RgbaImage;

#[derive(Clone)]
struct DocumentSnapshot {
    base_image: RgbaImage,
    overlays: Vec<OverlayObject>,
    source_uri: Option<String>,
}

impl From<&Document> for DocumentSnapshot {
    fn from(document: &Document) -> Self {
        Self {
            base_image: document.base_image().clone(),
            overlays: document.overlays().to_vec(),
            source_uri: document.source_uri().cloned(),
        }
    }
}

#[derive(Default)]
pub struct HistoryManager {
    undo_stack: Vec<DocumentSnapshot>,
    redo_stack: Vec<DocumentSnapshot>,
    limit: usize,
}

impl HistoryManager {
    pub fn new(limit: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            limit,
        }
    }

    pub fn checkpoint(&mut self, document: &Document) {
        if self.limit > 0 && self.undo_stack.len() >= self.limit {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(DocumentSnapshot::from(document));
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, document: &mut Document) -> bool {
        let Some(snapshot) = self.undo_stack.pop() else {
            return false;
        };

        self.redo_stack.push(DocumentSnapshot::from(&*document));
        document.restore(snapshot.base_image, snapshot.overlays, snapshot.source_uri);
        true
    }

    pub fn redo(&mut self, document: &mut Document) -> bool {
        let Some(snapshot) = self.redo_stack.pop() else {
            return false;
        };

        self.undo_stack.push(DocumentSnapshot::from(&*document));
        document.restore(snapshot.base_image, snapshot.overlays, snapshot.source_uri);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
