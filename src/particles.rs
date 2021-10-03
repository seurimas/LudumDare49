use std::f32::consts::PI;

use amethyst::{
    core::{SystemBundle, Time, Transform},
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    Error,
};
use nalgebra::{Point2, Vector2};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ParticleType {
    TractorHeavy,
    TractorLight,
    TractorPull,
    Delivery,
    Explosion(usize),
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Particle {
    pub my_type: ParticleType,
    pub lifetime: f32,
    pub velocity: (f32, f32),
    pub delta_velocity: (f32, f32),
    pub rotation: f32,
}

impl Particle {
    pub fn tractor_heavy(direction: Vector2<f32>) -> Self {
        Particle {
            my_type: ParticleType::TractorHeavy,
            lifetime: 1.0,
            velocity: (25.0, 0.0),
            delta_velocity: (25.0, 0.0),
            rotation: f32::atan2(direction.y, direction.x),
        }
    }
    pub fn tractor_light(direction: Vector2<f32>) -> Self {
        Particle {
            my_type: ParticleType::TractorLight,
            lifetime: 0.2,
            velocity: (50.0, 0.0),
            delta_velocity: (0.0, 5.0 - rand::random::<f32>() * 10.0),
            rotation: f32::atan2(direction.y, direction.x),
        }
    }
    pub fn tractor_pull(direction: Vector2<f32>) -> Self {
        Particle {
            my_type: ParticleType::TractorPull,
            lifetime: 0.1,
            velocity: (75.0, 0.0),
            delta_velocity: (15.0, 15.0 - rand::random::<f32>() * 30.0),
            rotation: f32::atan2(direction.y, direction.x),
        }
    }
    pub fn delivery(direction: Vector2<f32>) -> Self {
        Particle {
            my_type: ParticleType::Delivery,
            lifetime: 0.3,
            velocity: (rand::random::<f32>() * 30.0 + 30.0, 0.0),
            delta_velocity: (rand::random::<f32>() * 30.0, 0.0),
            rotation: f32::atan2(direction.y, direction.x),
        }
    }
    pub fn explosion(sprite_numbers: &Vec<usize>, direction: Vector2<f32>) -> Self {
        Particle {
            my_type: ParticleType::Explosion(
                *sprite_numbers
                    .get((rand::random::<f32>() * sprite_numbers.len() as f32) as usize)
                    .unwrap(),
            ),
            lifetime: rand::random::<f32>() + 0.2,
            velocity: (rand::random::<f32>() * 100.0 + 100.0, 0.0),
            delta_velocity: (rand::random::<f32>() * -100.0, 0.0),
            rotation: f32::atan2(direction.y, direction.x),
        }
    }
    fn get_sprite_num(&self) -> usize {
        match self.my_type {
            ParticleType::Explosion(sprite_number) => sprite_number,
            ParticleType::TractorHeavy => 12,
            ParticleType::TractorLight => 13,
            ParticleType::TractorPull => 14,
            ParticleType::Delivery => {
                if rand::random() {
                    if rand::random() {
                        33
                    } else {
                        34
                    }
                } else {
                    if rand::random() {
                        35
                    } else {
                        36
                    }
                }
            }
        }
    }
}

pub fn random_direction() -> Vector2<f32> {
    let rotation = rand::random::<f32>() * PI * 2.0;
    Vector2::new(f32::cos(rotation), f32::sin(rotation))
}

pub fn emit_particle(
    builder: impl Builder,
    sprites: SpriteSheetHandle,
    particle: Particle,
    center: Point2<f32>,
) {
    let mut transform = Transform::default();
    transform.set_translation_x(center.x);
    transform.set_translation_y(center.y);
    transform.set_translation_z(1.0);
    transform.set_rotation_2d(particle.rotation);
    builder
        .with(SpriteRender::new(sprites, particle.get_sprite_num()))
        .with(transform)
        .with(particle)
        .build();
}

pub struct ParticleSystem;
impl<'s> System<'s> for ParticleSystem {
    type SystemData = (
        WriteStorage<'s, Particle>,
        WriteStorage<'s, Transform>,
        Entities<'s>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut particles, mut transforms, entities, time): Self::SystemData) {
        let dt = time.delta_seconds();
        for (particle, transform, entity) in (&mut particles, &mut transforms, &entities).join() {
            if particle.lifetime > dt {
                particle.lifetime -= dt;
                particle.velocity.0 += particle.delta_velocity.0 * dt;
                particle.velocity.1 += particle.delta_velocity.1 * dt;
                transform.append_translation_xyz(
                    particle.velocity.0 * dt,
                    particle.velocity.1 * dt,
                    0.0,
                );
            } else {
                entities.delete(entity);
            }
        }
    }
}

pub struct ParticleBundle;
impl<'a, 'b> SystemBundle<'a, 'b> for ParticleBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(ParticleSystem, "particles", &[]);
        Ok(())
    }
}
