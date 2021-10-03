#![allow(warnings)]
#[macro_use]
extern crate serde;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use amethyst::{
    assets::{AssetStorage, Directory, Processor, ProgressCounter, Source},
    audio::output::init_output,
    core::{HideHierarchySystem, HideHierarchySystemDesc, Transform, TransformBundle},
    ecs::*,
    input::is_close_requested,
    prelude::*,
    renderer::{
        types::DefaultBackend, Camera, RenderDebugLines, RenderFlat2D, RenderToWindow,
        RenderingBundle,
    },
    tiles::RenderTiles2D,
    ui::{RenderUi, UiBundle, UiCreator, UiEventType, UiFinder},
    utils::{application_root_dir, fps_counter::FpsCounterBundle},
    winit::{dpi::LogicalSize, Event, WindowEvent},
    Application, GameData, GameDataBuilder, SimpleState, SimpleTrans, StateData, StateEvent, Trans,
};
use assets::{
    load_level, load_sound_file, load_spritesheet, LevelStorage, LoadingState, SoundStorage,
    SpriteStorage,
};
use asteroid::{generate_asteroid, generate_asteroid_field, AsteroidBundle, AsteroidType};
use billboards::BillboardBundle;
use delivery::DeliveryZone;
use economy::{EconomyBundle, Enterprise};
use level::{generate_boundaries, initialize_level, Level, LevelBundle, LevelHandle};
use particles::ParticleBundle;
use physics::{PhysicsBundle, PhysicsHandle};
use player::{initialize_player, PlayerBundle};
use serde::Deserialize;

use crate::{
    delivery::generate_delivery_zone,
    menu::{CardDesc, MenuState, MenuTransition},
};
mod assets;
mod asteroid;
mod billboards;
mod delivery;
mod economy;
mod explosions;
mod level;
mod menu;
mod particles;
mod physics;
mod player;
mod tractor;

type ASSETS = (SpriteStorage, LevelStorage);

struct GameplayState {
    assets: ASSETS,
    level: LevelHandle,
    enterprise: Enterprise,
}
impl SimpleState for GameplayState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.insert(self.assets.0.clone());
        data.world.insert(self.assets.1.clone());
        data.world.insert(self.enterprise.clone());
        initialize_level(data.world, &self.level);
        data.world.exec(|mut creator: UiCreator<'_>| {
            creator.create("ui/hud.ron", ());
        });
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        match &event {
            StateEvent::Window(event) => match *event {
                Event::WindowEvent { ref event, .. } => match *event {
                    WindowEvent::CloseRequested => Trans::Quit,
                    WindowEvent::Resized(LogicalSize { width, height }) => {
                        data.world.exec(|mut camera: WriteStorage<Camera>| {
                            if let Some(camera) = (&mut camera).join().next() {
                                *camera =
                                    Camera::standard_2d(width as f32 / 2.0, height as f32 / 2.0);
                            }
                        });
                        Trans::None
                    }
                    _ => Trans::None,
                },
                _ => Trans::None,
            },
            _ => Trans::None,
        }
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        // let (entities, names): (Entities<'_>, ReadStorage<'_, Named>) = data.world.system_data();
        // if get_named_entity(&entities, &names, "player").is_none() {
        //     return SimpleTrans::Switch(Box::new(MenuState {
        //         assets: self.assets.clone(),
        //         menu: "game_over.ron",
        //     }));
        // }
        // if get_named_entity(&entities, &names, "pylon").is_none() {
        //     return SimpleTrans::Switch(Box::new(MenuState {
        //         assets: self.assets.clone(),
        //         menu: "game_over.ron",
        //     }));
        // }
        if data.world.exec(|deliveries: ReadStorage<DeliveryZone>| {
            (&deliveries)
                .join()
                .find(|delivery| delivery.jumped())
                .is_some()
        }) {
            let enterprise = { data.world.read_resource::<Enterprise>().deref().clone() };
            return SimpleTrans::Switch(Box::new(MenuState::end_level(
                self.assets.clone(),
                Some(enterprise),
            )));
        }
        SimpleTrans::None
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let display_config_path = app_root.join("assets/display.ron");
    let input_path = app_root.join("assets/input.ron");

    let assets_dir = app_root.join("assets/");

    let game_data = GameDataBuilder::default()
        .with(Processor::<Level>::new(), "level_loader", &[])
        .with_bundle(TransformBundle::new())?
        .with_system_desc(HideHierarchySystemDesc, "hide_hieracry", &[])
        .with_bundle(
            amethyst::input::InputBundle::<amethyst::input::StringBindings>::new()
                .with_bindings_from_file(input_path)?,
        )?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.0, 0.0, 0.0, 1.0]),
                )
                .with_plugin(RenderFlat2D::default())
                .with_plugin(RenderDebugLines::default())
                .with_plugin(RenderUi::default()),
        )?
        .with_bundle(PhysicsBundle)?
        .with_bundle(AsteroidBundle)?
        .with_bundle(EconomyBundle)?
        .with_bundle(BillboardBundle)?
        .with_bundle(ParticleBundle)?
        .with_bundle(LevelBundle)?
        .with_bundle(PlayerBundle)?
        .with_bundle(FpsCounterBundle)?
        .with_bundle(UiBundle::<amethyst::input::StringBindings>::new())?;

    let mut game = Application::new(
        assets_dir,
        LoadingState::with_levels(Directory::new("assets"), "levels/levels.ron")?,
        game_data,
    )?;
    game.run();

    Ok(())
}
