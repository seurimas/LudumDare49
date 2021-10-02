use amethyst::{
    assets::{AssetStorage, ProgressCounter},
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
use assets::{load_sound_file, load_spritesheet, SoundStorage, SpriteStorage};
use asteroid::{generate_asteroid, generate_asteroid_field, AsteroidBundle, AsteroidType};
use level::{generate_boundaries, LevelBundle};
use physics::{PhysicsBundle, PhysicsHandle};
use player::{initialize_player, PlayerBundle};

use crate::delivery::generate_delivery_zone;
mod assets;
mod asteroid;
mod delivery;
mod explosions;
mod level;
mod physics;
mod player;
mod tractor;

type ASSETS = (SpriteStorage,);

#[derive(Default)]
struct LoadingState {
    progress: Option<ProgressCounter>,
    assets: Option<ASSETS>,
}

struct GameplayState {
    assets: (SpriteStorage,),
}
impl SimpleState for GameplayState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.insert(self.assets.0.clone());
        let mut transform = Transform::default();
        transform.set_translation_x(50.0);
        transform.set_translation_y(50.0);
        initialize_player(data.world, transform);
        let mut transform = Transform::default();
        transform.set_translation_x(25.0);
        transform.set_translation_y(25.0);
        transform.set_translation_x(0.0);
        transform.set_translation_y(0.0);
        generate_delivery_zone(data.world, (75.0, 75.0), transform.clone());
        transform.set_translation_x(0.0);
        transform.set_translation_y(0.0);
        generate_asteroid_field(data.world, (1000.0, 1000.0), 300, 60, transform);
        generate_boundaries(data.world, (1200.0, 1200.0));
        // initialize_tile_world(data.world);
        // data.world.exec(|mut creator: UiCreator<'_>| {
        //     creator.create(get_resource("hud.ron"), ());
        // });
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
                        println!("Resized!");
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

impl SimpleState for LoadingState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.register::<PhysicsHandle>();
        // data.world.insert(AssetStorage::<TiledMap>::default());

        init_output(data.world);

        let mut progress_counter = ProgressCounter::new();
        let sprites = load_spritesheet(data.world, "Sprites", &mut progress_counter);
        // let main_theme = load_sound_file(data.world, "MainTheme.wav", &mut progress_counter);

        self.progress = Some(progress_counter);
        self.assets = Some((SpriteStorage { sprites },));
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
        .with_bundle(LevelBundle)?
        .with_bundle(PlayerBundle)?
        .with_bundle(FpsCounterBundle)?
        .with_bundle(UiBundle::<amethyst::input::StringBindings>::new())?;

    let mut game = Application::new(assets_dir, LoadingState::default(), game_data)?;
    game.run();

    Ok(())
}
