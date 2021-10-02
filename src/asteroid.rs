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
    physics::{Physics, PhysicsContactEvent, PhysicsDesc, PhysicsHandle},
};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum AsteroidType {
    Bomb,
    Big,
    Medium,
    Small,
    Bitty,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Asteroid {
    pub my_type: AsteroidType,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub enum Explosion {
    Combusting,
    Expanding { time: f32 },
}

impl Explosion {
    pub fn combusted(&self) -> bool {
        match self {
            Explosion::Combusting => false,
            _ => true,
        }
    }
}

impl AsteroidType {
    pub fn get_sprite_num(&self) -> usize {
        match self {
            AsteroidType::Big => 1,
            AsteroidType::Medium => 2,
            AsteroidType::Small => 3,
            AsteroidType::Bitty => 4,
            // Special
            AsteroidType::Bomb => 10,
        }
    }
    pub fn get_radius(&self) -> f32 {
        match self {
            AsteroidType::Big => 8.0,
            AsteroidType::Medium => 6.0,
            AsteroidType::Small => 4.0,
            AsteroidType::Bitty => 2.0,
            // Special
            AsteroidType::Bomb => 4.0,
        }
    }
    pub fn get_mass(&self) -> f32 {
        match self {
            AsteroidType::Big => 20.0,
            AsteroidType::Medium => 10.0,
            AsteroidType::Small => 5.0,
            AsteroidType::Bitty => 4.0,
            // Special
            AsteroidType::Bomb => 20.0,
        }
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

pub fn generate_explosion(builder: impl Builder, sprites: SpriteSheetHandle, transform: Transform) {
    let asteroid = builder
        .with(SpriteRender::new(sprites, 5))
        .with(transform)
        .with(Explosion::Combusting)
        .build();
}

pub fn generate_asteroid_field(
    world: &mut World,
    size: (f32, f32),
    asteroid_count: usize,
    bomb_count: usize,
    transform: Transform,
) {
    let spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.sprites.clone()
    };
    for _ in 0..asteroid_count {
        let x = rand::random::<f32>() * size.0;
        let y = rand::random::<f32>() * size.1;
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
        transform.set_translation_xyz(x, y, 0.0);
        generate_asteroid(world.create_entity(), spritesheet.clone(), size, transform);
    }
    for _ in 0..bomb_count {
        let x = rand::random::<f32>() * size.0;
        let y = rand::random::<f32>() * size.1;
        let mut transform = transform.clone();
        transform.set_translation_xyz(x, y, 0.0);
        generate_asteroid(
            world.create_entity(),
            spritesheet.clone(),
            AsteroidType::Bomb,
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
                                if asteroid_a.my_type == asteroid_b.my_type
                                    && asteroid_a.my_type == AsteroidType::Bomb
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

pub struct ExplosionForceSystem;
impl<'s> System<'s> for ExplosionForceSystem {
    type SystemData = (
        WriteStorage<'s, Explosion>,
        ReadStorage<'s, Transform>,
        ReadStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Asteroid>,
        Entities<'s>,
        Read<'s, LazyUpdate>,
        Write<'s, Physics>,
    );

    fn run(
        &mut self,
        (mut explosions, transforms, handles, mut asteroids, entities, update, mut physics): Self::SystemData,
    ) {
        for (explosion, transform) in (&mut explosions, &transforms).join() {
            if explosion.combusted() {
                println!("Combusted");
                continue;
            }
            *explosion = Explosion::Expanding { time: 0.0 };
            for (handle, asteroid, entity) in (&handles, &mut asteroids, &entities).join() {
                let location = transform.translation();
                if let Some(asteroid_location) = physics.get_location(handle) {
                    let difference = nalgebra::Vector2::new(
                        asteroid_location.x - location.x,
                        asteroid_location.y - location.y,
                    );
                    let mut distance = difference.magnitude();
                    let mut strength = 50000.0;
                    if distance > 250.0 {
                        strength = 0.0;
                    } else if distance < 25.0 {
                        let mut changed_size = false;
                        match asteroid.my_type {
                            AsteroidType::Big => {
                                asteroid.my_type = AsteroidType::Medium;
                                changed_size = true;
                            }
                            AsteroidType::Medium => {
                                asteroid.my_type = AsteroidType::Small;
                                changed_size = true;
                            }
                            AsteroidType::Small => {
                                asteroid.my_type = AsteroidType::Bitty;
                                changed_size = true;
                            }
                            AsteroidType::Bitty => {
                                entities.delete(entity);
                            }
                            AsteroidType::Bomb => {
                                entities.delete(entity);
                                *explosion = Explosion::Combusting;
                            }
                        }
                        if changed_size {
                            update.exec(resize_asteroid(entity));
                        }
                    }
                    if distance < 10.0 {
                        distance = 10.0;
                    }
                    physics.apply_impulse(handle, difference * (strength / distance / distance));
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
        dispatcher.add(ExplosionForceSystem, "explosion_force", &[]);
        Ok(())
    }
}
