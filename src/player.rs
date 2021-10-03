use amethyst::{
    core::{Parent, SystemBundle, Transform},
    ecs::*,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{Camera, Sprite, SpriteRender},
    shred::Fetch,
    shred::World,
    utils::fps_counter::FpsCounter,
    window::ScreenDimensions,
    Error,
};
use nalgebra::Vector2;
use ncollide2d::shape::{Ball, Cuboid, ShapeHandle};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{
    assets::SpriteStorage,
    delivery::{PlayerDeliveryArrowSystem, PlayerDeliverySystem, PlayerJumpSystem},
    physics::{Physics, PhysicsDesc, PhysicsHandle},
    tractor::{PlayerTractorSystem, TractorGravitySystem},
};

#[derive(Debug, PartialEq)]
pub enum PlayerState {
    Active,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Player {
    pub state: PlayerState,
}

fn initialize_camera(
    builder: impl Builder,
    screen_dimensions: (f32, f32),
    player: &Entity,
) -> Entity {
    // Setup camera in a way that our screen covers whole arena and (0, 0) is in the bottom left.
    let mut transform = Transform::default();
    transform.set_translation_xyz(0.0, 0.0, 100.0);

    builder
        .with(Camera::standard_2d(
            screen_dimensions.0,
            screen_dimensions.1,
        ))
        .with(Parent { entity: *player })
        .with(transform)
        .build()
}

pub fn initialize_player(world: &mut World, transform: Transform) {
    let spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.sprites.clone()
    };
    let body = RigidBodyDesc::new()
        .mass(500.0)
        .linear_damping(1.0)
        .status(BodyStatus::Dynamic);
    let shape = ShapeHandle::new(Ball::new(8.0));
    let collider = ColliderDesc::new(shape);
    let player = world
        .create_entity()
        .with(Player {
            state: PlayerState::Active,
        })
        .with(SpriteRender::new(spritesheet, 0))
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .build();
    let (width, height) = {
        let dimensions = world.read_resource::<ScreenDimensions>();
        (dimensions.width(), dimensions.height())
    };
    initialize_camera(world.create_entity(), (width, height), &player);
}

struct PlayerMovementSystem;
impl<'s> System<'s> for PlayerMovementSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, Physics>,
        ReadStorage<'s, PhysicsHandle>,
        WriteStorage<'s, Player>,
        Entities<'s>,
        Read<'s, FpsCounter>,
    );

    fn run(&mut self, (input, mut physics, handles, mut player, entities, fps): Self::SystemData) {
        let x_tilt = input.axis_value("leftright");
        let y_tilt = input.axis_value("updown");
        let boost = input.action_is_down("boost").unwrap_or(false);
        if let (Some(x_tilt), Some(y_tilt)) = (x_tilt, y_tilt) {
            if let Some((entity, handle, player)) = (&entities, &handles, &mut player).join().next()
            {
                if player.state != PlayerState::Active {
                    return;
                }
                let position = physics.get_position(handle).unwrap();
                let speed = if boost { 200_000.0 } else { 100_000.0 };
                physics.apply_force(
                    handle,
                    position
                        .rotation
                        .transform_vector(&Vector2::new(0.0, y_tilt * speed)),
                );
                physics.set_angular_velocity(handle, -x_tilt);
            }
        }
        // println!("{}", fps.sampled_fps());
    }
}

pub struct PlayerBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PlayerBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(PlayerMovementSystem, "player_movement", &[]);
        dispatcher.add(PlayerTractorSystem, "player_tractor", &[]);
        dispatcher.add(PlayerDeliverySystem, "player_delivery", &[]);
        dispatcher.add(PlayerJumpSystem, "player_jump", &["player_delivery"]);
        dispatcher.add(PlayerDeliveryArrowSystem, "player_delivery_arrow", &[]);
        dispatcher.add(TractorGravitySystem, "tractor_gravity", &[]);
        Ok(())
    }
}
