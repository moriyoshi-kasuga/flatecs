use std::{any::TypeId, sync::Arc};

use dashmap::DashMap;
use rustc_hash::FxBuildHasher;

use crate::{EntityId, Extractable, World, entity::EntityData};

pub struct QueryIter<'a, T: Extractable> {
    _phantom: std::marker::PhantomData<T>,
    matching: Vec<(usize, Arc<DashMap<EntityId, EntityData, FxBuildHasher>>)>,
    current: Option<(
        usize,
        dashmap::iter::Iter<
            'a,
            EntityId,
            EntityData,
            FxBuildHasher,
            DashMap<EntityId, EntityData, FxBuildHasher>,
        >,
    )>,
}

impl<'a, T: Extractable> QueryIter<'a, T> {
    pub(crate) fn new(world: &World) -> Self {
        let type_id = TypeId::of::<T>();
        let archetype = world
            .type_index
            .get(&type_id)
            .iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|archetype_id| {
                world.archetypes.get(archetype_id).map(|a| {
                    // SAFETY: The archetype is guaranteed to contain type T
                    let offset = unsafe { a.extractor.offset(&type_id).unwrap_unchecked() };
                    (offset, a.entities.clone())
                })
            });
        Self {
            _phantom: std::marker::PhantomData,
            matching: archetype,
            current: None,
        }
    }
}

impl<'a, T: Extractable> Iterator for QueryIter<'a, T> {
    type Item = (EntityId, crate::Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((offset, current_iter)) = &mut self.current {
                if let Some(entry) = current_iter.next() {
                    let entity_id = *entry.key();
                    let entity_data = entry.value();
                    return Some((entity_id, unsafe { entity_data.extract_by_offset(*offset) }));
                } else {
                    self.current = None;
                }
            } else {
                todo!();
                // if let Some((offset, next_map)) = self.matching.pop() {
                //     self.current = Some((offset, next_map.iter()));
                // } else {
                //     return None;
                // }
            }
        }
    }
}
