use std::{any::TypeId, sync::Arc};

use dashmap::DashMap;
use rustc_hash::FxBuildHasher;

use crate::{EntityId, Extractable, World, entity::EntityData};

pub struct QueryIter<T: Extractable> {
    _phantom: std::marker::PhantomData<T>,
    matching: Vec<Arc<DashMap<EntityId, EntityData, FxBuildHasher>>>,
}

impl<T: Extractable> QueryIter<T> {
    pub(crate) fn new(world: &World) -> Self {
        let type_id = TypeId::of::<T>();
        let archetype = world
            .type_index
            .get(&type_id)
            .iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|archetype_id| {
                world
                    .archetypes
                    .get(archetype_id)
                    .map(|a| a.entities.clone())
            })
            .collect();
        Self {
            _phantom: std::marker::PhantomData,
            matching: archetype,
        }
    }
}

impl<T: Extractable> Iterator for QueryIter<T> {
    type Item = (EntityId, crate::Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
