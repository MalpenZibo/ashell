use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use styletto::foundations::tokens::{Space, size};
use styletto::theme::config::UiConfig;
use toml::{Table, Value};

use super::catalogue::{Action, Gestures, ItemId, Open, builtin_zone};
use crate::modules::{self, ModuleId};

use super::{
    Background, Bar, Config, Holds, Host, Item, Monitor, Node, Padding, Panels, Parsed, Place,
    Position, Spec, Zone,
};

/// The zones of the one group in each place, when a bar places nothing.
const DEFAULT_GROUPS: [&[&str]; 3] = [
    &["workspaces"],
    &["window"],
    &["clock", "privacy", "status"],
];

pub fn read(file: &Table) -> Parsed {
    let mut reader = Reader::default();
    let keys: Vec<&str> = ["appearance", "bar", "monitor", "zone"]
        .into_iter()
        .chain(ModuleId::ALL.iter().map(|module| module.spec().name))
        .collect();
    reader.known_keys("", file, &keys);

    let appearance = file
        .get("appearance")
        .map(|value| reader.appearance(value))
        .unwrap_or_default();

    let mut modules = modules::Configs::default();

    for module in ModuleId::ALL {
        let name = module.spec().name;

        if let Some(table) = file.get(name).and_then(|value| reader.table(name, value)) {
            let problems = modules.read(module, table);

            reader.problems.extend(problems);
        }
    }

    if let Some(zones) = file.get("zone") {
        reader.zone_tables(zones);
    }

    let bars = reader.bars(file.get("bar"));
    let monitors = reader.monitors(file.get("monitor"), &bars);

    for name in reader.zones.keys() {
        if !reader.placed.contains(name) {
            reader.problems.push(format!(
                "zone.{name}: not placed in any group, so not shown"
            ));
        }
    }

    Parsed {
        config: Config {
            appearance,
            modules,
            bars: bars.into_iter().map(|(_, bar)| bar).collect(),
            monitors,
        },
        problems: reader.problems,
    }
}

const BAR_KEYS: &[&str] = &[
    "position",
    "background",
    "shape",
    "padding",
    "gap",
    "hover",
    "panels",
    Place::Start.name(),
    Place::Center.name(),
    Place::End.name(),
];

const NODE_KEYS: &[&str] = &[
    "zones",
    "nodes",
    "shape",
    "hover",
    "panels",
    Place::Start.name(),
    Place::Center.name(),
    Place::End.name(),
];

fn one_per_edge(
    listed: impl IntoIterator<Item = usize>,
    bars: &[(String, Bar)],
) -> (Vec<usize>, Vec<(usize, usize)>) {
    let mut kept: Vec<usize> = Vec::new();
    let mut dropped = Vec::new();

    for index in listed {
        let position = bars[index].1.position;
        match kept.iter().find(|&&i| bars[i].1.position == position) {
            Some(&taken) => dropped.push((index, taken)),
            None => kept.push(index),
        }
    }

    (kept, dropped)
}

/// What a node passes down to the nodes inside it.
#[derive(Clone, Copy)]
struct Inherited {
    hover: bool,
    panels: Panels,
    /// Whether a group inside has a shape when it doesn't say: false once a
    /// node round it has one.
    shape: bool,
    host: Host,
}

#[derive(Default)]
struct ZoneTable {
    items: Option<Vec<Item>>,
    details: Option<Vec<Item>>,
    open: Option<Open>,
}

#[derive(Default)]
struct Reader {
    problems: Vec<String>,
    zones: BTreeMap<String, ZoneTable>,
    placed: BTreeSet<String>,
    next_id: usize,
}

impl Reader {
    fn appearance(&mut self, value: &Value) -> UiConfig {
        value
            .clone()
            .try_into()
            .unwrap_or_else(|err: toml::de::Error| {
                self.warn("appearance", err.message());
                UiConfig::default()
            })
    }

    fn bars(&mut self, value: Option<&Value>) -> Vec<(String, Bar)> {
        let mut bars = Vec::new();

        if let Some(table) = value.and_then(|value| self.table("bar", value)) {
            for (name, value) in table {
                let at = format!("bar.{name}");
                if let Some(table) = self.table(&at, value) {
                    bars.push((name.clone(), self.bar(&at, table)));
                }
            }
        }

        if bars.is_empty() {
            bars.push(("main".to_string(), self.bar("bar.main", &Table::new())));
        }

        bars
    }

