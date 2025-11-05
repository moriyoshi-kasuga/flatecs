use std::{
    any::TypeId,
    sync::atomic::{AtomicU32, Ordering},
};

use dashmap::DashMap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use crate::{
    Acquirable, EntityId, Extractable, WorldError,
    archetype::{Archetype, ArchetypeId},
};

/// The central storage for all entities and their components.
///
/// Entities are organized into archetypes based on their structure for better performance.
///
/// # Thread Safety
///
/// World uses lock-free data structures (DashMap) and per-archetype RwLocks for
/// efficient concurrent access. Multiple threads can:
/// - Add entities to different archetypes in parallel
/// - Query different archetypes in parallel
/// - Query and add entities simultaneously (queries snapshot archetypes)
///
/// # Query Optimization
///
/// The World maintains a type index that maps component types to the archetypes
/// that contain them. This eliminates the need to check all archetypes during queries,
/// significantly improving performance when many archetypes exist.
#[derive(Default)]
pub struct World {
    /// Archetypes indexed by their TypeId
    pub(crate) archetypes: DashMap<ArchetypeId, Archetype, FxBuildHasher>,

    /// Maps entity IDs to their archetype for fast lookup (lock-free concurrent access).
    pub(crate) entity_index: DashMap<EntityId, ArchetypeId, FxBuildHasher>,

    /// Type index: maps component TypeId to archetypes that contain it
    /// This cache dramatically speeds up queries when there are many archetypes
    pub(crate) type_index: DashMap<TypeId, FxHashSet<ArchetypeId>, FxBuildHasher>,

    /// Next entity ID to assign (atomic for lock-free ID generation).
    next_entity_id: AtomicU32,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create an archetype for type E.
    fn get_archetype<E: Extractable>(
        &self,
    ) -> dashmap::mapref::one::RefMut<'_, ArchetypeId, Archetype> {
        let archetype_id = ArchetypeId::of::<E>();

