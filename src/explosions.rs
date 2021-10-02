use amethyst::{
    core::Transform,
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    shred::System,
};

use crate::{
    assets::{SpriteHandles, SpriteRes},
    asteroid::{resize_asteroid, Asteroid, AsteroidType},
    physics::{Physics, PhysicsHandle},
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub enum Explosion {
    Combusting { strength: f32 },
    Expanding { time: f32 },
}

impl Explosion {
    pub fn combusted(&self) -> bool {
        match self {
            Explosion::Combusting { .. } => false,
            _ => true,
        }
    }
}

pub fn generate_explosion(
    builder: impl Builder,
    sprites: SpriteSheetHandle,
    transform: Transform,
    strength: f32,
) {
    let asteroid = builder
        .with(SpriteRender::new(sprites, 5))
        .with(transform)
        .with(Explosion::Combusting { strength })
        .build();
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
        SpriteRes<'s>,
    );

    fn run(
        &mut self,
        (
            mut explosions,
            transforms,
            handles,
            mut asteroids,
            entities,
            update,
            mut physics,
            sprites,
        ): Self::SystemData,
    ) {
        for (explosion, transform) in (&mut explosions, &transforms).join() {
            if let Explosion::Combusting { strength } = explosion {
                for (handle, asteroid, entity) in (&handles, &mut asteroids, &entities).join() {
                    let location = transform.translation();
                    if let Some(asteroid_location) = physics.get_location(handle) {
                        let difference = nalgebra::Vector2::new(
                            asteroid_location.x - location.x,
                            asteroid_location.y - location.y,
                        );
                        let mut distance = difference.magnitude();
                        if distance < 25.0 {
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
                                    generate_explosion(
                                        update.create_entity(&entities),
                                        sprites.get_handle(),
                                        transform.clone(),
                                        *strength,
                                    );
                                }
                                _ => {}
                            }
                            if changed_size {
                                update.exec(resize_asteroid(entity));
                            }
                        }
                        if distance < 10.0 {
                            distance = 10.0;
                        }
                        physics
                            .apply_impulse(handle, difference * (*strength / distance / distance));
                    }
                }
                *explosion = Explosion::Expanding { time: 0.0 };
            }
        }
    }
}
