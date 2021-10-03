use std::f32::consts::PI;

use amethyst::{
    core::{
        math::{Point3, Translation3, UnitQuaternion, Vector2, Vector3},
        Time, Transform,
    },
    ecs::*,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{
        palette::Srgb, resources::Tint, sprite::SpriteSheetHandle, ActiveCamera, Camera,
        SpriteRender,
    },
    window::ScreenDimensions,
    winit::MouseButton,
};

use crate::{
    assets::{SpriteHandles, SpriteRes, SpriteStorage},
    asteroid::Asteroid,
    economy::Enterprise,
    particles::{emit_particle, random_direction, Particle},
    physics::{Physics, PhysicsDesc, PhysicsHandle},
};

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Tractor {
    pub strength: f32,
    pub attenuation: f32,
}

const TRACTOR_SPRITE: usize = 5;

fn init_tractor(builder: impl Builder, sprites: SpriteSheetHandle, location: Point3<f32>) {
    let transform = Transform::new(
        Translation3::new(location.x, location.y, 0.0),
        UnitQuaternion::identity(),
        Vector3::new(1.0, 1.0, 1.0),
    );
    let mut builder = builder
        .with(transform)
        .with(Tractor {
            strength: 100.0,
            attenuation: 100.0,
        })
        .build();
}

fn move_tractor<'s>(
    entity: Entity,
    tractor: &mut Tractor,
    transforms: &mut WriteStorage<'s, Transform>,
    location: Point3<f32>,
) {
    if let Some(transform) = transforms.get_mut(entity) {
        transform.set_translation_x(location.x);
        transform.set_translation_y(location.y);
    }
}

pub struct PlayerTractorSystem;
impl<'s> System<'s> for PlayerTractorSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        WriteStorage<'s, Tractor>,
        Option<Read<'s, SpriteStorage>>,
        Read<'s, LazyUpdate>,
        ReadStorage<'s, Camera>,
        Option<Read<'s, ScreenDimensions>>,
        WriteStorage<'s, Transform>,
        Write<'s, Enterprise>,
        Read<'s, Time>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            input,
            mut tractors,
            sprites,
            update,
            cameras,
            dimensions,
            mut transforms,
            mut enterprise,
            time,
            entities,
        ): Self::SystemData,
    ) {
        let location = {
            if let Some((transform, camera)) = (&transforms, &cameras).join().next() {
                if let Some((x, y)) = input.mouse_position() {
                    Some(camera.screen_to_world_point(
                        Point3::new(x, y, 0.0),
                        dimensions.unwrap().diagonal(),
                        transform,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some((tractor, entity)) = (&mut tractors, &entities).join().next() {
            let entity = entity.clone();
            if input.mouse_button_is_down(MouseButton::Left) {
                enterprise.eat_fuel(0.5, &time);
                if let Some(location) = location {
                    move_tractor(entity, tractor, &mut transforms, location);
                }
            } else {
                entities.delete(entity);
            }

            let strength_change = input.axis_value("strength");
            let attenuation_change = input.axis_value("attenuation");
            if let (Some(strength_change), Some(attenuation_change)) =
                (strength_change, attenuation_change)
            {
                tractor.strength += strength_change;
                tractor.attenuation += attenuation_change;
            }
        } else if input.mouse_button_is_down(MouseButton::Left) {
            enterprise.eat_fuel(0.05, &time);
            if let Some(location) = location {
                init_tractor(
                    update.create_entity(&entities),
                    sprites.unwrap().sprites.clone(),
                    location,
                );
            }
        }
    }
}

pub struct TractorGravitySystem;
impl<'s> System<'s> for TractorGravitySystem {
    type SystemData = (
        ReadStorage<'s, Tractor>,
        ReadStorage<'s, Transform>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Asteroid>,
        Read<'s, LazyUpdate>,
        Entities<'s>,
        Write<'s, Physics>,
        SpriteRes<'s>,
    );

    fn run(
        &mut self,
        (tractors, transforms, handles, asteroids, update, entities, mut physics, sprites): Self::SystemData,
    ) {
        for (tractor, transform) in (&tractors, &transforms).join() {
            if rand::random::<f32>() > 0.1 {
                let translation = transform.translation();
                let direction = random_direction();
                emit_particle(
                    update.create_entity(&entities),
                    sprites.get_handle(),
                    Particle::tractor_pull(direction),
                    nalgebra::Point2::new(
                        translation.x - direction.x * 30.0,
                        translation.y - direction.y * 30.0,
                    ),
                );
            }
            for (handle, _asteroid, entity) in (&handles, &asteroids, &entities).join() {
                let location = transform.translation();
                if let Some(asteroid_location) = physics.get_location(handle) {
                    let difference = nalgebra::Vector2::new(
                        location.x - asteroid_location.x,
                        location.y - asteroid_location.y,
                    );
                    let distance = difference.magnitude();
                    let mut strength = tractor.strength;
                    if distance > 100.0 {
                        strength = 0.0;
                    } else if distance > 50.0 {
                        if rand::random::<f32>() > 0.9 {
                            emit_particle(
                                update.create_entity(&entities),
                                sprites.get_handle(),
                                Particle::tractor_heavy(difference.normalize()),
                                asteroid_location,
                            );
                        }
                        strength = strength * 5.0;
                        physics.apply_dampening(handle, 1.0);
                    } else if distance > 5.0 {
                        if rand::random::<f32>() > 0.9 {
                            emit_particle(
                                update.create_entity(&entities),
                                sprites.get_handle(),
                                Particle::tractor_light(difference.normalize()),
                                asteroid_location,
                            );
                        }
                        physics.apply_dampening(handle, 5.0);
                    } else {
                        strength = 0.0;
                        physics.apply_dampening(handle, 10.0);
                    }
                    physics.apply_velocity_change(
                        handle,
                        difference * (strength / distance / distance),
                    );
                }
            }
        }
    }
}
