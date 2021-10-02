use amethyst::{
    assets::{AssetStorage, Format, Handle, Loader, ProgressCounter, RonFormat},
    audio::{output::Output, Source, SourceHandle, WavFormat},
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, ImageFormat, SpriteSheet, SpriteSheetFormat, Texture},
    shred::{Read, ResourceId, SystemData, World},
    Error,
};

use crate::level::{Level, LevelHandle};

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
    sources: Read<'a, AssetStorage<Source>>,
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
