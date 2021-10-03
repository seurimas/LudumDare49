use amethyst::{
    core::Transform,
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    shred::System,
};
use nalgebra::Point2;

use crate::{
    assets::{SpriteHandles, SpriteRes},
    asteroid::{resize_asteroid, Asteroid, AsteroidType},
    economy::Enterprise,
    particles::{emit_particle, random_direction, Particle},
    physics::{Physics, PhysicsHandle},
    player::Player,
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub enum Explosion {
    Combusting {
        strength: f32,
        particles: Vec<usize>,
    },
    Expanding {
        time: f32,
    },
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
    (strength, particles): (f32, Vec<usize>),
) {
    let asteroid = builder
        .with(SpriteRender::new(sprites, 5))
        .with(transform)
        .with(Explosion::Combusting {
            strength,
            particles,
        })
        .build();
}

pub struct ExplosionForceSystem;
impl<'s> System<'s> for ExplosionForceSystem {
    type SystemData = (
        WriteStorage<'s, Explosion>,
        ReadStorage<'s, Transform>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Player>,
        WriteStorage<'s, Asteroid>,
        Entities<'s>,
        Read<'s, LazyUpdate>,
        Write<'s, Enterprise>,
        Write<'s, Physics>,
        SpriteRes<'s>,
    );

    fn run(
        &mut self,
        (
            mut explosions,
            transforms,
            handles,
            players,
            mut asteroids,
            entities,
            update,
            mut enterprise,
            mut physics,
            sprites,
        ): Self::SystemData,
    ) {
        for (explosion, transform) in (&mut explosions, &transforms).join() {
            if let Explosion::Combusting {
                strength,
                particles,
            } = explosion
            {
                let location = transform.translation();
                let particle_count = ((rand::random::<f32>() * 20.0) as usize + 10);
                for _ in 0..particle_count {
                    let direction = random_direction();
                    emit_particle(
                        update.create_entity(&entities),
                        sprites.get_handle(),
                        Particle::explosion(&particles, direction),
                        Point2::new(location.x, location.y),
                    );
                }
                for (handle, player, entity) in (&handles, &players, &entities).join() {
                    if let Some(player_location) = physics.get_location(handle) {
                        let difference = nalgebra::Vector2::new(
                            player_location.x - location.x,
                            player_location.y - location.y,
                        );
                        let mut distance = difference.magnitude();
                        if distance > 100.0 {
                            continue;
                        }
                        enterprise
                            .burn_fuel((*strength / distance / distance / distance / 100.0).into());
                        physics.apply_impulse(
                            handle,
                            difference * (*strength / distance / distance / distance),
                        );
                    }
                }
                for (handle, asteroid, entity) in (&handles, &mut asteroids, &entities).join() {
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
                                AsteroidType::EncasedArtifact => {
                                    asteroid.my_type = AsteroidType::Artifact;
                                    for _ in 0..10 {
                                        let direction = random_direction();
                                        emit_particle(
                                            update.create_entity(&entities),
                                            sprites.get_handle(),
                                            Particle::explosion(&vec![33], direction),
                                            Point2::new(asteroid_location.x, asteroid_location.y),
                                        );
                                    }
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
                                        (*strength, particles.clone()),
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
                        physics.apply_impulse(
                            handle,
                            difference * (*strength / distance / distance / distance),
                        );
                    }
                }
                *explosion = Explosion::Expanding { time: 0.0 };
            }
        }
    }
}
