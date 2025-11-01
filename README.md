# structecs

**A flexible entity-component framework without the System.**

Manage your data like ECS, control your logic like OOP.

---

## ⚠️ Development Status

This crate is currently under active development. The API is not stable and may change significantly.

Current focus:

- Performance optimization
- Multi-threading support

---

## What is structecs?

`structecs` is an ECS-inspired data management framework designed for complex applications like game servers where traditional ECS systems can be limiting.

Unlike conventional ECS frameworks (Bevy, specs, hecs), structecs:

- ✅ **No rigid System architecture** - Write your logic however you want
- ✅ **Hierarchical components** - Nest components naturally like OOP
- ✅ **Dynamic type extraction** - Query for any component type on-the-fly
- ✅ **Zero-cost abstractions** - Uses compile-time offsets for component access

### When to use structecs?

**Good for:**

- Complex game servers (Minecraft, MMOs) with intricate entity relationships
- Applications where game logic doesn't fit cleanly into Systems
- Projects transitioning from OOP to data-oriented design
- Scenarios requiring flexible, ad-hoc data access patterns

**Not ideal for:**

- Simple games where traditional ECS works well
- Projects heavily invested in existing ECS ecosystems
- Use cases requiring maximum cache coherency (archetype-based storage coming soon)

---

## Core Concepts

### 1. Entity

An `Entity` is just an ID - a lightweight handle to your data.

```rust
pub struct EntityId {
    id: u32,
}
```

Entities don't "own" components. Instead, they reference structured data stored in the `World`.

### 2. Component (via Extractable)

In structecs, components are **fields within structs**. The `Extractable` trait allows the framework to understand your data structure and extract specific types.

```rust
pub trait Extractable: 'static + Sized {
    const METADATA_LIST: &'static [ExtractionMetadata];
}
```

**Key insight:** Components are hierarchical. A `Player` might contain an `Entity`, which itself is extractable.

#### ExtractionMetadata

Describes how to extract types from your data:

```rust
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
```

This metadata is generated at compile-time by the derive macro, enabling zero-cost component access using memory offsets.

### 3. World

The central data store that manages all entities and their data.

```rust
pub struct World {
    entities: HashMap<EntityId, EntityData>,
    extractors: HashMap<TypeId, Arc<Extractor>>,
    next_entity_id: u32,
}
```

**Core operations:**

- `add_entity<E: Extractable>(entity: E) -> EntityId` - Register new entity
- `query<T: 'static>() -> Vec<(&EntityId, Acquirable<T>)>` - Find all entities with component T
- `extract_component<T>(entity_id) -> Option<Acquirable<T>>` - Get specific component

### 4. Acquirable

A smart reference to a component that keeps the underlying entity data alive.

```rust
pub struct Acquirable<T: 'static> {
    target: NonNull<T>,
    inner: EntityDataInner,
    // ...
}
```

**Features:**

- Implements `Deref<Target = T>` for transparent access
- Reference-counted to prevent use-after-free
- Can `extract()` other component types from the same entity

This enables OOP-like method chaining:

```rust
entity.extract::<Player>()?.extract::<Health>()?
```

### 5. Extractor

The engine that performs type extraction using pre-computed offsets.

```rust
pub struct Extractor {
    offsets: HashMap<TypeId, usize>,
    dropper: unsafe fn(NonNull<u8>),
}
```

Each unique entity structure gets one `Extractor` (cached in `World`), which knows:

- Where each component type lives in memory (offset)
- How to safely drop the entity when done

---

## Architecture

### Memory Layout

```
Player struct in memory:
┌─────────────────────────────┐
│ Entity { name: String }     │ ← offset 0: Entity
│  ├─ name: String            │ ← offset 0: String
├─────────────────────────────┤
│ health: u32                 │ ← offset X: u32
└─────────────────────────────┘

The Extractor knows:
- TypeId(Entity) -> offset 0
- TypeId(String) -> offset 0  (flattened from Entity)
- TypeId(u32) -> offset X
```

### Data Flow

1. **Entity Registration:**

   ```
   User creates struct → Derive macro generates METADATA_LIST
   → add_entity() creates Extractor → Data stored in World
   ```

2. **Component Query:**

   ```
   query<T>() → Iterate all entities → Check if Extractor has TypeId(T)
   → Calculate pointer via offset → Wrap in Acquirable
   ```

3. **Component Extraction:**

   ```
   Acquirable<A>.extract<B>() → Reuse same Extractor
   → Get offset for TypeId(B) → Return new Acquirable<B>
   ```

### Design Philosophy

**"Data is hierarchical, access is flat"**

- Store entities as natural Rust structs (hierarchical)
- Query any component type regardless of nesting (flat access)
- No forced System architecture (user controls logic flow)

This gives you:

- **Expressiveness** of OOP (nested data, clear relationships)
- **Performance** of data-oriented design (offset-based access, no virtual dispatch)
- **Flexibility** of procedural code (write systems however you want)

---

## Comparison with Traditional ECS

| Aspect | Traditional ECS | structecs |
|--------|----------------|-----------|
| **Entity** | Opaque ID | Opaque ID ✓ |
| **Component** | Standalone data types | Fields in structs |
| **System** | First-class concept with scheduling | User implements freely |
| **Data Layout** | Archetype/sparse sets | Per-entity structs |
| **Query Pattern** | Compile-time system parameters | Runtime extraction |
| **Nesting** | Components are flat | Components can nest |
| **Cache Coherency** | Excellent (packed arrays) | Moderate (under optimization) |
| **Flexibility** | Constrained by System API | Maximum flexibility |

---

## Development Roadmap

### Phase 1: Performance (Current)

- [ ] Archetype-based storage for better cache locality
- [ ] Iterator-based queries (eliminate Vec allocation)
- [ ] Query result caching

### Phase 2: Multi-threading

- [ ] Parallel query execution
- [ ] Read/Write access separation
- [ ] Lock-free World operations where possible

### Phase 3: Features

- [ ] Entity removal
- [ ] Dynamic component add/remove
- [ ] Event system
- [ ] Query filtering and composition

---

## Motivation

This framework was created for building a Minecraft server in Rust, where:

- Entity relationships are complex (Player ⊂ LivingEntity ⊂ Entity)
- Game logic is too varied to fit into rigid Systems
- OOP patterns are familiar but Rust's ownership makes traditional OOP difficult

structecs bridges the gap: data-oriented storage with OOP-like access patterns.

---

## License

Licensed under:

- MIT License

---

## Contributing

This project is in early development. Feedback, ideas, and contributions are welcome!
