use std::{any::TypeId, collections::HashMap, hash::Hash, ptr::NonNull, sync::Arc};

pub use flatecs_macros::Extractable;

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    id: u32,
}

pub enum ExtractionMetadata {
    Target {
        type_id: TypeId,
        offset: usize,
    },
    Nested {
        type_id: TypeId,
        offset: usize,
        nested: &'static [ExtractionMetadata],
    },
}

impl ExtractionMetadata {
    pub const fn new<T: 'static>(offset: usize) -> Self {
        Self::Target {
            type_id: TypeId::of::<T>(),
            offset,
        }
    }

    pub const fn new_nested<T: Extractable>(
        offset: usize,
        nested: &'static [ExtractionMetadata],
    ) -> Self {
        Self::Nested {
            type_id: TypeId::of::<T>(),
            offset,
            nested,
        }
    }

    pub fn flatten(list: &[ExtractionMetadata]) -> HashMap<TypeId, usize> {
        let mut result = HashMap::new();
        Self::flatten_internal(list, 0, &mut result);
        result
    }

    fn flatten_internal(
        list: &[ExtractionMetadata],
        base_offset: usize,
        result: &mut HashMap<TypeId, usize>,
    ) {
        for metadata in list {
            match metadata {
                ExtractionMetadata::Target { type_id, offset } => {
                    result.insert(*type_id, base_offset + *offset);
                }
                ExtractionMetadata::Nested {
                    type_id,
                    offset,
                    nested,
                } => {
                    result.insert(*type_id, base_offset + *offset);
                    Self::flatten_internal(nested, base_offset + *offset, result);
                }
            }
        }
    }
}

pub trait Extractable: 'static + Sized {
    const METADATA_LIST: &'static [ExtractionMetadata];
}

struct EntityDataInner {
    data: NonNull<u8>,
    extractor: Arc<Extractor>,
}

unsafe impl Send for EntityDataInner {}
unsafe impl Sync for EntityDataInner {}

impl Drop for EntityDataInner {
    fn drop(&mut self) {
        unsafe { (self.extractor.dropper)(self.data) };
    }
}

impl Clone for EntityDataInner {
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            extractor: Arc::clone(&self.extractor),
        }
    }
}

impl EntityDataInner {
    fn extract<T: 'static>(&self) -> Option<&T> {
        self.extractor.extract::<T>(self.data)
    }
}

pub struct EntityData {
    inner: Arc<EntityDataInner>,
}

impl EntityData {
    pub(crate) fn new<E: Extractable>(entity: E, extractor: Arc<Extractor>) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        Self {
            inner: Arc::new(EntityDataInner {
                data: unsafe { NonNull::new_unchecked(ptr) },
                extractor,
            }),
        }
    }

    pub fn extract<T: 'static>(&self) -> Option<&T> {
        self.inner.extract::<T>()
    }
}

pub struct Extractor {
    offsets: HashMap<TypeId, usize>,
    dropper: unsafe fn(NonNull<u8>),
}

impl Extractor {
    pub(crate) fn new<E: Extractable>() -> Self {
        Self {
            offsets: ExtractionMetadata::flatten(E::METADATA_LIST)
                .into_iter()
                .collect(),
            dropper: |ptr| unsafe {
                drop(Box::from_raw(ptr.as_ptr() as *mut E));
            },
        }
    }

    pub fn extract<T: 'static>(&self, data: NonNull<u8>) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let offset = self.offsets.get(&type_id)?;
        let ptr = unsafe { data.as_ptr().add(*offset) as *const T };
        Some(unsafe { &*ptr })
    }
}

#[derive(Default)]
pub struct World {
    entities: HashMap<EntityId, EntityData>,
    extractors: HashMap<TypeId, Arc<Extractor>>,
    next_entity_id: u32,
}

impl World {
    pub fn extract_component<T: 'static>(&self, entity_id: &EntityId) -> Option<&T> {
        let entity_data = self.entities.get(entity_id)?;
        entity_data.extract::<T>()
    }

    fn get_extractor<E: Extractable>(&mut self) -> Arc<Extractor> {
        let type_id = TypeId::of::<E>();
        let extractor = self
            .extractors
            .entry(type_id)
            .or_insert_with(|| Arc::new(Extractor::new::<E>()));
        Arc::clone(extractor)
    }

    pub fn add_entity<E: Extractable>(&mut self, entity: E) -> EntityId {
        let entity_id = EntityId {
            id: self.next_entity_id,
        };
        self.next_entity_id += 1;
        let entity_data = EntityData::new(entity, self.get_extractor::<E>());
        self.entities.insert(entity_id, entity_data);
        entity_id
    }

    pub fn query<T: 'static>(&self) -> Vec<(&EntityId, &T)> {
        let mut results = Vec::new();
        for (entity_id, entity_data) in &self.entities {
            if let Some(component) = entity_data.extract::<T>() {
                results.push((entity_id, component));
            }
        }
        results
    }
}
