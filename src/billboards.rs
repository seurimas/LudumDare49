use std::collections::HashMap;

use amethyst::{
    core::{SystemBundle, Time, Transform},
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    ui::{UiFinder, UiImage},
    Error,
};

use crate::{
    assets::{SpriteHandles, SpriteRes},
    asteroid::AsteroidType,
    level::Level,
    player::Player,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct BillboardDesc {
    pub location: (f32, f32),
    pub sprite_number: usize,
}

#[derive(Serialize, Deserialize, Component, Clone)]
#[storage(VecStorage)]
pub struct Billboard {
    sprite_number: usize,
}

pub fn generate_billboard(builder: impl Builder, sprites: SpriteSheetHandle, desc: &BillboardDesc) {
    let mut transform = Transform::default();
    transform.set_translation_xyz(desc.location.0, desc.location.1, -1.0);
    builder
        .with(SpriteRender::new(sprites, desc.sprite_number))
        .with(transform)
        .with(Billboard {
            sprite_number: desc.sprite_number,
        })
        .build();
}

pub struct BillboardSystem;
impl<'s> System<'s> for BillboardSystem {
    type SystemData = (
        ReadStorage<'s, Player>,
        ReadStorage<'s, Billboard>,
        WriteStorage<'s, SpriteRender>,
        WriteStorage<'s, Transform>,
    );

    fn run(&mut self, (players, billboards, mut sprite_renders, mut transforms): Self::SystemData) {
        let mut rotation = 0.0;
        if let Some((_, player_transform)) = (&players, &transforms).join().next() {
            rotation = player_transform.rotation().euler_angles().2;
        }
        for (billboard, sprite_render, transform) in
            (&billboards, &mut sprite_renders, &mut transforms).join()
        {
            sprite_render.sprite_number = billboard.sprite_number;
            transform.set_rotation_2d(rotation);
        }
    }
}

pub struct BillboardBundle;
impl<'a, 'b> SystemBundle<'a, 'b> for BillboardBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(BillboardSystem, "billboards", &[]);
        Ok(())
    }
}