    fn bar(&mut self, at: &str, table: &Table) -> Bar {
        self.known_keys(at, table, BAR_KEYS);

        let position: Position = self.get(at, table, "position").unwrap_or_default();
        let background: Background = self.get(at, table, "background").unwrap_or_default();

        let padding = table
            .get("padding")
            .and_then(|value| self.padding(&format!("{at}.padding"), value))
            .unwrap_or(Padding::all(size::S2));
        let gap = table
            .get("gap")
            .and_then(|value| self.space(&format!("{at}.gap"), value))
            .unwrap_or(size::S2);

        self.next_id = 0;
        let inherited = Inherited {
            hover: false,
            panels: Panels::default(),
            shape: true,
            host: match background {
                Background::Solid => Host::Background,
                Background::Transparent => Host::None,
            },
        };
        let id = self.id();
        let (shape, inherited) = self.inherit(at, id, table, inherited, false);
        let places = if writes_places(table) {
            self.places(at, table, inherited)
        } else {
            DEFAULT_GROUPS.map(|zones| self.default_group(at, zones, inherited))
        };

        Bar {
            position,
            background,
            padding,
            gap,
            root: Self::holding(id, Holds::Placed(places), shape, inherited),
        }
    }

    fn monitors(&mut self, value: Option<&Value>, bars: &[(String, Bar)]) -> Vec<Monitor> {
        let mut monitors = Vec::new();
        let mut any = None;

        if let Some(table) = value.and_then(|value| self.table("monitor", value)) {
            for (spec, value) in table {
                let at = format!("monitor.{spec}");
                let Some(table) = self.table(&at, value) else {
                    continue;
                };
                self.known_keys(&at, table, &["bars"]);

                let at = format!("{at}.bars");
                let Some(listed) = table.get("bars") else {
                    self.warn(&at, "missing");
                    continue;
                };
                let listed = self.bar_names(&at, listed, bars);

                if spec == "*" {
                    any = Some(listed);
                } else {
                    monitors.push(Monitor {
                        on: Spec::Named(spec.clone()),
                        bars: listed,
                    });
                }
            }
        }

        let any = any.unwrap_or_else(|| one_per_edge(0..bars.len(), bars).0);
        monitors.push(Monitor {
            on: Spec::Any,
            bars: any,
        });

        monitors
    }

