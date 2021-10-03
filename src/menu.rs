use amethyst::{
    core::Parent,
    ecs::WriteStorage,
    input::is_close_requested,
    prelude::*,
    ui::{UiCreator, UiEventType, UiFinder, UiImage, UiText},
    GameData, SimpleState, SimpleTrans, StateData, StateEvent, Trans,
};

use crate::{economy::Enterprise, GameplayState, ASSETS};

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
    Level,
    Quit,
}

pub struct MenuState {
    assets: ASSETS,
    menu: &'static str,
    cards: Vec<(CardDesc, MenuTransition)>,
    initialized: bool,
}

impl MenuState {
    pub fn card_menu(assets: ASSETS, cards: Vec<(CardDesc, MenuTransition)>) -> MenuState {
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
            cards,
            initialized: false,
        }
    }
}

impl SimpleState for MenuState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
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
                    |(mut ui_text, mut ui_image, finder): (
                        WriteStorage<UiText>,
                        WriteStorage<UiImage>,
                        UiFinder,
                    )| {
                        for i in 0.. {
                            if let (Some(label), Some(card)) = (
                                finder.find(format!("card_label_{}", i).as_ref()),
                                self.cards.get(i),
                            ) {
                                if let Some(label) = ui_text.get_mut(label) {
                                    label.text = card.0.title.clone();
                                }
                            } else {
                                break;
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
            StateEvent::Ui(ui_event) => data.world.exec(|finder: UiFinder<'_>| {
                if ui_event.event_type == UiEventType::Click {
                    for i in 0.. {
                        if let (Some(card_entity), Some(card)) = (
                            finder.find(format!("card_container_{}", i).as_ref()),
                            self.cards.get(i),
                        ) {
                            if card_entity == ui_event.target {
                                match card.1 {
                                    MenuTransition::Begin => {
                                        return Trans::Push(Box::new(GameplayState {
                                            assets: self.assets.clone(),
                                            enterprise: Enterprise::begin_enterprise(),
                                            level: self.assets.1.levels.get(0).unwrap().clone(),
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
            }),
            _ => Trans::None,
        }
    }
}
