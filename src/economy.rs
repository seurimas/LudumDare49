use std::collections::HashMap;

use amethyst::{
    core::{HiddenPropagate, SystemBundle, Time, Transform},
    ecs::*,
    prelude::*,
    renderer::{sprite::SpriteSheetHandle, SpriteRender},
    ui::{UiFinder, UiImage, UiText, UiTransform},
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
    loans: i32,
    bankruptcies: usize,
    tried_jump: Option<f32>,
    last_refueling: (u64, bool),
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
            loans: 0,
            bankruptcies: 0,
            tried_jump: None,
            last_refueling: (0, false),
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

    pub fn burn_fuel(&mut self, burn: f64) {
        self.fuel -= burn;
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

    pub fn refueling_cost(&self) -> u64 {
        return f64::max(0.0, (100.0 - self.fuel) * 10.0) as u64;
    }

    pub fn refuel(&mut self) {
        let fuel_costs = self.refueling_cost();
        self.last_refueling = (fuel_costs, fuel_costs > self.funds);
        if self.last_refueling.1 {
            self.loans += 1;
            self.funds += 2000;
        }
        self.funds = self.funds - fuel_costs;
        self.fuel = 100.0;
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
        WriteStorage<'s, UiText>,
        WriteStorage<'s, HiddenPropagate>,
        SpriteRes<'s>,
        Read<'s, Level>,
        Write<'s, Enterprise>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut transforms,
            mut images,
            mut texts,
            mut hiddens,
            sprites,
            level,
            mut enterprise,
        ): Self::SystemData,
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
        if let Some(refueling) = find_by_id(&entities, &transforms, "refueling") {
            if let Some(refueling_text) = texts.get_mut(refueling) {
                refueling_text.text =
                    format!("Your refueling costs: {}", enterprise.last_refueling.0);
            }
        }
        if let Some(loan) = find_by_id(&entities, &transforms, "loan") {
            if enterprise.last_refueling.1 {
                hiddens.remove(loan);
            } else {
                hiddens.insert(loan, HiddenPropagate::new());
            }
        }
        if let Some(fuel_cost) = find_by_id(&entities, &transforms, "fuel_cost") {
            if let Some(fuel_cost) = texts.get_mut(fuel_cost) {
                fuel_cost.text = format!(
                    "Cost to jump: {} - Current fuel cost: {}",
                    level.jump_cost,
                    enterprise.refueling_cost()
                );
            }
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