    fn bar_names(&mut self, at: &str, value: &Value, bars: &[(String, Bar)]) -> Vec<usize> {
        let listed: Vec<usize> = self
            .array(at, value)
            .unwrap_or_default()
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let at = format!("{at}[{index}]");
                let name = self.string(&at, name)?;
                let found = bars.iter().position(|(known, _)| known == name);
                if found.is_none() {
                    self.warn(&at, format!("no bar named \"{name}\""));
                }
                found
            })
            .collect();

        let (kept, dropped) = one_per_edge(listed, bars);
        for (index, taken) in dropped {
            self.warn(
                at,
                format!(
                    "\"{}\" is on the same edge as \"{}\": left out",
                    bars[index].0, bars[taken].0
                ),
            );
        }

        kept
    }

    fn id(&mut self) -> usize {
        self.next_id += 1;

        self.next_id - 1
    }

    fn places(&mut self, at: &str, table: &Table, inherited: Inherited) -> [Vec<Node>; 3] {
        Place::ALL.map(|place| match table.get(place.name()) {
            Some(value) => self.nodes(&format!("{at}.{}", place.name()), value, inherited),
            None => Vec::new(),
        })
    }

    /// A list of nodes, or one node on its own.
    fn nodes(&mut self, at: &str, value: &Value, inherited: Inherited) -> Vec<Node> {
        // Each element of an array is a node, `["a", "b"]` two groups; any
        // other value is one node. An element that can't be read is skipped.
        match value {
            Value::Array(nodes) => nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| self.node(&format!("{at}[{index}]"), node, inherited))
                .collect(),
            _ => self.node(at, value, inherited).into_iter().collect(),
        }
    }

    fn node(&mut self, at: &str, value: &Value, inherited: Inherited) -> Option<Node> {
        let id = self.id();

        let node = match value {
            Value::String(_) | Value::Array(_) => {
                let zones = self.zones(at, value).unwrap_or_default();
                self.plain_group(at, id, zones, inherited)
            }
            Value::Table(table) => {
                self.known_keys(at, table, NODE_KEYS);

                let places = writes_places(table);
                let mut holds = [
                    ("zones", table.contains_key("zones")),
                    ("nodes", table.contains_key("nodes")),
                    ("places", places),
                ]
                .into_iter()
                .filter_map(|(kind, written)| written.then_some(kind));
                let Some(kind) = holds.next() else {
                    self.warn(at, "holds nothing: `zones`, `nodes` or the places");
                    return None;
                };
                if holds.next().is_some() {
                    self.warn(
                        at,
                        format!("holds one of `zones`, `nodes` or the places: `{kind}` is"),
                    );
                }

                let default_shape = kind == "zones" && inherited.shape;
                let (shape, inherited) = self.inherit(at, id, table, inherited, default_shape);

                match kind {
                    "zones" => {
                        let zones_at = format!("{at}.zones");
                        let zones = self.zones(&zones_at, &table["zones"]).unwrap_or_else(|| {
                            self.warn(&zones_at, "expected a zone or a list of zones");
                            Vec::new()
                        });
                        self.group(at, id, zones, shape, inherited)
                    }
                    "nodes" => {
                        let nodes = self.nodes(&format!("{at}.nodes"), &table["nodes"], inherited);
                        Self::holding(id, Holds::Nodes(nodes), shape, inherited)
                    }
                    _ => {
                        let places = self.places(at, table, inherited);
                        Self::holding(id, Holds::Placed(places), shape, inherited)
                    }
                }
            }
            _ => {
                self.warn(
                    at,
                    "expected a zone, a list of zones, or a table with `zones`, `nodes` or the places",
                );
                return None;
            }
        };

        (!node.is_empty()).then_some(node)
    }

    /// A group written as its zones alone.
    fn plain_group(&mut self, at: &str, id: usize, zones: Vec<Zone>, inherited: Inherited) -> Node {
        let (shape, inherited) = self.inherit(at, id, &Table::new(), inherited, inherited.shape);

        self.group(at, id, zones, shape, inherited)
    }

    fn default_group(&mut self, at: &str, names: &[&str], inherited: Inherited) -> Vec<Node> {
        let id = self.id();
        let zones = names
            .iter()
            .filter_map(|name| self.zone(at, name))
            .collect();

        vec![self.plain_group(at, id, zones, inherited)]
    }

    fn holding(id: usize, holds: Holds, shape: bool, inherited: Inherited) -> Node {
        Node {
            id,
            holds,
            shape,
            host: inherited.host,
            panels: inherited.panels,
            hover: inherited.hover,
        }
    }

    fn group(
        &mut self,
        at: &str,
        id: usize,
        zones: Vec<Zone>,
        shape: bool,
        inherited: Inherited,
    ) -> Node {
        let panels = match inherited.panels {
            Panels::Morph if !shape => {
                self.warn(
                    at,
                    "`panels = \"morph\"` needs a shape on the group: its panels drop down",
                );
                Panels::Dropdown
            }
            panels if panels.merges() && inherited.host == Host::None => {
                self.warn(
                    at,
                    "no shape nor background round the group for its panels to merge with: they drop down",
                );
                Panels::Dropdown
            }
            panels => panels,
        };

        Node {
            panels,
            ..Self::holding(id, Holds::Zones(zones), shape, inherited)
        }
    }

    fn zones(&mut self, at: &str, value: &Value) -> Option<Vec<Zone>> {
        match value {
            Value::String(name) => Some(self.zone(at, name).into_iter().collect()),
            Value::Array(names) => Some(self.zone_names(at, names)),
            _ => None,
        }
    }

    fn zone_names(&mut self, at: &str, names: &[Value]) -> Vec<Zone> {
        names
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let at = format!("{at}[{index}]");
                let name = self.string(&at, name)?;
                self.zone(&at, name)
            })
            .collect()
    }

    fn zone(&mut self, at: &str, name: &str) -> Option<Zone> {
        self.placed.insert(name.to_string());

        let builtin = builtin_zone(name);
        let table = self.zones.get(name);
        if builtin.is_none() && table.is_none() {
            self.warn(
                at,
                format!("no zone named \"{name}\": not built in, and no [zone.{name}] table"),
            );
            return None;
        }

        let defaults = |ids: &[ItemId]| ids.iter().copied().map(Item::new).collect::<Vec<_>>();
        let pick = |set: Option<&Vec<Item>>, builtin: Option<&[ItemId]>| {
            set.cloned()
                .or_else(|| builtin.map(defaults))
                .unwrap_or_default()
        };

        Some(Zone {
            name: name.to_string(),
            items: pick(
                table.and_then(|t| t.items.as_ref()),
                builtin.as_ref().map(|b| b.items),
            ),
            details: pick(
                table.and_then(|t| t.details.as_ref()),
                builtin.as_ref().map(|b| b.details),
            ),
            open: table
                .and_then(|t| t.open)
                .or(builtin.as_ref().map(|b| b.open))
                .unwrap_or(Open::Nothing),
        })
    }

    fn zone_tables(&mut self, value: &Value) {
        let Some(zones) = self.table("zone", value) else {
            return;
        };

        for (name, value) in zones {
            let at = format!("zone.{name}");
            let Some(table) = self.table(&at, value) else {
                continue;
            };
            self.known_keys(&at, table, &["items", "details", "open"]);

            let zone = ZoneTable {
                items: table
                    .get("items")
                    .map(|items| self.items(&format!("{at}.items"), items)),
                details: table
                    .get("details")
                    .map(|items| self.items(&format!("{at}.details"), items)),
                open: self.get(&at, table, "open"),
            };
            self.zones.insert(name.clone(), zone);
        }
    }

    fn items(&mut self, at: &str, value: &Value) -> Vec<Item> {
        let Some(items) = self.array(at, value) else {
            return Vec::new();
        };

        items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| self.item(&format!("{at}[{index}]"), item))
            .collect()
    }

    fn item(&mut self, at: &str, value: &Value) -> Option<Item> {
        match value {
            Value::String(name) => self.item_id(at, name).map(Item::new),
            Value::Table(table) => {
                let Some(name) = table.get("item") else {
                    self.warn(at, "missing `item`");
                    return None;
                };
                let name = self.string(&format!("{at}.item"), name)?;
                let mut item = Item::new(self.item_id(at, name)?);

                if let Some(show) = self.get(at, table, "show") {
                    item.show = show;
                }
                if let Some(open) = self.get(at, table, "open") {
                    item.open = Some(open);
                }
                for key in Gestures::KEYS {
                    if let Some(value) = table.get(key) {
                        let action = self.action(&format!("{at}.{key}"), item.id, value);

                        if let (Some(action), Some(slot)) = (action, item.gestures.get_mut(key)) {
                            *slot = Some(action);
                        }
                    }
                }
                item.options = table
                    .iter()
                    .filter(|(key, _)| {
                        !["item", "show", "open"].contains(&key.as_str())
                            && !Gestures::KEYS.contains(&key.as_str())
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect();
                for problem in modules::check_options(item.id, &item.options) {
                    self.warn(at, problem);
                }

                Some(item)
            }
            _ => {
                self.warn(at, "expected an item, or a table with `item`");
                None
            }
        }
    }

    /// An action of the item's by name, `"none"`, `{ open = "<panel>" }` or
    /// `{ run = "<command>" }`.
    fn action(&mut self, at: &str, item: ItemId, value: &Value) -> Option<Action> {
        match value {
            Value::String(name) if name == "none" => Some(Action::None),
            Value::String(name) => {
                let action = modules::actions(item).iter().find(|action| *action == name);

                if action.is_none() {
                    self.warn(
                        at,
                        format!("{} has no action named \"{name}\"", item.name()),
                    );
                }

                action.map(|action| Action::Named(action))
            }
            Value::Table(table) => {
                self.known_keys(at, table, &["open", "run"]);

                if table.contains_key("open") {
                    return self.get::<Open>(at, table, "open").map(|open| match open {
                        Open::Nothing => Action::None,
                        open => Action::Open(open),
                    });
                }

                if let Some(command) = table.get("run") {
                    let command = self.string(&format!("{at}.run"), command)?;

                    return Some(Action::Run(command.to_string()));
                }

                self.warn(at, "expected `open` or `run`");
                None
            }
            _ => {
                self.warn(
                    at,
                    "expected an action's name, or a table with `open` or `run`",
                );
                None
            }
        }
    }

    fn item_id(&mut self, at: &str, name: &str) -> Option<ItemId> {
        let id = ItemId::parse(name);
        if id.is_none() {
            let why = if name.starts_with("custom:") {
                "custom items are not supported yet"
            } else {
                "not in the catalogue"
            };
            self.warn(at, format!("no item named \"{name}\": {why}"));
        }

        id
    }

    /// Returns whether the node has a shape, and what its children inherit.
    fn inherit(
        &mut self,
        at: &str,
        id: usize,
        table: &Table,
        inherited: Inherited,
        default_shape: bool,
    ) -> (bool, Inherited) {
        // `None` if the node doesn't write `shape`.
        let shape_requested: Option<bool> = self.get(at, table, "shape");
        let has_shape = shape_requested.unwrap_or(default_shape);

        let inherited = Inherited {
            hover: self.get(at, table, "hover").unwrap_or(inherited.hover),
            panels: self.get(at, table, "panels").unwrap_or(inherited.panels),
            // No shape for the children if this node has one or writes
            // `shape = false`.
            shape: !has_shape && shape_requested != Some(false) && inherited.shape,
            host: if has_shape {
                Host::Shape(id)
            } else {
                inherited.host
            },
        };

        (has_shape, inherited)
    }

    /// One step for every side, two for top and bottom then the sides, or
    /// four from the top round, as CSS writes them.
    fn padding(&mut self, at: &str, value: &Value) -> Option<Padding> {
        let steps: Vec<Space> = match value {
            Value::Array(values) => values
                .iter()
                .enumerate()
                .map(|(index, value)| self.space(&format!("{at}[{index}]"), value))
                .collect::<Option<_>>()?,
            value => vec![self.space(at, value)?],
        };

        let [top, right, bottom, left] = match steps[..] {
            [all] => [all; 4],
            [vertical, horizontal] => [vertical, horizontal, vertical, horizontal],
            [top, right, bottom, left] => [top, right, bottom, left],
            _ => {
                self.warn(at, "one, two or four steps");
                return None;
            }
        };

        Some(Padding {
            top,
            right,
            bottom,
            left,
        })
    }

    fn get<T: DeserializeOwned>(&mut self, at: &str, table: &Table, key: &str) -> Option<T> {
        let at = format!("{at}.{key}");

        table
            .get(key)?
            .clone()
            .try_into()
            .map_err(|err: toml::de::Error| self.warn(&at, err.message()))
            .ok()
    }

    fn space(&mut self, at: &str, value: &Value) -> Option<Space> {
        let name = self.string(at, value)?;
        let step = step(name);
        if step.is_none() {
            self.warn(at, format!("\"{name}\" is not one of the values it takes"));
        }

        step
    }

    fn known_keys(&mut self, at: &str, table: &Table, known: &[&str]) {
        for key in table.keys() {
            if !known.contains(&key.as_str()) {
                let at = if at.is_empty() {
                    key.clone()
                } else {
                    format!("{at}.{key}")
                };
                self.warn(&at, "unknown key, ignored");
            }
        }
    }

    fn table<'a>(&mut self, at: &str, value: &'a Value) -> Option<&'a Table> {
        let table = value.as_table();
        if table.is_none() {
            self.warn(at, "expected a table");
        }

        table
    }

    fn array<'a>(&mut self, at: &str, value: &'a Value) -> Option<&'a [Value]> {
        let array = value.as_array().map(Vec::as_slice);
        if array.is_none() {
            self.warn(at, "expected a list");
        }

        array
    }

    fn string<'a>(&mut self, at: &str, value: &'a Value) -> Option<&'a str> {
        let string = value.as_str();
        if string.is_none() {
            self.warn(at, "expected a string");
        }

        string
    }

    fn warn(&mut self, at: &str, problem: impl std::fmt::Display) {
        self.problems.push(format!("{at}: {problem}"));
    }
}

