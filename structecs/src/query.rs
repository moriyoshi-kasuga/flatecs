use std::{any::TypeId, sync::Arc};

use parking_lot::{RwLock, lock_api::RawRwLock};
use rustc_hash::FxHashMap;

use crate::{EntityId, Extractable, World, entity::EntityData};

type MapIter<'a> = std::collections::hash_map::Iter<'a, EntityId, EntityData>;

type Map = Arc<RwLock<FxHashMap<EntityId, EntityData>>>;

pub struct QueryIter<T: 'static> {
    _phantom: std::marker::PhantomData<T>,
    matching: Vec<(usize, Map)>,
    current: Option<(usize, Map, MapIter<'static>)>,
}

impl<T: 'static> QueryIter<T> {
    pub(crate) fn new(world: &World) -> Self {
        let type_id = TypeId::of::<T>();
        let matching = if let Some(archetype_ids) = world.type_index.get(&type_id) {
            // Pre-allocate capacity for better performance
            archetype_ids
                .iter()
                .filter_map(|archetype_id| {
                    world.archetypes.get(archetype_id).map(|archetype| {
                        // SAFETY: The archetype is guaranteed to contain type T
                        let offset =
                            unsafe { archetype.extractor.offset(&type_id).unwrap_unchecked() };
                        (offset, archetype.entities.clone())
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            _phantom: std::marker::PhantomData,
            matching,
            current: None,
        }
    }
}

impl<T: Extractable> Iterator for QueryIter<T> {
    type Item = (EntityId, crate::Acquirable<T>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((offset, map, current_iter)) = &mut self.current {
                if let Some((entity_id, entity_data)) = current_iter.next() {
                    return Some((*entity_id, unsafe {
                        entity_data.extract_by_offset(*offset)
                    }));
                } else {
                    unsafe { map.raw().unlock_shared() }
                    self.current = None;
                }
            } else if let Some((offset, next_map)) = self.matching.pop() {
                unsafe { next_map.raw().lock_shared() };
                let iter = unsafe { &*next_map.data_ptr() }.iter();
                self.current = Some((offset, next_map, iter));
            } else {
                return None;
            }
        }
    }
}

impl<T: 'static> Drop for QueryIter<T> {
    fn drop(&mut self) {
        if let Some((_, map, _)) = &self.current {
            unsafe { map.raw().unlock_shared() }
        }
    }
}
