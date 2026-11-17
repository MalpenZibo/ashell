pub mod catalogue;
mod layout;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use guido::prelude::*;
use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask, Watches};
use serde::Deserialize;
use styletto::foundations::tokens::Space;
use styletto::theme::config::UiConfig;

pub use catalogue::{ItemId, Open, Panel, Show};

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub appearance: UiConfig,
    pub bars: Vec<Bar>,
    /// The first that matches a monitor gives it its bars; the last matches
    /// any.
    pub monitors: Vec<Monitor>,
}

impl Config {
    pub fn bars_on(&self, name: Option<&str>, description: Option<&str>) -> &[usize] {
        self.monitors
            .iter()
            .find(|monitor| monitor.on.matches(name, description))
            .map_or(&[], |monitor| &monitor.bars)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub on: Spec,
    pub bars: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Spec {
    Any,
    Named(String),
}

impl Spec {
    pub fn matches(&self, name: Option<&str>, description: Option<&str>) -> bool {
        match self {
            Spec::Any => true,
            Spec::Named(spec) => {
                name == Some(spec.as_str()) || description.is_some_and(|text| text.contains(spec))
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        parse("").expect("the empty file is valid TOML").config
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub position: Position,
    pub background: Background,
    pub padding: Padding,
    /// Always placed.
    pub root: Node,
}

impl Bar {
    pub fn places(&self) -> &[Vec<Node>; 3] {
        match &self.root.holds {
            Holds::Placed(places) => places,
            _ => unreachable!("the root is placed"),
        }
    }

    pub fn nodes(&self) -> Vec<&Node> {
        let mut all = Vec::new();
        self.root.walk(&mut all);

        all
    }
}

/// Between the screen's edges and the outermost shapes, as CSS writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Padding {
    pub top: Space,
    pub right: Space,
    pub bottom: Space,
    pub left: Space,
}

impl Padding {
    pub fn all(step: Space) -> Self {
        Self {
            top: step,
            right: step,
            bottom: step,
            left: step,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    #[default]
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Background {
    #[default]
    Transparent,
    Solid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Its place in the bar, in the order written: the root is 0.
    pub id: usize,
    pub holds: Holds,
    pub shape: bool,
    /// What the panels and details of the groups in it come out of.
    pub host: Host,
    pub panels: Panels,
    pub hover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Holds {
    Zones(Vec<Zone>),
    Nodes(Vec<Node>),
    Placed([Vec<Node>; 3]),
}

impl Node {
    pub fn is_group(&self) -> bool {
        matches!(self.holds, Holds::Zones(_))
    }

    pub fn zones(&self) -> &[Zone] {
        match &self.holds {
            Holds::Zones(zones) => zones,
            _ => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.holds {
            Holds::Zones(zones) => zones.is_empty(),
            Holds::Nodes(nodes) => nodes.is_empty(),
            Holds::Placed(places) => places.iter().all(Vec::is_empty),
        }
    }

    fn walk<'a>(&'a self, all: &mut Vec<&'a Node>) {
        all.push(self);
        match &self.holds {
            Holds::Zones(_) => {}
            Holds::Nodes(nodes) => nodes.iter().for_each(|node| node.walk(all)),
            Holds::Placed(places) => places.iter().flatten().for_each(|node| node.walk(all)),
        }
    }
}

/// What a group's panels and details come out of: the nearest shape at or
/// round it, the group's own or a node's, by the node's id; else the bar's
/// background, else nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Shape(usize),
    Background,
    None,
}

/// Where in a placed node: `start`, `center` or `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Start,
    Center,
    End,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Start, Place::Center, Place::End];

    pub fn name(self) -> &'static str {
        match self {
            Place::Start => "start",
            Place::Center => "center",
            Place::End => "end",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panels {
    Morph,
    Attached,
    Detached,
    #[default]
    Dropdown,
}

impl Panels {
    pub fn merges(self) -> bool {
        matches!(self, Panels::Attached | Panels::Detached)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub name: String,
    pub items: Vec<Item>,
    pub details: Vec<Item>,
    pub open: Open,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: ItemId,
    pub show: Show,
    pub open: Option<Open>,
    pub options: toml::Table,
}

impl Item {
    pub fn new(id: ItemId) -> Self {
        Self {
            id,
            show: id.default_show(),
            open: id.default_open(),
            options: toml::Table::new(),
        }
    }
}

#[derive(Debug)]
pub struct Parsed {
    pub config: Config,
    pub problems: Vec<String>,
}

pub fn parse(text: &str) -> Result<Parsed, toml::de::Error> {
    let file: toml::Table = toml::from_str(text)?;

    Ok(layout::read(&file))
}

pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(|| Some(std::env::home_dir()?.join(".config")))?;

    Some(base.join("ashell").join("ashell-experimental.toml"))
}

pub fn load() -> Config {
    reload().unwrap_or_default()
}

pub fn reload() -> Option<Config> {
    let Some(path) = path() else {
        return Some(Config::default());
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Some(Config::default()),
        Err(err) => {
            log::error!("{}: {err}", path.display());
            return None;
        }
    };

    match parse(&text) {
        Ok(parsed) => {
            for problem in &parsed.problems {
                log::warn!("{}: {problem}", path.display());
            }
            Some(parsed.config)
        }
        Err(err) => {
            log::error!("{}: {err}", path.display());
            None
        }
    }
}

pub fn watched() -> Signal<Config> {
    let config = create_signal(load());
    let writer = config.writer();

    create_task(move |_| async move {
        if let Err(err) = watch(writer).await {
            log::error!("the configuration is not watched, edits need a restart: {err}");
        }
    });

    config.read_only()
}

async fn watch(config: WriteSignal<Config>) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no $XDG_CONFIG_HOME nor $HOME"))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }

    // A symlink is watched where it is, for the link made, deleted or
    // replaced, and where it points, for the file edited; pointed elsewhere,
    // it is watched there.
    let inotify = Inotify::init()?;
    let mut watches = inotify.watches();
    let link = watch_file(&mut watches, &path)?;
    let mut target = pointed_to(&path);
    let mut pointed = watch_file(&mut watches, &target)?;

    // The events that come together are read together: an editor that moves
    // the file away and writes it again makes the bars once, from the new
    // file, rather than once from the default in between.
    let mut events = inotify.into_event_stream([0; 1024])?.ready_chunks(16);

    while let Some(events) = events.next().await {
        let mut touched = false;
        for event in events {
            let event = event?;
            let is = |watched: &Option<Watched>| {
                watched.as_ref().is_some_and(|(watch, name)| {
                    event.wd == *watch && event.name.as_deref() == Some(name)
                })
            };
            // A file made is read once written; a link made, now.
            let made = event.mask.contains(EventMask::CREATE);
            touched |= (is(&pointed) && !made) || (is(&link) && (!made || path.is_symlink()));
        }
        if !touched {
            continue;
        }

        let now = pointed_to(&path);
        if now != target {
            let next = watch_file(&mut watches, &now).unwrap_or_else(|err| {
                log::warn!("{}: not watched: {err}", now.display());
                None
            });
            let watch =
                |watched: &Option<Watched>| watched.as_ref().map(|(watch, _)| watch.clone());
            if let Some(old) = watch(&pointed)
                && Some(&old) != watch(&link).as_ref()
                && Some(&old) != watch(&next).as_ref()
            {
                let _ = watches.remove(old);
            }
            (target, pointed) = (now, next);
        }

        // A file deleted or moved away reads as the default bar. The bars are
        // made again only if what is read differs.
        if let Some(read) = reload() {
            log::info!("configuration read again");
            config.set(read);
        }
    }

    Ok(())
}

type Watched = (WatchDescriptor, OsString);

/// Watches the folder `file` is in for it.
fn watch_file(watches: &mut Watches, file: &Path) -> std::io::Result<Option<Watched>> {
    let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
        return Ok(None);
    };
    let watch = watches.add(
        folder,
        WatchMask::CLOSE_WRITE
            | WatchMask::CREATE
            | WatchMask::MOVED_TO
            | WatchMask::DELETE
            | WatchMask::MOVED_FROM,
    )?;

    Ok(Some((watch, name.to_os_string())))
}

/// The file a symlink points to, or the file itself.
fn pointed_to(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
