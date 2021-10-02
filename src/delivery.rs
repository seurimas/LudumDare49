use amethyst::{
    core::Transform,
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
};
use nalgebra::Vector2;
use ncollide2d::shape::{Ball, Cuboid, ShapeHandle};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{
    assets::SpriteStorage,
    asteroid::Asteroid,
    physics::{Physics, PhysicsDesc, PhysicsHandle},
};

#[derive(Component, Debug, Clone, Copy)]
#[storage(VecStorage)]
pub enum DeliveryCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct DeliveryZone;

impl DeliveryCorner {
    fn get_offset(&self, size: (f32, f32)) -> (f32, f32) {
        match self {
            Self::TopLeft => (-size.0 / 2.0, -size.1 / 2.0),
            Self::TopRight => (size.0 / 2.0, -size.1 / 2.0),
            Self::BottomLeft => (-size.0 / 2.0, size.1 / 2.0),
            Self::BottomRight => (size.0 / 2.0, size.1 / 2.0),
        }
    }
    fn get_sprite_num(&self) -> usize {
        match self {
            Self::TopLeft => 6,
            Self::TopRight => 7,
            Self::BottomLeft => 8,
            Self::BottomRight => 9,
        }
    }
}

pub fn generate_delivery_corner(
    builder: impl Builder,
    sprites: SpriteSheetHandle,
    position: DeliveryCorner,
    size: (f32, f32),
    mut transform: Transform,
) {
    let body = RigidBodyDesc::new().status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(6.0, 6.0)));
    let collider = ColliderDesc::new(shape);
    let (dx, dy) = position.get_offset(size);
    transform.move_right(dx);
    transform.move_down(dy);
    builder
        .with(SpriteRender::new(sprites, position.get_sprite_num()))
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        // .with(position)
        .build();
}

pub fn generate_delivery_zone(world: &mut World, size: (f32, f32), transform: Transform) {
    let spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.sprites.clone()
    };
    for position in vec![
        DeliveryCorner::TopLeft,
        DeliveryCorner::TopRight,
        DeliveryCorner::BottomLeft,
        DeliveryCorner::BottomRight,
    ]
    .drain(..)
    {
        generate_delivery_corner(
            world.create_entity(),
            spritesheet.clone(),
            position,
            size,
            transform.clone(),
        );
    }
    let body = RigidBodyDesc::new().status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0 / 2.0, size.1 / 2.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .with(DeliveryZone)
        .build();
}

pub struct PlayerDeliverySystem;
impl<'s> System<'s> for PlayerDeliverySystem {
    type SystemData = (
        ReadStorage<'s, DeliveryZone>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Asteroid>,
        Entities<'s>,
        Write<'s, Physics>,
    );

    fn run(&mut self, (deliveries, handles, asteroids, entities, physics): Self::SystemData) {
        if let Some((delivery, delivery_handle)) = (&deliveries, &handles).join().next().clone() {
            for (asteroid, handle, entity) in (&asteroids, &handles, &entities).join() {
                if physics.is_intersecting(delivery_handle, handle) {
                    entities.delete(entity);
                }
            }
        }
    }
}
