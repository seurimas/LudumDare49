use amethyst::{
    core::{math::Vector3, SystemBundle, Transform},
    ecs::*,
    prelude::*,
    shrev::EventChannel,
    Error,
};
use nalgebra::Vector2;
use ncollide2d::{
    narrow_phase::ProximityEvent,
    query::Proximity,
    shape::{Cuboid, ShapeHandle},
};
use nphysics2d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

use crate::{
    asteroid::Asteroid,
    physics::{Physics, PhysicsDesc, PhysicsHandle, PhysicsProximityEvent},
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Boundaries {
    width: f32,
    height: f32,
}

pub fn generate_boundaries(world: &mut World, size: (f32, f32)) {
    let body = RigidBodyDesc::new().status(BodyStatus::Static);

    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0 / 2.0, size.1)));
    let collider = ColliderDesc::new(shape).sensor(true);
    let mut transform = Transform::default();
    transform.set_translation(Vector3::new(size.0 * 1.05, 0.0, 0.0));
    world
        .create_entity()
        .with(PhysicsDesc::new(body.clone(), collider))
        .with(transform)
        .with(Boundaries {
            width: size.0,
            height: size.1,
        })
        .build();

    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0 / 2.0, size.1)));
    let collider = ColliderDesc::new(shape).sensor(true);
    let mut transform = Transform::default();
    transform.set_translation(Vector3::new(size.0 * -1.05, 0.0, 0.0));
    world
        .create_entity()
        .with(PhysicsDesc::new(body.clone(), collider))
        .with(transform)
        .with(Boundaries {
            width: size.0,
            height: size.1,
        })
        .build();

    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0, size.1 / 2.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    let mut transform = Transform::default();
    transform.set_translation(Vector3::new(0.0, size.1 * 1.05, 0.0));
    world
        .create_entity()
        .with(PhysicsDesc::new(body.clone(), collider))
        .with(transform)
        .with(Boundaries {
            width: size.0,
            height: size.1,
        })
        .build();

    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(size.0, size.1 / 2.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    let mut transform = Transform::default();
    transform.set_translation(Vector3::new(0.0, size.1 * -1.05, 0.0));
    world
        .create_entity()
        .with(PhysicsDesc::new(body.clone(), collider))
        .with(transform)
        .with(Boundaries {
            width: size.0,
            height: size.1,
        })
        .build();
}

#[derive(Default)]
pub struct AsteroidReintroductionSystem {
    reader: Option<ReaderId<PhysicsProximityEvent>>,
}

fn reintroduce(
    physics: &mut Physics,
    boundary: &Boundaries,
    handle: &PhysicsHandle,
    asteroid: Entity,
) {
    let (x, y, vx, vy) = if rand::random() {
        // Top/Bottom
        if rand::random() {
            (
                rand::random::<f32>() * boundary.width - (boundary.width / 2.0),
                boundary.height / 2.0,
                rand::random::<f32>(),
                -rand::random::<f32>(),
            )
        } else {
            (
                rand::random::<f32>() * boundary.width - (boundary.width / 2.0),
                -boundary.height / 2.0,
                rand::random::<f32>(),
                rand::random::<f32>(),
            )
        }
    } else {
        // Left/Right
        if rand::random() {
            (
                -boundary.width / 2.0,
                rand::random::<f32>() * boundary.height - (boundary.height / 2.0),
                rand::random::<f32>(),
                rand::random::<f32>(),
            )
        } else {
            (
                boundary.width / 2.0,
                rand::random::<f32>() * boundary.height - (boundary.height / 2.0),
                -rand::random::<f32>(),
                rand::random::<f32>(),
            )
        }
    };
    physics.set_location(&handle, x, y);
    let current_speed = physics.get_velocity(&handle).unwrap().magnitude();
    physics.set_velocity(&handle, Vector2::new(vx, vy).normalize() * current_speed);
}

struct DummySystem;
impl<'s> System<'s> for DummySystem {
    type SystemData = (ReadStorage<'s, Boundaries>,);

    fn run(&mut self, _: Self::SystemData) {}
}

impl<'s> System<'s> for AsteroidReintroductionSystem {
    type SystemData = (
        ReadStorage<'s, Asteroid>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Boundaries>,
        Entities<'s>,
        Read<'s, EventChannel<PhysicsProximityEvent>>,
        Write<'s, Physics>,
    );

    fn setup(&mut self, world: &mut World) {
        self.reader = Some(
            world
                .write_resource::<EventChannel<PhysicsProximityEvent>>()
                .register_reader(),
        );
    }

    fn run(
        &mut self,
        (asteroids, handles, boundaries, entities, events, mut physics): Self::SystemData,
    ) {
        if let Some(reader) = &mut self.reader {
            for ProximityEvent {
                collider1,
                collider2,
                new_status,
                prev_status: _,
            } in events.read(reader)
            {
                match new_status {
                    Proximity::Intersecting => {
                        if let (Some(a), Some(b)) = (
                            physics.get_collider_entity(*collider1).cloned(),
                            physics.get_collider_entity(*collider2).cloned(),
                        ) {
                            if let (true, Some(boundary)) =
                                (asteroids.contains(a), boundaries.get(b))
                            {
                                reintroduce(&mut physics, boundary, handles.get(a).unwrap(), a);
                            } else if let (true, Some(boundary)) =
                                (asteroids.contains(b), boundaries.get(a))
                            {
                                reintroduce(&mut physics, boundary, handles.get(b).unwrap(), b);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub struct LevelBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for LevelBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(DummySystem, "boundary_dummy", &[]);
        dispatcher.add(
            AsteroidReintroductionSystem::default(),
            "asteroid_reintroduction",
            &[],
        );
        Ok(())
    }
}
