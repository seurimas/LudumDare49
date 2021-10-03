use amethyst::{
    assets::{AssetStorage, Directory, Format, Handle, Loader, ProgressCounter, RonFormat, Source},
    audio::{
        output::{init_output, Output},
        SourceHandle, WavFormat,
    },
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, ImageFormat, SpriteSheet, SpriteSheetFormat, Texture},
    shred::{Read, ResourceId, SystemData, World},
    Error,
};
use serde::Deserialize;

use crate::{
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
    loader.load(path, WavFormat, progress, &world.read_resource())
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
                sink.append(sound);
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
        // let main_theme = load_sound_file(data.world, "MainTheme.wav", &mut progress_counter);

        self.progress = Some(progress_counter);
        self.assets = Some((SpriteStorage { sprites }, LevelStorage { levels }));
    }

    fn update(&mut self, data: &mut StateData<GameData>) -> SimpleTrans {
        if let Some(progress) = &self.progress {
            println!("{:?}", progress);
            if progress.is_complete() {
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
                )));
            }
        }
        SimpleTrans::None
    }
}
