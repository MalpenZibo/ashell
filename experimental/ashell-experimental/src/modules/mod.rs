//! What the zones show: each module offers items, panels and the actions
//! its items' gestures run, and draws them from its own data.
//!
//! The catalogue names which module draws each item and panel. A module adds
//! its [`Spec`], for what reading the configuration needs before any module
//! exists, its table to [`Configs`], and its state to [`Modules`].

pub mod configurator;
pub mod settings;
pub mod tempo;

use std::rc::Rc;

use guido::prelude::*;
use toml::Table;

use crate::config::{Config, Gestures, Item, ItemId, Panel, Show};
use configurator::Configurator;
use settings::Settings;
use tempo::{Tempo, TempoConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleId {
    Tempo,
    Settings,
    Config,
}

impl ModuleId {
    pub const ALL: [ModuleId; 3] = [ModuleId::Tempo, ModuleId::Settings, ModuleId::Config];

    pub fn spec(self) -> &'static Spec {
        match self {
            ModuleId::Tempo => &tempo::SPEC,
            ModuleId::Settings => &settings::SPEC,
            ModuleId::Config => &configurator::SPEC,
        }
    }
}

/// What a module tells before it exists, for reading the configuration.
pub struct Spec {
    /// Its table in the configuration.
    pub name: &'static str,
    /// What its items' gestures can be bound to, by name.
    pub actions: fn(ItemId) -> &'static [&'static str],
    pub default_gestures: fn(ItemId) -> Gestures,
    /// What is wrong with an item's own options.
    pub check_options: fn(ItemId, &Table) -> Vec<String>,
    pub panel_width: fn(Panel) -> f32,
}

impl Spec {
    /// A module whose items have no actions, gestures or options of their
    /// own, and that draws no panel.
    pub const fn bare(name: &'static str) -> Self {
        Self {
            name,
            actions: |_| &[],
            default_gestures: |_| Gestures::default(),
            check_options: |_, _| Vec::new(),
            panel_width: |_| 0.0,
        }
    }
}

pub fn actions(id: ItemId) -> &'static [&'static str] {
    id.module()
        .map_or(&[], |module| (module.spec().actions)(id))
}

pub fn default_gestures(id: ItemId) -> Gestures {
    id.module()
        .map(|module| (module.spec().default_gestures)(id))
        .unwrap_or_default()
}

pub fn check_options(id: ItemId, options: &Table) -> Vec<String> {
    id.module()
        .map(|module| (module.spec().check_options)(id, options))
        .unwrap_or_default()
}

/// `None` for a panel no module draws yet.
pub fn panel_width(panel: Panel) -> Option<f32> {
    panel
        .module()
        .map(|module| (module.spec().panel_width)(panel))
}

/// Each module's table of the configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Configs {
    pub tempo: TempoConfig,
}

impl Configs {
    /// Reads `module`'s table, and returns what is wrong with it.
    pub fn read(&mut self, module: ModuleId, table: &Table) -> Vec<String> {
        match module {
            ModuleId::Tempo => {
                let (config, problems) = TempoConfig::read(table);

                self.tempo = config;
                problems
            }
            ModuleId::Settings => table
                .keys()
                .map(|key| format!("settings.{key}: unknown key, ignored"))
                .collect(),
            ModuleId::Config => configurator::check(table),
        }
    }
}

/// Whether an item shows: it `has` something to, and its `show` lets it,
/// `Notable` only while it is `notable`.
pub fn shown(
    show: Show,
    has: impl Fn() -> bool + 'static,
    notable: impl Fn() -> bool + 'static,
) -> Memo<bool> {
    create_memo(move || {
        has()
            && match show {
                Show::Always => true,
                Show::Notable => notable(),
                Show::Never => false,
            }
    })
}

/// An item as its module draws it.
pub struct ItemView {
    pub view: Container,
    /// It has something to show, and its `show` lets it.
    pub shown: Memo<bool>,
    /// Runs one of its module's [`actions`].
    pub act: Rc<dyn Fn(&'static str)>,
}

/// Every module, made once for the whole shell: the bars made again on a
/// change of the configuration share them.
#[derive(Clone, Copy)]
pub struct Modules {
    tempo: Tempo,
    settings: Settings,
    configurator: Configurator,
}

impl Modules {
    pub fn new(config: Signal<Config>) -> Self {
        Self {
            tempo: Tempo::new(create_memo(move || {
                config.with(|c| c.modules.tempo.clone())
            })),
            settings: Settings::new(),
            configurator: Configurator::new(config),
        }
    }

    /// Tempo at `now`, its weather a sample week's, for pictures.
    #[cfg(test)]
    pub fn fixed(config: Signal<Config>, now: chrono::NaiveDateTime) -> Self {
        Self {
            tempo: Tempo::fixed(
                create_memo(move || config.with(|c| c.modules.tempo.clone())),
                now,
                Some(tempo::sample::weather(now)),
            ),
            settings: settings::sample::settings(),
            configurator: Configurator::new(config),
        }
    }

    /// Opens the config window `how`, unless it is open already.
    pub fn configure(self, how: configurator::Shown) {
        self.configurator.open(how);
    }

    /// `None` for an item no module draws yet.
    pub fn item(self, item: &Item) -> Option<ItemView> {
        match item.id.module()? {
            ModuleId::Tempo => Some(self.tempo.item(item)),
            ModuleId::Settings => Some(self.settings.item(item)),
            ModuleId::Config => Some(self.configurator.item(item)),
        }
    }

    /// `None` for a panel no module draws yet.
    pub fn panel(self, panel: Panel) -> Option<Container> {
        match panel.module()? {
            ModuleId::Tempo => Some(self.tempo.panel(panel)),
            ModuleId::Settings => None,
            ModuleId::Config => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_have_their_own_tables() {
        let names: Vec<_> = ModuleId::ALL.iter().map(|m| m.spec().name).collect();

        for name in &names {
            assert_eq!(names.iter().filter(|n| *n == name).count(), 1);
            assert!(!["appearance", "bar", "monitor", "zone"].contains(name));
        }
    }
}
