use std::{any::TypeId, sync::Arc};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::{Acquirable, EntityId, Extractable, entity::EntityData, extractor::Extractor};

/// An archetype represents a unique combination of component types.
/// All entities with the same structure share an archetype.
pub struct Archetype {
    pub(crate) extractor: &'static Extractor,

    /// Entities stored in this archetype.
    pub(crate) entities: Arc<RwLock<FxHashMap<EntityId, EntityData>>>,
}

impl Archetype {
    pub(crate) fn new<E: Extractable>() -> Self {
        Self {
            extractor: crate::get_extractor::<E>(),
            entities: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub(crate) fn add_entity<E: Extractable>(&self, id: EntityId, entity: E) -> EntityData {
        let data = EntityData::new(entity, self.extractor);
        self.entities.write().insert(id, data.clone());
        data
    }

    /// Get entity data by ID.
    pub(crate) fn extract_entity<T: 'static>(&self, entity_id: &EntityId) -> Option<Acquirable<T>> {
        self.entities
            .read()
            .get(entity_id)
            .and_then(|data| data.extract::<T>())
    }

    /// Remove an entity by ID.
    pub(crate) fn remove_entity(&self, entity_id: &EntityId) -> Option<EntityData> {
        self.entities.write().remove(entity_id)
    }
}

/// Unique identifier for an archetype based on its TypeId.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(crate) struct ArchetypeId(pub TypeId);

impl ArchetypeId {
    pub(crate) fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>())
    }
}
