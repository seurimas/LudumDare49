use std::collections::HashMap;

use amethyst::{
    core::{SystemBundle, Time, Transform},
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    ui::{UiFinder, UiImage, UiTransform},
    Error,
};

use crate::{
    assets::{SpriteHandles, SpriteRes},
    asteroid::AsteroidType,
    level::{Level, LevelHandle},
    menu::find_by_id,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Enterprise {
    fuel: f64,
    funds: u64,
    bankruptcies: usize,
    tried_jump: Option<f32>,
    last_completions: Vec<String>,
}

impl Default for Enterprise {
    fn default() -> Self {
        Self::begin_enterprise()
    }
}

impl Enterprise {
    pub fn begin_enterprise() -> Self {
        Enterprise {
            fuel: 100.0,
            funds: 0,
            bankruptcies: 0,
            tried_jump: None,
            last_completions: Vec::new(),
        }
    }

    pub fn deliver(&mut self, level: &Level, asteroid: AsteroidType, mass: f32) {
        let ppm = level.get_ppm(asteroid);
        self.funds += (mass * ppm) as u64;
    }

    pub fn eat_fuel(&mut self, rate: f64, time: &Time) {
        self.fuel -= rate * (time.delta_seconds() as f64);
    }

    pub fn try_jump(&mut self, level: &Level) -> bool {
        if !self.can_jump(level) {
            self.tried_jump = Some(3.5);
            false
        } else {
            self.funds -= level.jump_cost;
            self.last_completions.push(level.reference.name.clone());
            self.last_completions.truncate(6);
            true
        }
    }

    pub fn can_jump(&mut self, level: &Level) -> bool {
        self.funds >= level.jump_cost
    }

    pub fn get_next_levels(&self, levels: Vec<(Level, LevelHandle)>) -> Vec<(Level, LevelHandle)> {
        if self.last_completions.len() == 1 {
            levels
                .iter()
                .filter(|(level, handle)| level.reference.name != "Tutorial")
                .filter(|(level, handle)| level.reference.name == "Striking Out!")
                .cloned()
                .collect()
        } else {
            levels
                .iter()
                .filter(|(level, handle)| level.reference.name != "Tutorial")
                .filter(|(level, handle)| level.reference.name != "Striking Out!")
                .filter(|(level, handle)| !self.last_completions.contains(&level.reference.name))
                .cloned()
                .collect()
        }
    }
}

pub struct MoneyHudSystem;
impl MoneyHudSystem {
    fn symbol<'s>(sprites: &SpriteRes<'s>) -> UiImage {
        UiImage::Sprite(SpriteRender {
            sprite_sheet: sprites.get_handle(),
            sprite_number: 26,
        })
    }

    fn digit<'s>(sprites: &SpriteRes<'s>, idx: usize, value: u64) -> UiImage {
        let place = (10 as u64).pow(idx as u32);
        let mut digit = 16 + ((value / place) % 10);
        if value < place {
            digit = 15;
        }
        UiImage::Sprite(SpriteRender {
            sprite_sheet: sprites.get_handle(),
            sprite_number: digit as usize,
        })
    }

    fn insufficient_funds<'s>(sprites: &SpriteRes<'s>, insufficient: bool) -> UiImage {
        if insufficient {
            UiImage::Sprite(SpriteRender {
                sprite_sheet: sprites.get_handle(),
                sprite_number: 30,
            })
        } else {
            UiImage::SolidColor([0.0, 0.0, 0.0, 0.0])
        }
    }

    fn sufficient_funds<'s>(sprites: &SpriteRes<'s>, sufficient: bool) -> UiImage {
        if sufficient {
            UiImage::Sprite(SpriteRender {
                sprite_sheet: sprites.get_handle(),
                sprite_number: 31,
            })
        } else {
            UiImage::SolidColor([0.0, 0.0, 0.0, 0.0])
        }
    }
}
impl<'s> System<'s> for MoneyHudSystem {
    type SystemData = (
        Entities<'s>,
        WriteStorage<'s, UiTransform>,
        WriteStorage<'s, UiImage>,
        SpriteRes<'s>,
        Read<'s, Level>,
        Write<'s, Enterprise>,
    );

    fn run(
        &mut self,
        (entities, mut transforms, mut images, sprites, level, mut enterprise): Self::SystemData,
    ) {
        if let Some(symbol) = find_by_id(&entities, &transforms, "insufficient_funds") {
            images.insert(
                symbol,
                MoneyHudSystem::insufficient_funds(&sprites, enterprise.tried_jump.is_some()),
            );
        }
        if let Some(symbol) = find_by_id(&entities, &transforms, "sufficient_funds") {
            images.insert(
                symbol,
                MoneyHudSystem::sufficient_funds(&sprites, enterprise.can_jump(&level)),
            );
        }
        if let Some(symbol) = find_by_id(&entities, &transforms, "money_symbol") {
            images.insert(symbol, MoneyHudSystem::symbol(&sprites));
        }
        if let Some(fuel_level) = find_by_id(&entities, &transforms, "fuel_value") {
            if let Some(fuel_level) = transforms.get_mut(fuel_level) {
                fuel_level.width = (180.0 * enterprise.fuel / 100.0) as f32;
            }
        }
        for idx in 0..11 {
            if let Some(digit) =
                find_by_id(&entities, &transforms, format!("money_{}", idx).as_ref())
            {
                images.insert(
                    digit,
                    MoneyHudSystem::digit(&sprites, idx, enterprise.funds),
                );
            }
        }
    }
}

pub struct InsufficientFundsWarningSystem;
impl<'s> System<'s> for InsufficientFundsWarningSystem {
    type SystemData = (Write<'s, Enterprise>, Read<'s, Time>);

    fn run(&mut self, (mut enterprise, time): Self::SystemData) {
        enterprise.tried_jump = enterprise.tried_jump.and_then(|warning_lifetime| {
            if warning_lifetime > time.delta_seconds() {
                Some(warning_lifetime - time.delta_seconds())
            } else {
                None
            }
        })
    }
}

pub struct EconomyBundle;
impl<'a, 'b> SystemBundle<'a, 'b> for EconomyBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(MoneyHudSystem, "money_hud", &[]);
        dispatcher.add(
            InsufficientFundsWarningSystem,
            "insufficient_funds_cooldown",
            &[],
        );
        Ok(())
    }
}