        self.archetypes.entry(archetype_id).or_insert_with(|| {
            let archetype = Archetype::new::<E>();
            self.register_archetype_types(archetype_id, archetype.extractor.type_ids());
            archetype
        })
    }

    /// Register all component types that an archetype can provide
    fn register_archetype_types<'a>(
        &self,
        archetype_id: ArchetypeId,
        type_ids: impl Iterator<Item = &'a TypeId>,
    ) {
        for type_id in type_ids {
            self.type_index
                .entry(*type_id)
                .or_default()
                .insert(archetype_id);
        }
    }

    fn get_archetype_by_entity(
        &self,
        entity_id: &EntityId,
    ) -> Option<dashmap::mapref::one::Ref<'_, ArchetypeId, Archetype>> {
        let archetype_id = *self.entity_index.get(entity_id)?.value();
        self.archetypes.get(&archetype_id)
    }

    /// Add an entity to the world.
    ///
    /// Returns the ID assigned to the entity.
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    /// Entities with different types can be added in parallel with minimal contention.
    pub fn add_entity<E: Extractable>(&self, entity: E) -> EntityId {
        // Generate entity ID atomically
        let entity_id = EntityId::new(self.next_entity_id.fetch_add(1, Ordering::Relaxed));

        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();

        archetype.add_entity(entity_id, entity);

        self.entity_index.insert(entity_id, archetype_id);

        entity_id
    }

    pub fn add_entity_with_acquirable<E: Extractable>(
        &self,
        entity: E,
    ) -> (EntityId, Acquirable<E>) {
        let entity_id = EntityId::new(self.next_entity_id.fetch_add(1, Ordering::Relaxed));

        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();

        let data = archetype.add_entity(entity_id, entity);
        // SAFETY: The data contains type E, which matches the Acquirable<E> type we're creating.
        // This is guaranteed by the archetype.add_entity call above.
        let acquirable = unsafe { Acquirable::new_target(data) };

        self.entity_index.insert(entity_id, archetype_id);

        (entity_id, acquirable)
    }

    /// Add multiple entities to the world in batch.
    ///
    /// Returns a Vec of EntityIds assigned to the entities in order.
    ///
    /// This method is optimized for bulk insertion by:
    /// - Pre-allocating entity IDs in a single atomic operation
    /// - Getting the archetype once for all entities
    /// - Minimizing index update overhead
    ///
    /// # Performance
    ///
    /// For adding many entities of the same type, this method is significantly faster
    /// than calling `add_entity()` repeatedly due to reduced atomic operations and
    /// archetype lookups.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    pub fn add_entities<E: Extractable>(
        &self,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityId> {
        let entities: Vec<E> = entities.into_iter().collect();
        let count = entities.len();

        if count == 0 {
            return Vec::new();
        }

        // Pre-allocate entity IDs in bulk (single atomic operation)
        let start_id = self
            .next_entity_id
            .fetch_add(count as u32, Ordering::Relaxed);

        // Get archetype once for all entities
        let archetype_id = ArchetypeId::of::<E>();
        let archetype = self.get_archetype::<E>();

        // Pre-allocate result Vec
        let mut entity_ids = Vec::with_capacity(count);

        // Add all entities
        for (i, entity) in entities.into_iter().enumerate() {
            let entity_id = EntityId::new(start_id + i as u32);
            archetype.add_entity(entity_id, entity);
            self.entity_index.insert(entity_id, archetype_id);
            entity_ids.push(entity_id);
        }

        entity_ids
    }

    /// Extract a specific component from an entity.
    ///
    /// Returns `Ok(Acquirable<T>)` if the component was found.
    /// Returns `Err(WorldError::EntityNotFound)` if the entity doesn't exist.
    /// Returns `Err(WorldError::ComponentNotFound)` if the component type doesn't exist on the entity.
    ///
    /// # Example
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Debug, Extractable)]
    /// struct Entity {
    ///     name: String,
    /// }
    ///
    /// #[derive(Debug, Extractable)]
    /// #[extractable(entity)]
    /// struct Player {
    ///     entity: Entity,
    ///     health: u32,
    /// }
    ///
    /// let world = World::new();
    /// let player_id = world.add_entity(Player {
    ///     entity: Entity { name: "Alice".to_string() },
    ///     health: 100,
    /// });
    ///
    /// // Extract the Entity component from Player
    /// let entity = world.extract_component::<Entity>(&player_id).unwrap();
    /// assert_eq!(entity.name, "Alice");
    ///
    /// // Extract the whole Player
    /// let player = world.extract_component::<Player>(&player_id).unwrap();
    /// assert_eq!(player.health, 100);
    /// ```
    pub fn extract_component<T: 'static>(
        &self,
        entity_id: &EntityId,
    ) -> Result<Acquirable<T>, WorldError> {
        let archetype = self
            .get_archetype_by_entity(entity_id)
            .ok_or(WorldError::EntityNotFound(*entity_id))?;

        archetype
            .extract_entity(entity_id)
            .ok_or(WorldError::ComponentNotFound {
                entity_id: *entity_id,
                component_name: std::any::type_name::<T>(),
            })
    }

    /// Remove an entity from the world.
    ///
    /// Returns `Ok(())` if the entity was removed successfully.
    /// Returns `Err(WorldError::EntityNotFound)` if the entity doesn't exist.
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    ///
    /// # Errors
    ///
    /// Returns `WorldError::EntityNotFound` if the entity doesn't exist in the world.
    ///
    /// # Example
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Debug, Extractable)]
    /// struct Player {
    ///     name: String,
    ///     health: u32,
    /// }
    ///
    /// let world = World::new();
    /// let player_id = world.add_entity(Player {
    ///     name: "Alice".to_string(),
    ///     health: 100,
    /// });
    ///
    /// assert_eq!(world.entity_count(), 1);
    ///
    /// // Remove the entity
    /// world.remove_entity(&player_id).unwrap();
    /// assert_eq!(world.entity_count(), 0);
    /// ```
    pub fn remove_entity(&self, entity_id: &EntityId) -> Result<(), WorldError> {
        let archetype_id = self
            .entity_index
            .remove(entity_id)
            .map(|(_, id)| id)
            .ok_or(WorldError::EntityNotFound(*entity_id))?;

        if let Some(archetype) = self.archetypes.get(&archetype_id) {
            archetype
                .remove_entity(entity_id)
                .ok_or(WorldError::ArchetypeNotFound(*entity_id))?;
            Ok(())
        } else {
            Err(WorldError::ArchetypeNotFound(*entity_id))
        }
    }

    /// Remove multiple entities from the world in batch.
    ///
    /// Returns `Ok(())` if all entities were removed successfully.
    /// Returns `Err(WorldError::PartialRemoval)` if some entities failed to remove.
    /// Non-existent entities are treated as failures.
    ///
    /// This method is optimized for bulk deletion by:
    /// - Grouping entities by archetype to minimize archetype lookups
    /// - Batch-removing entities from each archetype
    ///
    /// # Performance
    ///
    /// For removing many entities, this method is more efficient than calling
    /// `remove_entity()` repeatedly because it processes entities in archetype
    /// groups, reducing overhead.
    ///
    /// If you don't need error tracking and want to avoid allocations,
    /// use `remove_entities()` instead.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    ///
    /// # Errors
    ///
    /// Returns `WorldError::PartialRemoval` with information about which entities
    /// were successfully removed and which failed.
    ///
    /// # Example
    ///
    /// ```
    /// use structecs::{*, WorldError};
    ///
    /// #[derive(Debug, Extractable)]
    /// struct Player {
    ///     name: String,
    ///     health: u32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// // Add multiple players
    /// let mut ids = vec![];
    /// for i in 0..5 {
    ///     let id = world.add_entity(Player {
    ///         name: format!("Player{}", i),
    ///         health: 100,
    /// });
    ///     ids.push(id);
    /// }
    ///
    /// assert_eq!(world.entity_count(), 5);
    ///
    /// // Remove first 3 entities
    /// world.try_remove_entities(&ids[0..3]).unwrap();
    /// assert_eq!(world.entity_count(), 2);
    ///
    /// // Try to remove with non-existent entity
    /// let mixed_ids = vec![ids[3], EntityId::from_raw(9999)];
    /// match world.try_remove_entities(&mixed_ids) {
    ///     Err(WorldError::PartialRemoval { succeeded, failed }) => {
    ///         assert_eq!(succeeded.len(), 1);
    ///         assert_eq!(failed.len(), 1);
    ///     }
    ///     _ => panic!("Expected PartialRemoval error"),
    /// }
    /// ```
    pub fn try_remove_entities(&self, entity_ids: &[EntityId]) -> Result<(), WorldError> {
        // Group entity IDs by archetype
        let mut archetype_groups: FxHashMap<ArchetypeId, Vec<EntityId>> = FxHashMap::default();
        let mut not_found = Vec::new();

        for entity_id in entity_ids {
            if let Some((_, archetype_id)) = self.entity_index.remove(entity_id) {
                archetype_groups
                    .entry(archetype_id)
                    .or_default()
                    .push(*entity_id);
            } else {
                not_found.push(*entity_id);
            }
        }

        // Remove entities from each archetype
        let mut removed = Vec::new();
        let mut failed = not_found;

        for (archetype_id, entities) in archetype_groups {
            if let Some(archetype) = self.archetypes.get(&archetype_id) {
                for entity_id in entities {
                    if archetype.remove_entity(&entity_id).is_some() {
                        removed.push(entity_id);
                    } else {
                        failed.push(entity_id);
                    }
                }
            } else {
                failed.extend(entities);
            }
        }

        if failed.is_empty() {
            Ok(())
        } else {
            Err(WorldError::PartialRemoval {
                succeeded: removed,
                failed,
            })
        }
    }

    /// Remove multiple entities from the world in batch without error tracking.
    ///
    /// This is a zero-allocation variant of `try_remove_entities()` that silently skips
    /// non-existent entities. Use this method when you don't need to know which
    /// entities failed to remove and want maximum performance.
    ///
    /// # Performance
    ///
    /// This method is more efficient than `try_remove_entities()` because it:
    /// - Does not allocate vectors to track succeeded/failed entities
    /// - Groups entities by archetype to minimize archetype lookups
    /// - Silently skips non-existent entities without error tracking overhead
    ///
    /// For bulk deletions where you don't care about individual failures,
    /// this method provides the best performance.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called concurrently from multiple threads.
    ///
    /// # Example
    ///
    /// ```
    /// use structecs::*;
    ///
    /// #[derive(Debug, Extractable)]
    /// struct Player {
    ///     name: String,
    ///     health: u32,
    /// }
    ///
    /// let world = World::new();
    ///
    /// // Add multiple players
    /// let mut ids = vec![];
    /// for i in 0..10 {
    ///     let id = world.add_entity(Player {
    ///         name: format!("Player{}", i),
    ///         health: 100,
    ///     });
    ///     ids.push(id);
    /// }
    ///
    /// assert_eq!(world.entity_count(), 10);
    ///
    /// // Fast batch removal - silently skips non-existent entities
    /// ids.push(EntityId::from_raw(9999)); // Add non-existent ID
    /// world.remove_entities(&ids);
    ///
    /// // All valid entities removed, invalid ones silently skipped
    /// assert_eq!(world.entity_count(), 0);
    /// ```
    pub fn remove_entities(&self, entity_ids: &[EntityId]) {
        // Group entity IDs by archetype (only allocates one HashMap)
        let mut archetype_groups: FxHashMap<ArchetypeId, Vec<EntityId>> = FxHashMap::default();

        for entity_id in entity_ids {
            if let Some((_, archetype_id)) = self.entity_index.remove(entity_id) {
                archetype_groups
                    .entry(archetype_id)
                    .or_default()
                    .push(*entity_id);
            }
            // Silently skip non-existent entities
        }

        // Remove entities from each archetype
        for (archetype_id, entities) in archetype_groups {
            if let Some(archetype) = self.archetypes.get(&archetype_id) {
                for entity_id in entities {
                    // Silently ignore removal failures
                    let _ = archetype.remove_entity(&entity_id);
                }
            }
            // Silently skip if archetype not found
        }
    }

    pub fn query<T: 'static>(&self) -> crate::query::QueryIter<T> {
        crate::query::QueryIter::new(self)
    }

    /// Get the number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entity_index.len()
    }

    /// Get the number of archetypes in the world.
    pub fn archetype_count(&self) -> usize {
        self.archetypes.len()
    }

    /// Check if an entity exists in the world.
    pub fn contains_entity(&self, entity_id: &EntityId) -> bool {
        self.entity_index.contains_key(entity_id)
    }

    /// Remove all entities from the world.
    ///
    /// This method clears all entities, archetypes, and the type index,
    /// resetting the world to an empty state. The entity ID counter is NOT reset.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe but should typically be called when no other
    /// operations are in progress for best performance.
    pub fn clear(&self) {
        self.entity_index.clear();
        self.archetypes.clear();
        self.type_index.clear();
    }
}
