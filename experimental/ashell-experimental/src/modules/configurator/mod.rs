//! The config window: opened on the first start, as a welcome, and by the
//! `config` item, it chooses the bar to start from and edits the look, each
//! change saved and shown on the bars as it is made.
//!
//! It is a surface of its own, not a panel: the bars are made again on each
//! change of the look, and a panel would close with them.

mod files;
mod window;

use std::cell::RefCell;
use std::rc::Rc;

use guido::prelude::*;
use styletto::foundations::tokens::icon as icon_box;
use styletto::{Icon, icon, look};
use toml::Table;

use super::{ItemView, Spec, shown};
use crate::config::{Config, Item};

/// How the window opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// A step at a time, on the first start.
    Steps,
    /// Every setting at once.
    Settings,
}

#[derive(Clone, Copy)]
pub struct Configurator {
    shown: RwSignal<Option<Shown>>,
}

impl Configurator {
    pub fn new(config: Signal<Config>) -> Self {
        let shown = create_signal(None);
        let window: Rc<RefCell<Option<SurfaceHandle>>> = Rc::default();

        create_effect(move || {
            let mut window = window.borrow_mut();

            match shown.get() {
                Some(how) => {
                    log::info!("config window: opened as {how:?}");
                    *window = Some(window::spawn(config, how, move || shown.set(None)));
                }
                None => {
                    if let Some(open) = window.take() {
                        open.close();
                    }
                }
            }
        });

        Self { shown }
    }

    /// Opens the window `how`, unless it is open already.
    pub fn open(self, how: Shown) {
        if self.shown.get_untracked().is_none() {
            self.shown.set(Some(how));
        }
    }

    /// The button that opens the window, as `item`'s `show` lets it. A click
    /// is the zone's, or the item's `open`, as for any item: `"config"`
    /// opens the window.
    pub fn item(self, item: &Item) -> ItemView {
        let t = look();

        ItemView {
            view: container().child(icon(
                Icon::Settings,
                icon_box::SMALL.px(),
                t.color(|c| c.sub),
            )),
            shown: shown(item.show, || true, || true),
            act: Rc::new(|_| {}),
        }
    }
}

pub const SPEC: Spec = Spec::bare("config");

/// Its table has no keys yet.
pub fn check(table: &Table) -> Vec<String> {
    table
        .keys()
        .map(|key| format!("config.{key}: unknown key, ignored"))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::config::catalogue::builtin_zone;
    use crate::config::{ItemId, Open};

    #[test]
    fn the_config_zone_holds_its_button_and_a_click_on_either_opens_the_window() {
        let zone = builtin_zone("config").unwrap();

        assert_eq!(zone.items, [ItemId::Config]);
        assert_eq!(ItemId::parse("config"), Some(ItemId::Config));
        assert_eq!(zone.open, Open::Config);
        assert_eq!(ItemId::Config.default_open(), Some(Open::Config));
        assert_eq!(Open::parse("config"), Some(Open::Config));
    }
}
