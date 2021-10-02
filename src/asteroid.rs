use std::collections::HashMap;

use amethyst::{
    core::{SystemBundle, Transform},
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    shrev::EventChannel,
    Error,
};
use ncollide2d::{
    narrow_phase::ContactEvent,
    shape::{Ball, ShapeHandle},
};
use nphysics2d::object::{BodyStatus, ColliderDesc, DefaultColliderHandle, RigidBodyDesc};

use crate::{
    assets::{SpriteHandles, SpriteRes, SpriteStorage},
    explosions::{generate_explosion, ExplosionForceSystem},
    physics::{Physics, PhysicsContactEvent, PhysicsDesc, PhysicsHandle},
};

#[derive(Debug, PartialEq, Copy, Clone, Hash, Eq)]
pub enum AsteroidType {
    // Mineral
    Big,
    Medium,
    Small,
    Bitty,
    // Explosive
    Bomb,
    // Reactive
    Hydrogen,
    Oxygen,
    Water,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Asteroid {
    pub my_type: AsteroidType,
}

impl AsteroidType {
    pub fn get_sprite_num(&self) -> usize {
        match self {
            // Mineral
            AsteroidType::Big => 1,
            AsteroidType::Medium => 2,
            AsteroidType::Small => 3,
            AsteroidType::Bitty => 4,
            // Explosive
            AsteroidType::Bomb => 10,
            // Reactive
            AsteroidType::Hydrogen => 27,
            AsteroidType::Oxygen => 28,
            AsteroidType::Water => 29,
        }
    }
    pub fn get_radius(&self) -> f32 {
        match self {
            // Mineral
            AsteroidType::Big => 8.0,
            AsteroidType::Medium => 6.0,
            AsteroidType::Small => 4.0,
            AsteroidType::Bitty => 2.0,
            // Explosive
            AsteroidType::Bomb => 4.0,
            // Reactive
            AsteroidType::Hydrogen => 4.0,
            AsteroidType::Oxygen => 4.0,
            AsteroidType::Water => 4.0,
        }
    }
    pub fn get_mass(&self) -> f32 {
        match self {
            AsteroidType::Big => 20.0,
            AsteroidType::Medium => 10.0,
            AsteroidType::Small => 5.0,
            AsteroidType::Bitty => 4.0,
            // Bomb
            AsteroidType::Bomb => 20.0,
            // Reactive
            AsteroidType::Hydrogen => 4.0,
            AsteroidType::Oxygen => 4.0,
            AsteroidType::Water => 8.0,
        }
    }
    pub fn explodes(&self, other: Self) -> Option<f32> {
        match (self, other) {
            (AsteroidType::Bomb, AsteroidType::Bomb) => Some(500_000.0),
            _ => None,
        }
    }
    pub fn reacts(&self, other: Self) -> Option<(Self, Option<Self>)> {
        match (self, other) {
            (AsteroidType::Hydrogen, AsteroidType::Oxygen)
            | (AsteroidType::Oxygen, AsteroidType::Hydrogen) => Some((AsteroidType::Water, None)),
            _ => None,
        }
    }

    pub fn base_prices() -> HashMap<AsteroidType, f32> {
        let mut prices = HashMap::new();
        prices.insert(AsteroidType::Bitty, 1.0);
        prices.insert(AsteroidType::Small, 1.0);
        prices.insert(AsteroidType::Medium, 1.5);
        prices.insert(AsteroidType::Big, 2.0);
        prices.insert(AsteroidType::Bomb, 5.0);
        prices.insert(AsteroidType::Hydrogen, 1.5);
        prices.insert(AsteroidType::Oxygen, 1.5);
        prices.insert(AsteroidType::Water, 1.0);
        prices
    }
}

pub fn generate_asteroid(
    builder: impl Builder,
    sprites: SpriteSheetHandle,
    size: AsteroidType,
    transform: Transform,
) {
    let body = RigidBodyDesc::new()
        .mass(size.get_mass())
        .status(BodyStatus::Dynamic);
    let shape = ShapeHandle::new(Ball::new(size.get_radius()));
    let collider = ColliderDesc::new(shape);
    let asteroid = builder
        .with(SpriteRender::new(sprites, size.get_sprite_num()))
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .with(Asteroid { my_type: size })
        .build();
}

pub fn resize_asteroid(entity: Entity) -> impl FnOnce(&mut World) + 'static + Sync + Send {
    move |world| {
        world.exec(
            |(mut sprites, asteroids, handles, mut physics): (
                WriteStorage<SpriteRender>,
                ReadStorage<Asteroid>,
                ReadStorage<PhysicsHandle>,
                Write<Physics>,
            )| {
                if let (Some(sprite), Some(asteroid), Some(handle)) = (
                    sprites.get_mut(entity),
                    asteroids.get(entity),
                    handles.get(entity),
                ) {
                    physics.change_shape(
                        handle,
                        ShapeHandle::new(Ball::new(asteroid.my_type.get_radius())),
                    );
                    sprite.sprite_number = asteroid.my_type.get_sprite_num();
                }
            },
        );
    }
}

