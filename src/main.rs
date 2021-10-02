#[macro_use]
extern crate serde;
use std::path::{Path, PathBuf};

use amethyst::{
    assets::{AssetStorage, Directory, Processor, ProgressCounter, Source},
    audio::output::init_output,
    core::{Transform, TransformBundle},
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
    load_level, load_sound_file, load_spritesheet, LevelStorage, SoundStorage, SpriteStorage,
};
use asteroid::{generate_asteroid, generate_asteroid_field, AsteroidBundle, AsteroidType};
use economy::{EconomyBundle, Enterprise};
use level::{generate_boundaries, initialize_level, Level, LevelBundle, LevelHandle};
use particles::ParticleBundle;
use physics::{PhysicsBundle, PhysicsHandle};
use player::{initialize_player, PlayerBundle};
use serde::Deserialize;

use crate::delivery::generate_delivery_zone;
mod assets;
mod asteroid;
mod delivery;
mod economy;
mod explosions;
mod level;
mod particles;
mod physics;
mod player;
mod tractor;

type ASSETS = (SpriteStorage, LevelStorage);

struct GameplayState {
    assets: ASSETS,
    level: LevelHandle,
}
impl SimpleState for GameplayState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.insert(self.assets.0.clone());
        data.world.insert(self.assets.1.clone());
        data.world.insert(Enterprise::begin_enterprise());
        initialize_level(data.world, &self.level);
        data.world.exec(|mut creator: UiCreator<'_>| {
            creator.create("hud.ron", ());
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
                                *camera = Camera::standard_2d(width as f32, height as f32);
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
        SimpleTrans::None
    }
}

struct MenuState {
    assets: ASSETS,
    menu: &'static str,
}
impl SimpleState for MenuState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.exec(|mut creator: UiCreator<'_>| {
            creator.create(self.menu, ());
        });
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        match &event {
            StateEvent::Window(event) => {
                if is_close_requested(&event) {
                    Trans::Quit
                } else {
                    Trans::None
                }
            }
            StateEvent::Ui(ui_event) => data.world.exec(|finder: UiFinder<'_>| {
                if ui_event.event_type == UiEventType::Click {
                    if let Some(start) = finder.find("play") {
                        if start == ui_event.target {
                            return Trans::Push(Box::new(GameplayState {
                                assets: self.assets.clone(),
                                level: self.assets.1.levels.get(0).unwrap().clone(),
                            }));
                        }
                    }
                    if let Some(exit) = finder.find("exit") {
                        if exit == ui_event.target {
                            return Trans::Quit;
                        }
                    }
                }
                Trans::None
            }),
            _ => Trans::None,
        }
    }
}

#[derive(Default)]
struct LoadingState {
    progress: Option<ProgressCounter>,
    assets: Option<ASSETS>,
    levels: Vec<String>,
}

impl LoadingState {
    fn with_levels(directory: Directory, path: &str) -> amethyst::Result<Self> {
        let val = directory.load(path)?;
        let mut de = ron::de::Deserializer::from_bytes(&val)?;
        let levels = Vec::<String>::deserialize(&mut de)?;
        de.end()?;

        Ok(LoadingState {
            progress: None,
            assets: None,
            levels,
        })
    }
}

impl SimpleState for LoadingState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.register::<PhysicsHandle>();
        // data.world.insert(AssetStorage::<TiledMap>::default());

        init_output(data.world);

        let mut progress_counter = ProgressCounter::new();
        let sprites = load_spritesheet(data.world, "Sprites", &mut progress_counter);
        let levels = self
            .levels
            .iter()
            .map(|path| load_level(data.world, path.to_string(), &mut progress_counter))
            .collect();
        // let main_theme = load_sound_file(data.world, "MainTheme.wav", &mut progress_counter);

        self.progress = Some(progress_counter);
        self.assets = Some((SpriteStorage { sprites }, LevelStorage { levels }));
    }

    fn update(&mut self, data: &mut StateData<GameData>) -> SimpleTrans {
        if let Some(progress) = &self.progress {
            println!("{:?}", progress);
            if progress.is_complete() {
                // return SimpleTrans::Switch(Box::new(MenuState {
                //     assets: self.assets.clone().unwrap(),
                //     menu: "main_menu.ron",
                // }));
                return SimpleTrans::Switch(Box::new(GameplayState {
                    assets: self.assets.clone().unwrap(),
                    level: self
                        .assets
                        .clone()
                        .unwrap()
                        .1
                        .levels
                        .get(0)
                        .unwrap()
                        .clone(),
                }));
            }
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
        .with_bundle(ParticleBundle)?
        .with_bundle(LevelBundle)?
        .with_bundle(PlayerBundle)?
        .with_bundle(FpsCounterBundle)?
        .with_bundle(UiBundle::<amethyst::input::StringBindings>::new())?;

    let mut game = Application::new(
        assets_dir,
        LoadingState::with_levels(Directory::new("assets"), "levels.ron")?,
        game_data,
    )?;
    game.run();

    Ok(())
}
