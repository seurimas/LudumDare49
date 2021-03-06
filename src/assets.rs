use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use amethyst::{
    assets::{AssetStorage, Directory, Format, Handle, Loader, ProgressCounter, RonFormat, Source},
    audio::{
        output::{init_output, Output},
        Mp3Format, SourceHandle, WavFormat,
    },
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, ImageFormat, SpriteSheet, SpriteSheetFormat, Texture},
    shred::{Read, ResourceId, System, SystemData, World},
    Error,
};
use serde::Deserialize;

use crate::{
    delivery::DeliveryZone,
    economy::Enterprise,
    level::{Level, LevelHandle},
    menu::{CardDesc, MenuState, MenuTransition},
    physics::PhysicsHandle,
    ASSETS,
};

pub fn load_sound_file<'a, N>(
    world: &mut World,
    path: N,
    progress: &'a mut ProgressCounter,
) -> SourceHandle
where
    N: Into<String>,
{
    let loader = world.read_resource::<Loader>();
    loader.load(path, Mp3Format, progress, &world.read_resource())
}

pub fn load_texture<'a, N>(
    world: &mut World,
    path: N,
    progress: &'a mut ProgressCounter,
) -> Handle<Texture>
where
    N: Into<String>,
{
    let loader = world.read_resource::<Loader>();
    let texture_storage = world.read_resource::<AssetStorage<Texture>>();
    loader.load(path, ImageFormat::default(), progress, &texture_storage)
}

pub fn load_spritesheet<'a, N>(
    world: &mut World,
    path: N,
    progress: &'a mut ProgressCounter,
) -> SpriteSheetHandle
where
    N: Into<String> + Copy,
{
    let texture_handle = load_texture(world, format!("{}.png", path.into()), progress);
    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        format!("{}.ron", path.into()), // Here we load the associated ron file
        SpriteSheetFormat(texture_handle),
        progress,
        &sprite_sheet_store,
    )
}

pub fn load_level<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> Handle<Level> {
    let loader = world.read_resource::<Loader>();
    let level_storage = world.read_resource::<AssetStorage<Level>>();
    loader.load(path, RonFormat, progress, &level_storage)
}

pub type SpriteRes<'s> = Option<Read<'s, SpriteStorage>>;

#[derive(Clone)]
pub struct SpriteStorage {
    pub sprites: SpriteSheetHandle,
}

pub trait SpriteHandles {
    fn get_handle(&self) -> SpriteSheetHandle;
}

impl<'s> SpriteHandles for Option<Read<'s, SpriteStorage>> {
    fn get_handle(&self) -> SpriteSheetHandle {
        self.as_ref().unwrap().sprites.clone()
    }
}
#[derive(Clone)]
pub struct LevelStorage {
    pub levels: Vec<LevelHandle>,
}

#[derive(Clone)]
pub struct SoundStorage {
    pub main_theme: SourceHandle,
    pub jump_theme: SourceHandle,
}

#[derive(SystemData)]
pub struct SoundPlayer<'a> {
    storage: Option<Read<'a, SoundStorage>>,
    output: Option<Read<'a, Output>>,
    sources: Read<'a, AssetStorage<amethyst::audio::Source>>,
}

impl<'a> SoundPlayer<'a> {
    pub fn play_main_theme(&self, sink: &amethyst::audio::AudioSink) {
        if let Some(ref sounds) = self.storage.as_ref() {
            if let Some(sound) = self.sources.get(&sounds.main_theme.clone()) {
                if sink.is_paused() {
                    sink.play();
                } else {
                    sink.append(sound);
                }
            }
        }
    }
    pub fn play_jumping_theme(&self, output: &Output, sink: &amethyst::audio::AudioSink) {
        if let Some(ref sounds) = self.storage.as_ref() {
            if let Some(sound) = self.sources.get(&sounds.jump_theme.clone()) {
                if !sink.is_paused() {
                    sink.pause();
                    output.play_once(sound, 0.5);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct LoadingState {
    progress: Option<ProgressCounter>,
    assets: Option<ASSETS>,
    levels: Vec<String>,
}

impl LoadingState {
    pub fn with_levels(directory: Directory, path: &str) -> amethyst::Result<Self> {
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
        let main_theme = load_sound_file(data.world, "audio/SpaceTheme.mp3", &mut progress_counter);
        let jump_theme =
            load_sound_file(data.world, "audio/JumpingTheme.mp3", &mut progress_counter);

        self.progress = Some(progress_counter);
        self.assets = Some((
            SpriteStorage { sprites },
            LevelStorage { levels },
            SoundStorage {
                main_theme,
                jump_theme,
            },
        ));
    }

    fn update(&mut self, data: &mut StateData<GameData>) -> SimpleTrans {
        if let Some(progress) = &self.progress {
            if progress.is_complete() {
                let enterprise: Option<Enterprise> = {
                    if let Ok(mut file) = File::open("enterprise.ron") {
                        let mut reader = BufReader::new(file);
                        let mut save = String::new();
                        reader.read_line(&mut save);
                        if let Ok(save) = ron::de::from_bytes(save.as_bytes()) {
                            Some(save)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                return SimpleTrans::Switch(Box::new(MenuState::card_menu(
                    self.assets.clone().unwrap(),
                    vec![
                        (
                            CardDesc::new("Begin Your Enterprise!", 0),
                            MenuTransition::Begin,
                        ),
                        (
                            CardDesc::new("Continue Your Enterprise!", 0),
                            MenuTransition::Continue,
                        ),
                        (
                            CardDesc::new("Retire For The Day...", 0),
                            MenuTransition::Quit,
                        ),
                    ],
                    enterprise,
                )));
            }
        }
        SimpleTrans::None
    }
}

pub struct DjSystem;

impl<'a> System<'a> for DjSystem {
    type SystemData = (
        Option<Read<'a, amethyst::audio::AudioSink>>,
        Read<'a, Output>,
        SoundPlayer<'a>,
        ReadStorage<'a, DeliveryZone>,
    );

    fn run(&mut self, (sink, output, player, deliveries): Self::SystemData) {
        if let Some(ref sink) = sink {
            if (&deliveries)
                .join()
                .find(|delivery| delivery.jump_started())
                .is_some()
            {
                player.play_jumping_theme(&output, sink);
            } else if (&deliveries)
                .join()
                .find(|delivery| delivery.jumping)
                .is_some()
            {
                // Silence.
            } else if sink.empty() || sink.is_paused() {
                player.play_main_theme(sink);
            }
        }
    }
}
