use std::{
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::extractor::Extractor;

/// Unique identifier for an entity in the World.
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct EntityId {
    pub(crate) id: u32,
}

impl EntityId {
    pub(crate) fn new(id: u32) -> Self {
        Self { id }
    }

    /// Get the raw ID value
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// Internal reference-counted data for an entity.
pub(crate) struct EntityDataInner {
    pub(crate) data: NonNull<u8>,
    pub(crate) counter: AtomicUsize,
    pub(crate) extractor: Arc<Extractor>,
}

#[repr(transparent)]
pub(crate) struct EntityData {
    inner: NonNull<EntityDataInner>,
}

unsafe impl Send for EntityData {}
unsafe impl Sync for EntityData {}

impl EntityData {
    pub(crate) fn inner(&self) -> &EntityDataInner {
        unsafe { self.inner.as_ref() }
    }

    pub(crate) fn new<E: crate::Extractable>(entity: E, extractor: Arc<Extractor>) -> Self {
        let ptr = Box::into_raw(Box::new(entity)) as *mut u8;
        let inner = EntityDataInner {
            data: unsafe { NonNull::new_unchecked(ptr) },
            counter: AtomicUsize::new(1),
            extractor,
        };
        Self {
            inner: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(inner))) },
        }
    }

    pub(crate) fn extract<T: 'static>(&self) -> Option<crate::Acquirable<T>> {
        let extracted = unsafe { self.extract_ptr::<T>()? };
        Some(crate::Acquirable::new(extracted, self.clone()))
    }

    pub(crate) unsafe fn extract_ptr<T: 'static>(&self) -> Option<NonNull<T>> {
        unsafe { self.inner().extractor.extract_ptr::<T>(self.inner().data) }
    }
}

impl Drop for EntityData {
    fn drop(&mut self) {
        let inner = self.inner();
        if inner.counter.fetch_sub(1, Ordering::Release) > 1 {
            return;
        }

        std::sync::atomic::fence(Ordering::Acquire);

        unsafe { (inner.extractor.dropper)(inner.data) };
        unsafe {
            let inner = Box::from_raw(self.inner.as_ptr());
            drop(inner);
        }
    }
}

impl Clone for EntityData {
    fn clone(&self) -> Self {
        self.inner().counter.fetch_add(1, Ordering::Relaxed);

        Self { inner: self.inner }
    }
}
