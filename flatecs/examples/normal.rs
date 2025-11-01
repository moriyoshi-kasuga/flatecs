use flatecs::*;

fn main() {
    let mut world = World::default();

    #[derive(Debug, Extractable)]
    struct Entity {
        name: String,
    }

    // Example entity
    #[derive(Debug, Extractable)]
    #[extractable(entity)]
    struct Player {
        entity: Entity,
        health: u32,
    }

    let zombie = Entity {
        name: "Zombie".to_string(),
    };

    world.add_entity(zombie);

    let player = Player {
        entity: Entity {
            name: "Hero".to_string(),
        },
        health: 100,
    };

    world.add_entity(player);

    for (_, entity) in world.query::<Entity>() {
        println!("{:?}", entity);
    }

    for (_, player) in world.query::<Player>() {
        println!("{:?}", player);
    }
}
