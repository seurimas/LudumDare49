use amethyst::{
    core::Transform,
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
};
use ncollide2d::shape::{Ball, ShapeHandle};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{assets::SpriteStorage, physics::PhysicsDesc};

#[derive(Debug, Copy, Clone)]
pub enum AsteroidSize {
    Big,
    Medium,
    Small,
    Bitty,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Asteroid {
    pub size: AsteroidSize,
}

impl AsteroidSize {
    pub fn get_sprite_num(&self) -> usize {
        match self {
            AsteroidSize::Big => 1,
            AsteroidSize::Medium => 2,
            AsteroidSize::Small => 3,
            AsteroidSize::Bitty => 4,
        }
    }
    pub fn get_radius(&self) -> f32 {
        match self {
            AsteroidSize::Big => 4.0,
            AsteroidSize::Medium => 3.0,
            AsteroidSize::Small => 2.0,
            AsteroidSize::Bitty => 1.0,
        }
    }
    pub fn get_mass(&self) -> f32 {
        match self {
            AsteroidSize::Big => 20.0,
            AsteroidSize::Medium => 10.0,
            AsteroidSize::Small => 5.0,
            AsteroidSize::Bitty => 4.0,
        }
    }
}

pub fn generate_asteroid(
    builder: impl Builder,
    sprites: SpriteSheetHandle,
    size: AsteroidSize,
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
        .with(Asteroid { size })
        .build();
}
