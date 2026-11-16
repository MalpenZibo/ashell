//! What the zones show: each module offers items, panels and the actions
//! its items' gestures run, and draws them from its own data.

pub mod tempo;

use std::rc::Rc;

use guido::prelude::*;
use toml::Table;

use crate::config::{Config, Item, ItemId, Panel};
use tempo::Tempo;

/// An item as its module draws it.
pub struct ItemView {
    pub view: Container,
    /// It has something to show, and its `show` lets it.
    pub shown: Memo<bool>,
    /// Runs one of its [`actions`](ItemId::actions).
    pub act: Rc<dyn Fn(&'static str)>,
}

/// Every module, made once for the whole shell: the bars made again on a
/// change of the configuration share them.
#[derive(Clone, Copy)]
pub struct Modules {
    tempo: Tempo,
}

impl Modules {
    pub fn new(config: Signal<Config>) -> Self {
        Self {
            tempo: Tempo::new(create_memo(move || config.with(|c| c.tempo.clone()))),
        }
    }

    /// `None` for an item no module draws yet.
    pub fn item(self, item: &Item) -> Option<ItemView> {
        match item.id {
            ItemId::Clock | ItemId::Date | ItemId::Weather | ItemId::Forecast => {
                Some(self.tempo.item(item))
            }
            _ => None,
        }
    }

    /// `None` for a panel no module draws yet.
    pub fn panel(self, panel: Panel) -> Option<Container> {
        match panel {
            Panel::Calendar => Some(self.tempo.calendar()),
            _ => None,
        }
    }

    pub fn panel_width(panel: Panel) -> Option<f32> {
        match panel {
            Panel::Calendar => Some(tempo::CALENDAR_WIDTH),
            _ => None,
        }
    }
}

/// What is wrong with an item's own options, as its module reads them.
pub fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    match id {
        ItemId::Clock | ItemId::Date | ItemId::Weather | ItemId::Forecast => {
            tempo::check_options(id, options)
        }
        _ => Vec::new(),
    }
}
