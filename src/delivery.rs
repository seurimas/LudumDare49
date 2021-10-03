use amethyst::{
    core::{math::Vector3, Time, Transform},
    ecs::*,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
};
use nalgebra::{Point2, Vector2};
use ncollide2d::shape::{Ball, Cuboid, ShapeHandle};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{
    assets::{SpriteHandles, SpriteRes, SpriteStorage},
    asteroid::Asteroid,
    economy::Enterprise,
    level::Level,
    particles::{emit_particle, random_direction, Particle},
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
    jumping: bool,
    arrow_distance: f32,
    size: (f32, f32),
    corners: Vec<Entity>,
}

impl DeliveryZone {
    pub fn jumped(&self) -> bool {
        self.jumping && self.cooldown.is_none()
    }
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
) -> Entity {
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
        .build()
}

pub fn generate_delivery_zone(world: &mut World, size: (f32, f32), transform: Transform) {
    let spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.sprites.clone()
    };
    let corners = vec![
        DeliveryCorner::TopLeft,
        DeliveryCorner::TopRight,
        DeliveryCorner::BottomLeft,
        DeliveryCorner::BottomRight,
    ]
    .iter()
    .map(|position| {
        generate_delivery_corner(
            world.create_entity(),
            spritesheet.clone(),
            *position,
            size,
            transform.clone(),
        )
    })
    .collect::<Vec<Entity>>();
    let body = RigidBodyDesc::new().status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0 / 2.0, size.1 / 2.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .with(DeliveryZone {
            cooldown: None,
            jumping: false,
            arrow_distance: 200.0,
            corners,
            size,
        })
        .build();
}

const DELIVERY_TIMESTEPS: [(f32, f32); 10] = [
    (1.0, 0.9),
    (1.0, 0.9),
    (2.0, 0.8),
    (2.0, 0.8),
    (2.0, 0.8),
    (3.0, 0.7),
    (5.0, 0.4),
    (6.0, 0.4),
    (7.0, 0.4),
    (8.0, 0.4),
];

pub struct DeliveryAnimationSystem;
impl<'s> System<'s> for DeliveryAnimationSystem {
    type SystemData = (
        WriteStorage<'s, DeliveryZone>,
        ReadStorage<'s, Transform>,
        Read<'s, LazyUpdate>,
        Read<'s, Time>,
        Entities<'s>,
        SpriteRes<'s>,
    );

    fn run(
        &mut self,
        (mut deliveries, transforms, update, time, entities, sprites): Self::SystemData,
    ) {
        for (delivery, transform) in (&mut deliveries, &transforms).join() {
            delivery.cooldown = delivery.cooldown.and_then(|cooldown| {
                if cooldown > time.delta_seconds() {
                    for (timestep, particle_chance) in DELIVERY_TIMESTEPS {
                        if cooldown < timestep {
                            if rand::random::<f32>() > particle_chance {
                                let center_location = transform.translation();
                                let mut direction = random_direction();
                                direction /= f32::max(direction.x.abs(), direction.y.abs());
                                emit_particle(
                                    update.create_entity(&entities),
                                    sprites.get_handle(),
                                    Particle::delivery(direction),
                                    Point2::new(
                                        center_location.x - direction.x * delivery.size.0 / 2.0,
                                        center_location.y - direction.y * delivery.size.1 / 2.0,
                                    ),
                                );
                            }
                            if !delivery.jumping {
                                break;
                            }
                        }
                    }
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
        Read<'s, Level>,
        Write<'s, Enterprise>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Asteroid>,
        Entities<'s>,
        Write<'s, Physics>,
    );

    fn run(
        &mut self,
        (input, mut deliveries, level, mut enterprise, handles, asteroids, entities, physics): Self::SystemData,
    ) {
        if input.action_is_down("deliver").unwrap_or(false) {
            for (delivery, delivery_handle) in (&mut deliveries, &handles).join() {
                if delivery.cooldown.is_some() {
                    continue;
                }
                for (asteroid, handle, entity) in (&asteroids, &handles, &entities).join() {
                    if physics.is_intersecting(delivery_handle, handle) {
                        enterprise.deliver(
                            &level,
                            asteroid.my_type,
                            physics.get_mass(handle).unwrap_or(10.0),
                        );
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
        (input, mut deliveries, mut enterprise, handles, players, entities, mut physics): Self::SystemData,
    ) {
        if input.action_is_down("deliver").unwrap_or(false) {
            for (delivery, delivery_handle) in (&mut deliveries, &handles).join() {
                if delivery.cooldown.is_some() {
                    continue;
                }
                for (player, handle, entity) in (&players, &handles, &entities).join() {
                    if enterprise.try_jump() {
                        delivery.cooldown = Some(8.0);
                        delivery.jumping = true;
                        physics.set_static(handle);
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
