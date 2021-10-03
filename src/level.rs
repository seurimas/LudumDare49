use std::collections::HashMap;

use amethyst::{
    assets::{Asset, AssetStorage, Handle, ProcessableAsset, ProcessingState},
    core::{math::Vector3, HiddenPropagate, SystemBundle, Transform},
    ecs::*,
    prelude::*,
    renderer::SpriteRender,
    shrev::EventChannel,
    ui::{UiButtonAction, UiEvent, UiEventType, UiImage, UiText, UiTransform},
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
    assets::{LevelStorage, SpriteHandles, SpriteRes, SpriteStorage},
    asteroid::{generate_asteroid_field, Asteroid, AsteroidType},
    billboards::{generate_billboard, BillboardDesc},
    delivery::{generate_delivery_zone, DeliveryAnimationSystem},
    economy::Enterprise,
    menu::{find_by_id, CardDesc},
    physics::{Physics, PhysicsDesc, PhysicsHandle, PhysicsProximityEvent},
    player::initialize_player,
};

#[derive(Serialize, Deserialize, Clone)]
pub enum AsteroidDesc {
    Field {
        location: Option<(f32, f32, f32, f32)>,
        normal: Option<usize>,
        bombs: Option<usize>,
        gases: Option<usize>,
        sulphur: Option<usize>,
        artifacts: Option<usize>,
        debris: Option<(usize, f32)>,
    },
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ReferenceDesc {
    pub name: String,
    description: String,
    shown_prices: Vec<AsteroidType>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Level {
    boundaries: (f32, f32),
    player_start: Option<(f32, f32)>,
    deliveries: Vec<(f32, f32)>,
    pub jump_cost: u64,
    pub card: CardDesc,
    asteroids: Vec<AsteroidDesc>,
    billboards: Vec<BillboardDesc>,
    modified_prices: Option<HashMap<AsteroidType, f32>>,
    pub reference: ReferenceDesc,
}

impl Level {
    pub fn get_ppm(&self, asteroid_type: AsteroidType) -> f32 {
        self.modified_prices
            .as_ref()
            .and_then(|modified_prices| modified_prices.get(&asteroid_type))
            .cloned()
            .unwrap_or(asteroid_type.get_base_ppm())
    }
}

pub type LevelHandle = Handle<Level>;

impl Asset for Level {
    const NAME: &'static str = "ld49::Level";
    type Data = Level;
    type HandleStorage = VecStorage<LevelHandle>;
}

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

pub fn initialize_level(world: &mut World, level: &LevelHandle) {
    let level = {
        world
            .read_resource::<AssetStorage<Level>>()
            .get(level)
            .cloned()
            .unwrap()
    };
    let mut transform = Transform::default();
    let (x, y) = level.player_start.unwrap_or((0.0, 0.0));
    transform.set_translation_x(x);
    transform.set_translation_y(y);
    initialize_player(world, transform);
    for (x, y) in level.deliveries.iter() {
        let mut transform = Transform::default();
        transform.set_translation_x(*x);
        transform.set_translation_y(*y);
        generate_delivery_zone(world, (75.0, 75.0), transform);
    }
    for asteroid_desc in &level.asteroids {
        match asteroid_desc {
            AsteroidDesc::Field {
                location,
                normal,
                bombs,
                gases,
                sulphur,
                artifacts,
                debris,
            } => {
                let mut transform = Transform::default();
                let location =
                    location.unwrap_or((0.0, 0.0, level.boundaries.0, level.boundaries.1));
                transform.set_translation_x(location.0);
                transform.set_translation_y(location.1);
                generate_asteroid_field(
                    world,
                    (location.2, location.3),
                    normal.unwrap_or_default(),
                    bombs.unwrap_or_default(),
                    gases.unwrap_or_default(),
                    sulphur.unwrap_or_default(),
                    artifacts.unwrap_or(1),
                    debris.unwrap_or_default(),
                    transform,
                );
            }
        }
    }
    for billboard_desc in &level.billboards {
        let spritesheet = {
            let sprites = world.read_resource::<SpriteStorage>();
            sprites.sprites.clone()
        };
        generate_billboard(world.create_entity(), spritesheet, billboard_desc);
    }
    generate_boundaries(world, level.boundaries);
    world.insert(level);
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
    if current_speed > 0.0 {
        physics.set_velocity(&handle, Vector2::new(vx, vy).normalize() * current_speed);
    }
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

#[derive(Default)]
pub struct ReferenceCardSystem {
    reader: Option<ReaderId<UiEvent>>,
}
impl<'s> System<'s> for ReferenceCardSystem {
    type SystemData = (
        Read<'s, EventChannel<UiEvent>>,
        WriteStorage<'s, UiTransform>,
        WriteStorage<'s, UiImage>,
        WriteStorage<'s, UiText>,
        WriteStorage<'s, HiddenPropagate>,
        Entities<'s>,
        Read<'s, Level>,
        Read<'s, Enterprise>,
        SpriteRes<'s>,
    );

    fn setup(&mut self, world: &mut World) {
        self.reader = Some(
            world
                .write_resource::<EventChannel<UiEvent>>()
                .register_reader(),
        );
    }

    fn run(
        &mut self,
        (
            events,
            mut transforms,
            mut images,
            mut texts,
            mut hiddens,
            entities,
            level,
            enterprise,
            sprites,
        ): Self::SystemData,
    ) {
        if let Some(reader) = &mut self.reader {
            for event in events.read(reader) {
                let target_name = transforms
                    .get(event.target)
                    .map(|transform| transform.id.clone())
                    .unwrap_or_default();
                if event.event_type != UiEventType::Click {
                    continue;
                }
                match target_name.as_ref() {
                    "show_reference" => {
                        if let (Some(hide), Some(show), Some(reference)) = (
                            find_by_id(&entities, &transforms, "hide_reference"),
                            find_by_id(&entities, &transforms, "show_reference"),
                            find_by_id(&entities, &transforms, "reference_area"),
                        ) {
                            hiddens.remove(reference);
                            hiddens.insert(show, HiddenPropagate::new());
                            hiddens.remove(hide);
                        }
                        if let (Some(level_name), Some(level_description)) = (
                            find_by_id(&entities, &transforms, "level_name"),
                            find_by_id(&entities, &transforms, "level_description"),
                        ) {
                            if let Some(level_name) = texts.get_mut(level_name) {
                                level_name.text = level.reference.name.clone();
                            }
                            if let Some(level_description) = texts.get_mut(level_description) {
                                level_description.text = level.reference.description.clone();
                            }
                        }
                        for idx in 0..6 {
                            let asteroid_id = format!("price_reference_{}_asteroid", idx);
                            let price_id = format!("price_reference_{}_price", idx);
                            if let (Some(asteroid_ref), Some(price_ref)) = (
                                find_by_id(&entities, &transforms, &asteroid_id),
                                find_by_id(&entities, &transforms, &price_id),
                            ) {
                                if let Some(asteroid) = level.reference.shown_prices.get(idx) {
                                    let price = level.get_ppm(*asteroid);
                                    if let Some(asteroid_image) = images.get_mut(asteroid_ref) {
                                        *asteroid_image = UiImage::Sprite(SpriteRender::new(
                                            sprites.get_handle(),
                                            asteroid.get_sprite_num(),
                                        ));
                                    }
                                    if let (Some(price_image), Some(price_transform)) =
                                        (images.get_mut(price_ref), transforms.get_mut(price_ref))
                                    {
                                        if let UiImage::PartialTexture {
                                            tex,
                                            left,
                                            top,
                                            bottom,
                                            ..
                                        } = price_image
                                        {
                                            *price_image = UiImage::PartialTexture {
                                                tex: tex.clone(),
                                                left: *left,
                                                top: *top,
                                                bottom: *bottom,
                                                right: *left + ((8.0 / 512.0) * price),
                                            };
                                        }
                                        price_transform.width = price * 24.0;
                                    }
                                } else {
                                    hiddens.insert(asteroid_ref, HiddenPropagate::new());
                                    hiddens.insert(price_ref, HiddenPropagate::new());
                                }
                            }
                        }
                    }
                    "hide_reference" => {
                        if let (Some(hide), Some(show), Some(reference)) = (
                            find_by_id(&entities, &transforms, "hide_reference"),
                            find_by_id(&entities, &transforms, "show_reference"),
                            find_by_id(&entities, &transforms, "reference_area"),
                        ) {
                            hiddens.insert(reference, HiddenPropagate::new());
                            hiddens.insert(hide, HiddenPropagate::new());
                            hiddens.remove(show);
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
        dispatcher.add(DeliveryAnimationSystem, "delivery_animation", &[]);
        dispatcher.add(ReferenceCardSystem::default(), "reference_card", &[]);
        dispatcher.add(
            AsteroidReintroductionSystem::default(),
            "asteroid_reintroduction",
            &[],
        );
        Ok(())
    }
}