fn writes_places(table: &Table) -> bool {
    Place::ALL
        .iter()
        .any(|place| table.contains_key(place.name()))
}

fn step(name: &str) -> Option<Space> {
    Some(match name {
        "s0" => size::S0,
        "s0_5" => size::S0_5,
        "s1" => size::S1,
        "s2" => size::S2,
        "s3" => size::S3,
        "s4" => size::S4,
        "s6" => size::S6,
        "s8" => size::S8,
        "s10" => size::S10,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::super::{Panel, Show, parse};
    use super::*;

    fn read(text: &str) -> Parsed {
        parse(text).expect("valid TOML")
    }

    fn place(bar: &Bar, index: usize) -> &[Node] {
        &bar.places()[index]
    }

    fn names(nodes: &[Node]) -> Vec<Vec<&str>> {
        nodes
            .iter()
            .map(|node| node.zones().iter().map(|zone| zone.name.as_str()).collect())
            .collect()
    }

    fn inner(node: &Node) -> &[Node] {
        match &node.holds {
            Holds::Nodes(nodes) => nodes,
            _ => panic!("not a row"),
        }
    }

    #[test]
    fn an_empty_file_is_ashell_default_bar() {
        let parsed = read("");
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let bar = parsed.config.bars[0].clone();
        assert_eq!(names(place(&bar, 0)), [["workspaces"]]);
        assert_eq!(names(place(&bar, 1)), [["window"]]);
        assert_eq!(names(place(&bar, 2)), [["clock", "privacy", "status"]]);
        assert_eq!(place(&bar, 2)[0].panels, Panels::Dropdown);
        assert!(place(&bar, 2)[0].shape);
        assert!(!bar.root.shape);
    }

    #[test]
    fn placing_anything_replaces_the_default_layout() {
        let bar = read("[bar.main]\ncenter = [\"clock\"]\n")
            .config
            .bars
            .swap_remove(0);

        assert!(place(&bar, 0).is_empty());
        assert_eq!(names(place(&bar, 1)), [["clock"]]);
        assert!(place(&bar, 2).is_empty());
    }

    #[test]
    fn the_short_and_long_forms_are_the_same_bar() {
        let short = read(
            r#"
            [bar.main]
            start = "clock"
            center = [["window", "clock"], "status"]
            "#,
        );
        let long = read(
            r#"
            [bar.main]
            center = [{ zones = ["window", "clock"] }, { zones = "status" }]

            [bar.main.start]
            zones = ["clock"]
            "#,
        );

        assert!(short.problems.is_empty(), "{:?}", short.problems);
        assert!(long.problems.is_empty(), "{:?}", long.problems);
        assert_eq!(short.config.bars[0], long.config.bars[0]);
    }

    #[test]
    fn options_flow_down_and_the_nearest_wins() {
        let bar = read(
            r#"
            [bar.main]
            panels = "merge"
            hover = true
            start = ["workspaces"]
            center = { hover = false, nodes = [{ zones = ["window"], panels = "morph" }, "clock"] }
            "#,
        )
        .config
        .bars[0]
            .clone();

        assert_eq!(place(&bar, 0)[0].panels, Panels::Merge);
        assert!(place(&bar, 0)[0].hover);

        let row = inner(&place(&bar, 1)[0]);
        assert_eq!(row[0].panels, Panels::Morph);
        assert!(!row[0].hover);
        assert_eq!(row[1].panels, Panels::Merge);
        assert!(!row[1].hover);
    }

    #[test]
    fn panels_name_how_they_open() {
        let parsed = read(
            "[bar.main]
panels = \"split\"
center = [\"clock\"]
",
        );
        assert!(parsed.problems[0].starts_with("bar.main.panels:"));

        let bar = parsed.config.bars[0].clone();
        assert_eq!(place(&bar, 1)[0].panels, Panels::Dropdown);
    }

    #[test]
    fn a_shape_round_a_node_leaves_those_inside_without_unless_they_say_so() {
        let parsed = read(
            r#"
            [bar.main]
            background = "solid"
            shape = true
            start = ["workspaces"]
            end = ["clock", { zones = "status", shape = true }]
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let bar = parsed.config.bars[0].clone();
        assert_eq!(bar.background, Background::Solid);
        assert!(bar.root.shape);
        assert!(!place(&bar, 0)[0].shape);
        assert!(!place(&bar, 2)[0].shape);
        assert!(place(&bar, 2)[1].shape);

        let bar = read("[bar.main]\ncenter = { nodes = [\"window\", \"clock\"], shape = true }\n")
            .config
            .bars[0]
            .clone();
        let row = &place(&bar, 1)[0];
        assert!(row.shape);
        assert!(!inner(row)[0].shape);
    }

    #[test]
    fn shape_false_is_the_default_for_what_is_inside() {
        let bar = read(
            r#"
            [bar.main]
            shape = false
            start = ["workspaces"]
            end = ["clock", { zones = "status", shape = true }]
            "#,
        )
        .config
        .bars[0]
            .clone();

        assert!(!bar.root.shape);
        assert!(!place(&bar, 0)[0].shape);
        assert!(!place(&bar, 2)[0].shape);
        assert!(place(&bar, 2)[1].shape);
    }

    #[test]
    fn a_node_places_nodes_at_its_own_ends() {
        let parsed = read(
            r#"
            [bar.main]
            start = { shape = true, start = ["workspaces"], end = ["window"] }
            end   = { shape = true, start = ["media"], end = [["clock", "status"]] }
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let bar = &parsed.config.bars[0];
        let half = &place(bar, 2)[0];
        assert!(half.shape);

        let Holds::Placed(places) = &half.holds else {
            panic!("not placed");
        };
        assert_eq!(names(&places[0]), [["media"]]);
        assert_eq!(names(&places[2]), [["clock", "status"]]);
        assert!(!places[2][0].shape);

        let ids: Vec<usize> = bar.nodes().iter().map(|node| node.id).collect();
        assert_eq!(ids, (0..ids.len()).collect::<Vec<_>>());
    }

    #[test]
    fn with_no_shape_to_use_a_panel_drops_down() {
        let parsed = read(
            r#"
            [bar.main]
            shape = false
            panels = "merge"
            center = [{ zones = "window", panels = "morph" }, "clock"]
            "#,
        );

        let bar = &parsed.config.bars[0];
        assert_eq!(place(bar, 1)[0].panels, Panels::Dropdown);
        assert_eq!(place(bar, 1)[1].panels, Panels::Dropdown);

        let problems = parsed.problems.join("\n");
        assert!(problems.contains("bar.main.center[0]: `panels = \"morph\"` needs a shape"));
        assert!(problems.contains("bar.main.center[1]: no shape nor background round the group"));

        let parsed = read(
            r#"
            [bar.main]
            shape = true
            center = [{ zones = "window", panels = "merge" }, { zones = "clock", panels = "morph" }]
            "#,
        );

        let bar = &parsed.config.bars[0];
        assert_eq!(place(bar, 1)[0].panels, Panels::Merge);
        assert_eq!(place(bar, 1)[1].panels, Panels::Dropdown);
        assert_eq!(parsed.problems.len(), 1, "{:?}", parsed.problems);
    }

    #[test]
    fn a_background_takes_the_panels_of_the_groups_with_no_shape() {
        let parsed = read(
            r#"
            [bar.main]
            background = "solid"
            shape = false
            panels = "merge"
            center = ["clock"]
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(place(&parsed.config.bars[0], 1)[0].panels, Panels::Merge);
    }

    #[test]
    fn a_group_s_host_is_the_nearest_shape_else_the_background() {
        let host = |text: &str, place_index: usize| {
            let bar = read(text).config.bars.swap_remove(0);
            place(&bar, place_index)[0].host
        };

        assert_eq!(
            host("[bar.main]\ncenter = [\"clock\"]\n", 1),
            Host::Shape(1)
        );

        let parsed = read("[bar.main]\nshape = true\ncenter = [\"clock\"]\n");
        let bar = &parsed.config.bars[0];
        assert_eq!(place(bar, 1)[0].host, Host::Shape(bar.root.id));

        let parsed = read("[bar.main]\nend = { nodes = [\"clock\"], shape = true }\n");
        let row = &place(&parsed.config.bars[0], 2)[0];
        assert_eq!(inner(row)[0].host, Host::Shape(row.id));

        let solid = "[bar.main]\nbackground = \"solid\"\nshape = false\ncenter = [\"clock\"]\n";
        assert_eq!(host(solid, 1), Host::Background);
        assert_eq!(
            host("[bar.main]\nshape = false\ncenter = [\"clock\"]\n", 1),
            Host::None
        );
    }

    #[test]
    fn the_padding_is_ds_steps_written_as_css_and_the_look_is_the_theme_s() {
        let padding = |text: &str| read(text).config.bars[0].padding;

        let all = padding("[bar.main]\npadding = \"s4\"\n");
        assert_eq!([all.top, all.right, all.bottom, all.left], [size::S4; 4]);

        let two = padding("[bar.main]\npadding = [\"s0\", \"s4\"]\n");
        assert_eq!(
            [two.top, two.right, two.bottom, two.left],
            [size::S0, size::S4, size::S0, size::S4]
        );

        let four = padding("[bar.main]\npadding = [\"s0\", \"s1\", \"s2\", \"s3\"]\n");
        assert_eq!(
            [four.top, four.right, four.bottom, four.left],
            [size::S0, size::S1, size::S2, size::S3]
        );

        let parsed = read("[bar.main]\npadding = \"7px\"\n");
        assert_eq!(parsed.config.bars[0].padding.top, size::S2);
        assert!(parsed.problems[0].starts_with("bar.main.padding:"));

        let parsed = read("[bar.main]\npadding = [\"s0\", \"s1\", \"s2\"]\n");
        assert!(parsed.problems[0].starts_with("bar.main.padding: one, two or four"));

        let parsed = read("[bar.main]\nradius = \"large\"\nborder = \"bold\"\n");
        let problems = parsed.problems.join("\n");
        assert!(
            problems.contains("bar.main.radius: unknown key"),
            "{problems}"
        );
        assert!(
            problems.contains("bar.main.border: unknown key"),
            "{problems}"
        );
    }

    #[test]
    fn a_monitor_has_the_bars_it_names_and_any_other_the_fallback() {
        let parsed = read(
            r#"
            [bar.main]
            center = ["clock"]

            [bar.media]
            position = "bottom"
            center = ["media"]

            [monitor."Dell U2720Q"]
            bars = ["main", "media"]

            [monitor."DP-2"]
            bars = []

            [monitor."*"]
            bars = ["main"]
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let config = &parsed.config;
        assert_eq!(
            config.bars_on(Some("DP-1"), Some("Dell Inc. Dell U2720Q")),
            [0, 1]
        );
        assert_eq!(config.bars_on(Some("DP-2"), None), [] as [usize; 0]);
        assert_eq!(config.bars_on(Some("eDP-1"), Some("BOE 0x0BCA")), [0]);
        assert_eq!(config.bars_on(None, None), [0]);
    }

    #[test]
    fn with_no_monitors_every_monitor_has_the_first_bar_on_each_edge() {
        let parsed = read(
            r#"
            [bar.main]
            center = ["clock"]

            [bar.other]
            center = ["window"]

            [bar.media]
            position = "bottom"
            center = ["media"]
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        assert_eq!(parsed.config.bars_on(Some("eDP-1"), None), [0, 2]);
    }

    #[test]
    fn a_monitor_names_known_bars_one_per_edge() {
        let parsed = read(
            r#"
            [bar.main]
            center = ["clock"]

            [bar.other]
            center = ["window"]

            [monitor."eDP-1"]
            bars = ["main", "nowhere", "other"]
            "#,
        );
        assert_eq!(parsed.config.bars_on(Some("eDP-1"), None), [0]);

        let problems = parsed.problems.join("\n");
        assert!(
            problems.contains("monitor.eDP-1.bars[1]: no bar named \"nowhere\""),
            "{problems}"
        );
        assert!(
            problems.contains("\"other\" is on the same edge as \"main\""),
            "{problems}"
        );
    }

    #[test]
    fn a_zone_table_changes_a_builtin_zone_or_makes_a_new_one() {
        let parsed = read(
            r#"
            [bar.main]
            center = [["status", "hot"]]

            [zone.status]
            items = ["battery"]

            [zone.hot]
            items = [{ item = "system", show = "notable" }]
            open = "system"
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let zones = &place(&parsed.config.bars[0], 1)[0].zones();
        assert_eq!(zones[0].items, [Item::new(ItemId::Battery)]);
        assert_eq!(
            zones[0].open,
            Open::Panel(Panel::Settings),
            "kept from the defaults"
        );
        assert!(!zones[0].details.is_empty(), "kept from the defaults");

        assert_eq!(zones[1].items[0].id, ItemId::System);
        assert_eq!(zones[1].open, Open::Panel(Panel::System));
        assert!(zones[1].details.is_empty());
    }

    #[test]
    fn an_item_keeps_its_own_options() {
        let parsed = read(
            r#"
            [bar.main]
            end = ["status"]

            [zone.status]
            items = [{ item = "battery", show = "notable", style = "ring", open = "none" }]
            "#,
        );

        let item = &place(&parsed.config.bars[0], 2)[0].zones()[0].items[0];
        assert_eq!(item.show, Show::Notable);
        assert_eq!(item.open, Some(Open::Nothing));
        assert_eq!(
            item.options.get("style").and_then(Value::as_str),
            Some("ring")
        );
    }

    #[test]
    fn a_mistake_costs_only_its_element_and_says_where() {
        let parsed = read(
            r#"
            [bar.main]
            colour = "red"
            center = [["window", "nowhere"], 3, { zones = ["clock"], shape = "blob" }]

            [zone.window]
            items = ["window", "sparkles"]

            [zone.unused]
            items = ["clock"]
            "#,
        );

        let bar = &parsed.config.bars[0];
        assert_eq!(names(place(bar, 1)), [vec!["window"], vec!["clock"]]);
        assert!(place(bar, 1)[1].shape);
        assert_eq!(
            place(bar, 1)[0].zones()[0].items,
            [Item::new(ItemId::Window)]
        );

        let problems = parsed.problems.join("\n");
        for expected in [
            "bar.main.colour: unknown key",
            "bar.main.center[0][1]: no zone named \"nowhere\"",
            "bar.main.center[1]: expected a zone",
            "bar.main.center[2].shape: invalid type: string \"blob\", expected a boolean",
            "zone.window.items[1]: no item named \"sparkles\"",
            "zone.unused: not placed",
        ] {
            assert!(
                problems.contains(expected),
                "{expected:?} not in:\n{problems}"
            );
        }
    }

    #[test]
    fn a_wrong_panel_is_told_the_panels_there_are() {
        let parsed = read(
            "[bar.main]\ncenter = \"hot\"\n\n[zone.hot]\nitems = [\"clock\"]\nopen = \"calendr\"\n",
        );

        let problems = parsed.problems.join("\n");

        assert!(
            problems.contains(
                "zone.hot.open: \"calendr\": expected \"none\", \"config\" or a panel, one of settings, calendar,"
            ),
            "{problems}"
        );
    }

    #[test]
    fn gestures_bind_actions_over_the_items_defaults() {
        let parsed = read(
            r#"
            [bar.main]
            center = "clock"

            [zone.clock]
            items = [
                "clock",
                { item = "clock", scroll_up = "previous_timezone", middle_click = { run = "gnome-calendar" } },
                { item = "date", right_click = { open = "calendar" }, scroll_down = "none" },
            ]
            "#,
        );
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);

        let items = &place(&parsed.config.bars[0], 1)[0].zones()[0].items;

        assert_eq!(items[0].gestures, modules::default_gestures(ItemId::Clock));
        assert_eq!(
            items[1].gestures.scroll_up,
            Some(Action::Named("previous_timezone"))
        );
        assert_eq!(
            items[1].gestures.right_click,
            Some(Action::Named("cycle_format"))
        );
        assert_eq!(
            items[1].gestures.middle_click,
            Some(Action::Run("gnome-calendar".into()))
        );
        assert_eq!(
            items[2].gestures.right_click,
            Some(Action::Open(Open::Panel(Panel::Calendar)))
        );
        assert_eq!(items[2].gestures.scroll_down, Some(Action::None));
        assert!(items[1].options.is_empty());
    }

    #[test]
    fn wrong_gestures_and_options_are_reported() {
        let parsed = read(
            r#"
            [bar.main]
            center = "clock"

            [zone.clock]
            items = [
                { item = "date", right_click = "cycle_format", format = "%Q" },
                { item = "clock", middle_click = { open = "nowhere" }, colour = "red" },
                { item = "forecast", hours = 40 },
            ]
            "#,
        );

        let problems = parsed.problems.join("\n");

        for expected in [
            "zone.clock.items[0].right_click: date has no action named \"cycle_format\"",
            "zone.clock.items[0]: format: \"%Q\" is not a date format",
            "zone.clock.items[1].middle_click.open: \"nowhere\": expected \"none\", \"config\" or a panel",
            "zone.clock.items[1]: colour: unknown option of clock",
            "zone.clock.items[2]: hours: expected a number from 1 to 12",
        ] {
            assert!(
                problems.contains(expected),
                "{expected:?} not in:\n{problems}"
            );
        }
    }

    #[test]
    fn the_appearance_is_styletto_own() {
        let parsed = read("[appearance]\ntheme = \"nord\"\nmaterial = \"glass\"\n");
        assert_eq!(parsed.config.appearance.theme.as_deref(), Some("nord"));
        assert_eq!(
            parsed.config.appearance.material,
            styletto::ui::Material::Glass
        );

        let parsed = read("[appearance]\nmaterial = \"paper\"\n");
        assert_eq!(parsed.config.appearance, UiConfig::default());
        assert!(parsed.problems[0].starts_with("appearance:"));
    }

    #[test]
    fn the_documents_examples_read_without_problems() {
        for example in [
            include_str!("../../docs/examples/ashell.toml"),
            include_str!("../../docs/examples/morph.toml"),
            include_str!("../../docs/examples/middle.toml"),
            include_str!("../../docs/examples/solid.toml"),
            include_str!("../../docs/examples/floating.toml"),
        ] {
            let parsed = read(example);
            assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        }
    }
}
