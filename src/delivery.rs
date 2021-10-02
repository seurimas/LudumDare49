use amethyst::{
    core::{math::Vector3, Time, Transform},
    ecs::*,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
};
use nalgebra::Vector2;
use ncollide2d::shape::{Ball, Cuboid, ShapeHandle};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{
    assets::{SpriteHandles, SpriteRes, SpriteStorage},
    asteroid::Asteroid,
    economy::Enterprise,
    physics::{Physics, PhysicsDesc, PhysicsHandle},
    player::Player,
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
pub struct DeliveryZone {
    cooldown: Option<f32>,
    arrow_distance: f32,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct DeliveryArrow(Entity);

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
        .with(DeliveryZone {
            cooldown: None,
            arrow_distance: 200.0,
        })
        .build();
}

pub struct DeliveryCooldownSystem;
impl<'s> System<'s> for DeliveryCooldownSystem {
    type SystemData = (WriteStorage<'s, DeliveryZone>, Read<'s, Time>);

    fn run(&mut self, (mut deliveries, time): Self::SystemData) {
        for delivery in (&mut deliveries).join() {
            delivery.cooldown = delivery.cooldown.and_then(|cooldown| {
                if cooldown > time.delta_seconds() {
                    Some(cooldown - time.delta_seconds())
                } else {
                    None
                }
            });
        }
    }
}

pub struct PlayerDeliverySystem;
impl<'s> System<'s> for PlayerDeliverySystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        WriteStorage<'s, DeliveryZone>,
        Write<'s, Enterprise>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Asteroid>,
        Entities<'s>,
        Write<'s, Physics>,
    );

    fn run(
        &mut self,
        (input, mut deliveries, mut enterprise, handles, asteroids, entities, physics): Self::SystemData,
    ) {
        if input.action_is_down("deliver").unwrap_or(false) {
            for (delivery, delivery_handle) in (&mut deliveries, &handles).join() {
                if delivery.cooldown.is_some() {
                    continue;
                }
                for (asteroid, handle, entity) in (&asteroids, &handles, &entities).join() {
                    if physics.is_intersecting(delivery_handle, handle) {
                        enterprise
                            .deliver(asteroid.my_type, physics.get_mass(handle).unwrap_or(10.0));
                        entities.delete(entity);
                        delivery.cooldown = Some(5.0);
                    }
                }
            }
        }
    }
}

pub struct PlayerJumpSystem;
impl<'s> System<'s> for PlayerJumpSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        WriteStorage<'s, DeliveryZone>,
        Write<'s, Enterprise>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Player>,
        Entities<'s>,
        Write<'s, Physics>,
    );

    fn run(
        &mut self,
        (input, mut deliveries, mut enterprise, handles, players, entities, physics): Self::SystemData,
    ) {
        if input.action_is_down("deliver").unwrap_or(false) {
            for (delivery, delivery_handle) in (&mut deliveries, &handles).join() {
                if delivery.cooldown.is_some() {
                    continue;
                }
                for (player, handle, entity) in (&players, &handles, &entities).join() {
                    // if physics.is_intersecting(delivery_handle, handle) {
                    //     entities.delete(entity);
                    // }
                    if enterprise.try_jump() {
                        delivery.cooldown = Some(5.0);
                    }
                }
            }
        }
    }
}

pub struct PlayerDeliveryArrowSystem;
impl<'s> System<'s> for PlayerDeliveryArrowSystem {
    type SystemData = (
        ReadStorage<'s, DeliveryZone>,
        ReadStorage<'s, DeliveryArrow>,
        ReadStorage<'s, Transform>,
        ReadStorage<'s, Player>,
        Read<'s, LazyUpdate>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (deliveries, arrows, transforms, players, update, entities): Self::SystemData,
    ) {
        if let Some((_player, player_transform)) = (&players, &transforms).join().next() {
            for (delivery, transform, entity) in (&deliveries, &transforms, &entities).join() {
                let direction = transform.translation() - player_transform.translation();
                update.exec(render_arrow(
                    player_transform.clone(),
                    direction.try_normalize(delivery.arrow_distance),
                    entity,
                ));
            }
        }
    }
}

pub const ARROW: usize = 11;
pub const JUMP_ARROW: usize = 32;

fn arrow_transform(player_transform: Transform, direction: Vector3<f32>) -> Transform {
    let mut transform = player_transform;
    transform.set_rotation_2d(0.0);
    transform.append_translation(direction * 24.0);
    transform.set_scale(Vector3::new(1.5, 1.5, 1.5));
    if direction.magnitude_squared() > 0.0 {
        transform.set_rotation_2d(f32::atan2(direction.y, direction.x));
    }
    transform
}

fn render_arrow(
    player_transform: Transform,
    direction: Option<Vector3<f32>>,
    entity: Entity,
) -> impl Send + Sync + FnOnce(&mut World) + 'static {
    move |world| {
        world.exec(
            |(mut arrows, mut transforms, mut sprite_renders, entities, sprites): (
                WriteStorage<DeliveryArrow>,
                WriteStorage<Transform>,
                WriteStorage<SpriteRender>,
                Entities,
                SpriteRes,
            )| {
                if let Some((_arrow, arrow_entity)) = (&arrows, &entities)
                    .join()
                    .find(|(arrow, _)| arrow.0 == entity)
                {
                    if let Some(direction) = direction {
                        transforms
                            .insert(arrow_entity, arrow_transform(player_transform, direction));
                        sprite_renders.insert(
                            arrow_entity,
                            SpriteRender {
                                sprite_sheet: sprites.get_handle(),
                                sprite_number: JUMP_ARROW,
                            },
                        );
                    } else {
                        entities.delete(arrow_entity);
                    }
                } else if let Some(direction) = direction {
                    entities
                        .build_entity()
                        .with(DeliveryArrow(entity), &mut arrows)
                        .with(
                            arrow_transform(player_transform, direction),
                            &mut transforms,
                        )
                        .with(
                            SpriteRender {
                                sprite_sheet: sprites.get_handle(),
                                sprite_number: JUMP_ARROW,
                            },
                            &mut sprite_renders,
                        )
                        .build();
                }
            },
        );
    }
}
