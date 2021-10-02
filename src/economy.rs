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
};

pub struct Enterprise {
    fuel: f64,
    funds: u64,
    jump_cost: u64,
    bankruptcies: usize,
    tried_jump: Option<f32>,
    prices: HashMap<AsteroidType, f32>,
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
            funds: 100,
            jump_cost: 1_000,
            bankruptcies: 0,
            tried_jump: None,
            prices: AsteroidType::base_prices(),
        }
    }

    pub fn deliver(&mut self, asteroid: AsteroidType, mass: f32) {
        let ppm = self.prices.get(&asteroid).cloned().unwrap_or(1.0);
        self.funds += (mass * ppm) as u64;
    }

    pub fn try_jump(&mut self) -> bool {
        if self.funds < self.jump_cost {
            self.tried_jump = Some(3.5);
            false
        } else {
            self.funds -= self.jump_cost;
            true
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
        UiFinder<'s>,
        WriteStorage<'s, UiImage>,
        SpriteRes<'s>,
        Write<'s, Enterprise>,
    );

    fn run(&mut self, (finder, mut images, sprites, mut enterprise): Self::SystemData) {
        if let Some(symbol) = finder.find("insufficient_funds") {
            images.insert(
                symbol,
                MoneyHudSystem::insufficient_funds(&sprites, enterprise.tried_jump.is_some()),
            );
        }
        if let Some(symbol) = finder.find("sufficient_funds") {
            images.insert(
                symbol,
                MoneyHudSystem::sufficient_funds(
                    &sprites,
                    enterprise.funds >= enterprise.jump_cost,
                ),
            );
        }
        if let Some(symbol) = finder.find("money_symbol") {
            images.insert(symbol, MoneyHudSystem::symbol(&sprites));
        }
        for idx in 0..11 {
            if let Some(digit) = finder.find(format!("money_{}", idx).as_ref()) {
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
