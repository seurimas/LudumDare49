use std::{
    fs::File,
    io::{BufWriter, Write},
};

use amethyst::{
    assets::AssetStorage,
    core::{HiddenPropagate, Parent},
    ecs::*,
    input::is_close_requested,
    prelude::*,
    ui::{UiCreator, UiEventType, UiFinder, UiImage, UiText, UiTransform},
    GameData, SimpleState, SimpleTrans, StateData, StateEvent, Trans,
};

use crate::{
    economy::Enterprise,
    level::{Level, LevelHandle},
    GameplayState, ASSETS,
};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct CardDesc {
    title: String,
    sprite_number: usize,
}

impl CardDesc {
    pub fn new<T>(title: T, sprite_number: usize) -> Self
    where
        T: Into<String>,
    {
        CardDesc {
            title: title.into(),
            sprite_number: sprite_number,
        }
    }
}

pub enum MenuTransition {
    Begin,
    Continue,
    Level(Level, LevelHandle),
    Quit,
}

pub struct MenuState {
    assets: ASSETS,
    menu: &'static str,
    enterprise: Option<Enterprise>,
    cards: Vec<(CardDesc, MenuTransition)>,
    initialized: bool,
}

pub fn find_by_id<'s>(
    entities: &Entities<'s>,
    transforms: &WriteStorage<'s, UiTransform>,
    id: &str,
) -> Option<Entity> {
    (entities, transforms)
        .join()
        .find(|(_, transform)| transform.id == id)
        .map(|(entity, _)| entity)
}

impl MenuState {
    pub fn end_level(assets: ASSETS, enterprise: Option<Enterprise>) -> MenuState {
        MenuState {
            assets,
            menu: "ui/end_level_menu.ron",
            enterprise,
            cards: vec![
                (
                    CardDesc::new("Continue Your Enterprise!", 0),
                    MenuTransition::Continue,
                ),
                (
                    CardDesc::new("Retire For The Day...", 0),
                    MenuTransition::Quit,
                ),
            ],
            initialized: false,
        }
    }
    pub fn card_menu(
        assets: ASSETS,
        cards: Vec<(CardDesc, MenuTransition)>,
        enterprise: Option<Enterprise>,
    ) -> MenuState {
        let menu = match cards.len() {
            3 => "ui/three_menu.ron",
            _ => panic!(
                "Failed to find a valid cards menu for {} cards",
                cards.len()
            ),
        };
        MenuState {
            assets,
            menu,
            enterprise,
            cards,
            initialized: false,
        }
    }
    pub fn level_menu(
        assets: ASSETS,
        mut levels: Vec<(Level, LevelHandle)>,
        enterprise: Option<Enterprise>,
    ) -> MenuState {
        let mut cards = vec![];
        if let Some(enterprise) = &enterprise {
            levels = enterprise.get_next_levels(levels);
        }
        for (level, handle) in levels.iter() {
            cards.push((
                level.card.clone(),
                MenuTransition::Level(level.clone(), handle.clone()),
            ));
        }
        cards.push((
            CardDesc::new("Retire For The Day...", 0),
            MenuTransition::Quit,
        ));
        MenuState {
            assets,
            menu: "ui/six_menu.ron",
            enterprise,
            cards,
            initialized: false,
        }
    }
}

impl SimpleState for MenuState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        if let Some(enterprise) = &self.enterprise {
            data.world.insert(enterprise.clone());
            if let Ok(mut file) = File::create("enterprise.ron") {
                if let Ok(save) = ron::ser::to_string(enterprise) {
                    println!("Saving");
                    file.write(save.as_bytes());
                }
            } else {
                println!("Could not save...");
            }
        }
        data.world.exec(|mut creator: UiCreator<'_>| {
            println!("Creating menu...");
            let main_menu = creator.create(self.menu, ());
        });
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if !self.initialized {
            let menu = data.world.exec(|finder: UiFinder<'_>| finder.find("menu"));
            if let Some(menu) = menu {
                data.world.exec(
                    |(mut transforms, mut images, mut texts, mut hiddens, entities): (
                        WriteStorage<UiTransform>,
                        WriteStorage<UiImage>,
                        WriteStorage<UiText>,
                        WriteStorage<HiddenPropagate>,
                        Entities,
                    )| {
                        for i in 0..6 {
                            if let Some(card) = self.cards.get(i) {
                                if let (Some(title_ref)) =
                                    find_by_id(&entities, &transforms, &format!("card_label_{}", i))
                                {
                                    if let Some(text) = texts.get_mut(title_ref) {
                                        text.text = card.0.title.clone();
                                    }
                                }
                            } else if let Some(entity) =
                                find_by_id(&entities, &transforms, &format!("card_container_{}", i))
                            {
                                hiddens.insert(entity, HiddenPropagate::new());
                            }
                        }
                    },
                );
                self.initialized = true;
            }
        }
        Trans::None
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
            StateEvent::Ui(ui_event) => data.world.exec(
                |(finder, level_storage): (UiFinder, Read<AssetStorage<Level>>)| {
                    if ui_event.event_type == UiEventType::Click {
                        for i in 0.. {
                            if let (Some(card_entity), Some(card)) = (
                                finder.find(format!("card_container_{}", i).as_ref()),
                                self.cards.get(i),
                            ) {
                                if card_entity == ui_event.target {
                                    match &card.1 {
                                        MenuTransition::Begin => {
                                            return Trans::Push(Box::new(GameplayState {
                                                assets: self.assets.clone(),
                                                enterprise: Enterprise::begin_enterprise(),
                                                level: self.assets.1.levels.get(0).unwrap().clone(),
                                            }));
                                        }
                                        MenuTransition::Continue => {
                                            let mut levels = Vec::new();
                                            for handle in self.assets.1.levels.iter() {
                                                if let Some(level) = level_storage.get(&handle) {
                                                    levels.push((level.clone(), handle.clone()));
                                                }
                                            }
                                            levels = self
                                                .enterprise
                                                .as_ref()
                                                .unwrap_or(&Enterprise::begin_enterprise())
                                                .get_next_levels(levels);
                                            return Trans::Push(Box::new(MenuState::level_menu(
                                                self.assets.clone(),
                                                levels,
                                                self.enterprise.clone(),
                                            )));
                                        }
                                        MenuTransition::Level(level, handle) => {
                                            return Trans::Push(Box::new(GameplayState {
                                                assets: self.assets.clone(),
                                                enterprise: self
                                                    .enterprise
                                                    .clone()
                                                    .unwrap_or(Enterprise::begin_enterprise()),
                                                level: handle.clone(),
                                            }));
                                        }
                                        MenuTransition::Quit => {
                                            return Trans::Quit;
                                        }
                                        _ => {
                                            return Trans::None;
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    Trans::None
                },
            ),
            _ => Trans::None,
        }
    }
}