pub fn generate_asteroid_field(
    world: &mut World,
    size: (f32, f32),
    asteroid_count: usize,
    bomb_count: usize,
    gas_count: usize,
    transform: Transform,
) {
    let spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.sprites.clone()
    };
    for _ in 0..asteroid_count {
        let x = rand::random::<f32>() * size.0 - size.0 / 2.0;
        let y = rand::random::<f32>() * size.1 - size.1 / 2.0;
        let size = {
            if rand::random::<f32>() > 0.9 {
                AsteroidType::Big
            } else if rand::random::<f32>() > 0.9 {
                AsteroidType::Bitty
            } else if rand::random() {
                AsteroidType::Medium
            } else {
                AsteroidType::Small
            }
        };
        let mut transform = transform.clone();
        transform.append_translation_xyz(x, y, 0.0);
        generate_asteroid(world.create_entity(), spritesheet.clone(), size, transform);
    }
    for _ in 0..bomb_count {
        let x = rand::random::<f32>() * size.0;
        let y = rand::random::<f32>() * size.1;
        let mut transform = transform.clone();
        transform.append_translation_xyz(x, y, 0.0);
        generate_asteroid(
            world.create_entity(),
            spritesheet.clone(),
            AsteroidType::Bomb,
            transform,
        );
    }
    for _ in 0..gas_count {
        let x = rand::random::<f32>() * size.0;
        let y = rand::random::<f32>() * size.1;
        let mut transform = transform.clone();
        transform.append_translation_xyz(x, y, 0.0);
        generate_asteroid(
            world.create_entity(),
            spritesheet.clone(),
            if rand::random() {
                AsteroidType::Hydrogen
            } else {
                AsteroidType::Oxygen
            },
            transform,
        );
    }
}

#[derive(Default)]
pub struct AsteroidExplosionSystem {
    reader: Option<ReaderId<ContactEvent<DefaultColliderHandle>>>,
}

impl<'s> System<'s> for AsteroidExplosionSystem {
    type SystemData = (
        Read<'s, EventChannel<PhysicsContactEvent>>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Asteroid>,
        Entities<'s>,
        Write<'s, Physics>,
        Read<'s, LazyUpdate>,
        SpriteRes<'s>,
    );

    fn setup(&mut self, world: &mut World) {
        self.reader = Some(
            world
                .write_resource::<EventChannel<PhysicsContactEvent>>()
                .register_reader(),
        );
    }

    fn run(
        &mut self,
        (events, handles, asteroids, entities, physics, update, sprites): Self::SystemData,
    ) {
        if let Some(reader) = &mut self.reader {
            for event in events.read(reader) {
                match event {
                    ContactEvent::Started(a, b) => {
                        if let (Some(a), Some(b)) = (
                            physics.get_collider_entity(*a),
                            physics.get_collider_entity(*b),
                        ) {
                            if let (Some(asteroid_a), Some(asteroid_b)) =
                                (asteroids.get(*a), asteroids.get(*b))
                            {
                                if let Some(strength) =
                                    asteroid_a.my_type.explodes(asteroid_b.my_type)
                                {
                                    if let Some((Some(location_a), Some(location_b))) = handles
                                        .get(*a)
                                        .and_then(|handle_a| {
                                            handles.get(*b).map(|handle_b| (handle_a, handle_b))
                                        })
                                        .map(|(handle_a, handle_b)| {
                                            (
                                                physics.get_location(handle_a),
                                                physics.get_location(handle_b),
                                            )
                                        })
                                    {
                                        let mut transform = Transform::default();
                                        transform.set_translation_xyz(
                                            (location_a.x + location_b.x) / 2.0,
                                            (location_a.y + location_b.y) / 2.0,
                                            0.0,
                                        );
                                        generate_explosion(
                                            update.create_entity(&entities),
                                            sprites.get_handle(),
                                            transform,
                                            strength,
                                        );
                                    }
                                    entities.delete(*a);
                                    entities.delete(*b);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Default)]
pub struct AsteroidReactionSystem {
    reader: Option<ReaderId<ContactEvent<DefaultColliderHandle>>>,
}

impl<'s> System<'s> for AsteroidReactionSystem {
    type SystemData = (
        Read<'s, EventChannel<PhysicsContactEvent>>,
        ReadStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Asteroid>,
        Entities<'s>,
        Write<'s, Physics>,
        Read<'s, LazyUpdate>,
        SpriteRes<'s>,
    );

    fn setup(&mut self, world: &mut World) {
        self.reader = Some(
            world
                .write_resource::<EventChannel<PhysicsContactEvent>>()
                .register_reader(),
        );
    }

    fn run(
        &mut self,
        (events, handles, mut asteroids, entities, physics, update, sprites): Self::SystemData,
    ) {
        if let Some(reader) = &mut self.reader {
            for event in events.read(reader) {
                match event {
                    ContactEvent::Started(a, b) => {
                        if let (Some(a), Some(b)) = (
                            physics.get_collider_entity(*a),
                            physics.get_collider_entity(*b),
                        ) {
                            let reaction = {
                                if let (Some(asteroid_a), Some(asteroid_b)) =
                                    (asteroids.get(*a), asteroids.get(*b))
                                {
                                    asteroid_a.my_type.reacts(asteroid_b.my_type)
                                } else {
                                    None
                                }
                            };
                            if let Some((reaction_a, reaction_b)) = reaction {
                                asteroids
                                    .get_mut(*a)
                                    .map(|asteroid| asteroid.my_type = reaction_a);
                                update.exec(resize_asteroid(*a));

                                if let Some(reaction_b) = reaction_b {
                                    asteroids
                                        .get_mut(*b)
                                        .map(|asteroid| asteroid.my_type = reaction_b);
                                    update.exec(resize_asteroid(*b));
                                } else {
                                    entities.delete(*b);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub struct AsteroidBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for AsteroidBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(AsteroidExplosionSystem::default(), "asteroid_explode", &[]);
        dispatcher.add(AsteroidReactionSystem::default(), "asteroid_react", &[]);
        dispatcher.add(ExplosionForceSystem, "explosion_force", &[]);
        Ok(())
    }
}
