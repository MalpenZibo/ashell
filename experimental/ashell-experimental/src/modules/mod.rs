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

use crate::config::{Config, Gestures, Item, ItemId, Panel};
use configurator::Configurator;
use settings::{Settings, SettingsConfig};
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
    /// The states an item can be in, by name, for its `show`.
    pub states: fn(ItemId) -> &'static [&'static str],
    /// Those of its states it is worth showing in, for `show = "notable"`.
    pub notable: fn(ItemId) -> &'static [&'static str],
    pub panel_width: fn(Panel) -> f32,
}

impl Spec {
    /// A module whose items have no actions, gestures, options or states of
    /// their own, and that draws no panel.
    pub const fn bare(name: &'static str) -> Self {
        Self {
            name,
            actions: |_| &[],
            default_gestures: |_| Gestures::default(),
            check_options: |_, _| Vec::new(),
            states: |_| &[],
            notable: |_| &[],
            panel_width: |_| 0.0,
        }
    }
}

pub fn actions(id: ItemId) -> &'static [&'static str] {
    id.module()
        .map_or(&[], |module| (module.spec().actions)(id))
}

pub fn states(id: ItemId) -> &'static [&'static str] {
    id.module().map_or(&[], |module| (module.spec().states)(id))
}

pub fn notable(id: ItemId) -> &'static [&'static str] {
    id.module()
        .map_or(&[], |module| (module.spec().notable)(id))
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
    pub settings: SettingsConfig,
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
            ModuleId::Settings => {
                let (config, problems) = SettingsConfig::read(table);

                self.settings = config;
                problems
            }
            ModuleId::Config => unknown_keys(module, table),
        }
    }
}

/// The keys of a table that has none of its own yet, each named.
fn unknown_keys(module: ModuleId, table: &Table) -> Vec<String> {
    table
        .keys()
        .map(|key| format!("{}.{key}: unknown key, ignored", module.spec().name))
        .collect()
}

/// Closes the panel it is given to.
pub type Close = Rc<dyn Fn()>;

/// An item as its module draws it.
pub struct ItemView {
    pub view: Container,
    /// It has something to show, and its `show` lets it.
    pub shown: Memo<bool>,
    /// Runs one of its module's [`actions`].
    pub act: Rc<dyn Fn(&'static str)>,
}

/// How an item with a value draws it: its icon, the value, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Icon,
    Value,
    IconValue,
}

impl Style {
    /// The table's `style`, `default` while unset; a wrong one is named in
    /// `problems` and `default` taken.
    pub fn read(table: &Table, default: Self, problems: &mut Vec<String>) -> Self {
        match table.get("style").map(toml::Value::as_str) {
            None => default,
            Some(Some("icon")) => Style::Icon,
            Some(Some("value")) => Style::Value,
            Some(Some("icon-value")) => Style::IconValue,
            Some(_) => {
                problems.push(r#"style: expected "icon", "value" or "icon-value""#.to_string());
                default
            }
        }
    }

    pub fn icon(self) -> bool {
        self != Style::Value
    }

    pub fn value(self) -> bool {
        self != Style::Icon
    }
}

/// Names in `problems` each of an item's options not among the `known`.
pub fn unknown_options(id: ItemId, table: &Table, known: &[&str], problems: &mut Vec<String>) {
    for key in table.keys() {
        if !known.contains(&key.as_str()) {
            problems.push(format!("{key}: unknown option of {}, ignored", id.name()));
        }
    }
}

/// Whether `item` shows: only with something to show, and then as its
/// `show` says, `is` saying whether it is in one of its states.
pub fn shown(
    item: &Item,
    has: impl Fn() -> bool + 'static,
    is: impl Fn(&str) -> bool + 'static,
) -> Memo<bool> {
    let show = item.show.clone();
    let notable = notable(item.id);

    create_memo(move || has() && show.admits(notable, &is))
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
            settings: Settings::new(create_memo(move || {
                config.with(|c| c.modules.settings.clone())
            })),
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
            settings: settings::sample::settings(create_memo(move || {
                config.with(|c| c.modules.settings.clone())
            })),
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

    /// `None` for a panel no module draws yet. `close` closes it, for what
    /// is done once a button in it has run.
    pub fn panel(self, panel: Panel, close: Close) -> Option<Container> {
        match panel.module()? {
            ModuleId::Tempo => Some(self.tempo.panel(panel)),
            ModuleId::Settings => Some(self.settings.panel(panel, close)),
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
