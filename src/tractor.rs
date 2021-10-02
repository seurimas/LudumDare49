use amethyst::{
    core::{
        math::{Point3, Translation3, UnitQuaternion, Vector2, Vector3},
        Transform,
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
    assets::SpriteStorage,
    asteroid::Asteroid,
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
        .with(SpriteRender::new(sprites, TRACTOR_SPRITE))
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
        Entities<'s>,
    );

    fn run(
        &mut self,
        (input, mut tractors, sprites, update, cameras, dimensions, mut transforms, entities): Self::SystemData,
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
                    println!("No mouse position...");
                    None
                }
            } else {
                println!("No camera...");
                None
            }
        };
        if let Some((tractor, entity)) = (&mut tractors, &entities).join().next() {
            let entity = entity.clone();
            if input.mouse_button_is_down(MouseButton::Left) {
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
            if let Some(location) = location {
                println!("Init!");
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
        WriteStorage<'s, Tint>,
        Entities<'s>,
        Write<'s, Physics>,
    );

    fn run(
        &mut self,
        (tractors, transforms, handles, asteroids, mut tints, entities, mut physics): Self::SystemData,
    ) {
        for (tractor, transform) in (&tractors, &transforms).join() {
            for (handle, _asteroid, entity) in (&handles, &asteroids, &entities).join() {
                let location = transform.translation();
                if let Some(asteroid_location) = physics.get_location(handle) {
                    let difference = nalgebra::Vector2::new(
                        location.x - asteroid_location.x,
                        location.y - asteroid_location.y,
                    );
                    let distance = difference.magnitude();
                    let mut strength = tractor.strength;
                    if distance > 250.0 {
                        tints.remove(entity);
                    } else if distance > 50.0 {
                        tints.insert(entity, Tint(Srgb::new(1.0, 0.0, 0.0).into()));
                        strength = strength * 5.0;
                        physics.apply_dampening(handle, 1.0);
                    } else if distance > 5.0 {
                        tints.insert(entity, Tint(Srgb::new(0.0, 1.0, 0.0).into()));
                        physics.apply_dampening(handle, 5.0);
                    } else {
                        tints.insert(entity, Tint(Srgb::new(0.0, 0.0, 1.0).into()));
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
