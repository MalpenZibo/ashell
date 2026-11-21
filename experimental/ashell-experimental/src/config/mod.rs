pub mod catalogue;
mod import;
mod layout;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use futures::StreamExt;
use guido::prelude::*;
use inotify::{Event, EventMask, Inotify, WatchDescriptor, WatchMask, Watches};
use serde::Deserialize;
use styletto::foundations::tokens::Space;
use styletto::theme::config::UiConfig;

pub use catalogue::{Action, Gestures, ItemId, Open, Panel, Show};

use crate::modules;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub appearance: UiConfig,
    pub modules: modules::Configs,
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
    /// How far the panels and details stand off what they come out of: close,
    /// a neck joins them; far, it breaks as they arrive.
    pub gap: Space,
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

    pub const fn name(self) -> &'static str {
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
    Merge,
    #[default]
    Dropdown,
}

impl Panels {
    pub fn merges(self) -> bool {
        self == Panels::Merge
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
    pub gestures: Gestures,
    /// The item's own options, as its module reads them.
    pub options: toml::Table,
}

impl Item {
    pub fn new(id: ItemId) -> Self {
        Self {
            id,
            show: id.default_show(),
            open: id.default_open(),
            gestures: modules::default_gestures(id),
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
    Ok(parse_in(text, Path::new("."))?.0)
}

/// `text` read as a config in `folder`, its imports followed: what it says,
/// and the files it imports.
fn parse_in(text: &str, folder: &Path) -> Result<(Parsed, Vec<PathBuf>), toml::de::Error> {
    let mut file: toml::Table = toml::from_str(text)?;
    let mut problems = Vec::new();
    let imported = import::follow(&mut file, folder, &mut problems);
    let mut parsed = layout::read(&file);
    problems.append(&mut parsed.problems);
    parsed.problems = problems;

    Ok((parsed, imported))
}

pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .or_else(|| Some(std::env::home_dir()?.join(".config")))?;

    Some(base.join("ashell").join("ashell-experimental.toml"))
}

/// The config read again, with the files it imports. `None` while it cannot
/// be read: the bars stay as they are.
fn reload() -> Option<(Config, Vec<PathBuf>)> {
    let Some(path) = path() else {
        return Some((Config::default(), Vec::new()));
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Some((Config::default(), Vec::new()));
        }
        Err(err) => {
            log::error!("{}: {err}", path.display());
            return None;
        }
    };

    match parse_in(&text, path.parent().unwrap_or(Path::new("."))) {
        Ok((parsed, imported)) => {
            for problem in &parsed.problems {
                log::warn!("{}: {problem}", path.display());
            }
            Some((parsed.config, imported))
        }
        Err(err) => {
            log::error!("{}: {err}", path.display());
            None
        }
    }
}

pub fn watched() -> Signal<Config> {
    let (read, imported) = reload().unwrap_or_default();
    let config = create_signal(read);
    let writer = config.writer();

    create_task(move |_| async move {
        if let Err(err) = watch(writer, imported).await {
            log::error!("the configuration is not watched, edits need a restart: {err}");
        }
    });

    config.read_only()
}

async fn watch(config: WriteSignal<Config>, mut imported: Vec<PathBuf>) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no $XDG_CONFIG_HOME nor $HOME"))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }

    let inotify = Inotify::init()?;
    let mut watching = Watching::new(inotify.watches());
    watching.follow(files(&path, &imported));

    // The events that come together are read together: an editor that moves
    // the file away and writes it again makes the bars once, from the new
    // file, rather than once from the default in between.
    let mut events = inotify.into_event_stream([0; 1024])?.ready_chunks(16);

    while let Some(events) = events.next().await {
        let mut touched = false;
        for event in events {
            touched |= watching.touches(&event?);
        }
        if !touched {
            continue;
        }

        // A file deleted or moved away reads as the default bar, an import
        // gone as the table written. The bars are made again only if what is
        // read differs.
        if let Some((read, now)) = reload() {
            log::info!("configuration read again");
            config.set(read);
            imported = now;
        }
        watching.follow(files(&path, &imported));
    }

    Ok(())
}

/// The files whose change changes the config: it and the files it imports,
/// each where it is, for a link made, deleted or replaced, and where it
/// points, for the file edited.
fn files(path: &Path, imported: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for file in std::iter::once(path).chain(imported.iter().map(PathBuf::as_path)) {
        for file in [file.to_path_buf(), pointed_to(file)] {
            if !files.contains(&file) {
                files.push(file);
            }
        }
    }

    files
}

/// The folders of the files followed, each watched once for them.
struct Watching {
    watches: Watches,
    folders: HashMap<PathBuf, WatchDescriptor>,
    files: Vec<PathBuf>,
}

impl Watching {
    fn new(watches: Watches) -> Self {
        Self {
            watches,
            folders: HashMap::new(),
            files: Vec::new(),
        }
    }

    /// Follows `files` from now on, and no other.
    fn follow(&mut self, files: Vec<PathBuf>) {
        let folders: Vec<&Path> = files.iter().filter_map(|file| file.parent()).collect();

        self.folders.retain(|folder, wd| {
            let kept = folders.contains(&folder.as_path());
            if !kept {
                let _ = self.watches.remove(wd.clone());
            }

            kept
        });
        for folder in folders {
            if self.folders.contains_key(folder) {
                continue;
            }
            let mask = WatchMask::CLOSE_WRITE
                | WatchMask::CREATE
                | WatchMask::MOVED_TO
                | WatchMask::DELETE
                | WatchMask::MOVED_FROM;

            match self.watches.add(folder, mask) {
                Ok(wd) => {
                    self.folders.insert(folder.to_path_buf(), wd);
                }
                Err(err) => log::warn!("{}: not watched: {err}", folder.display()),
            }
        }
        self.files = files;
    }

    /// Whether `event` changed a file followed. A file made is read once
    /// written; a link made, now.
    fn touches(&self, event: &Event<std::ffi::OsString>) -> bool {
        let made = event.mask.contains(EventMask::CREATE);

        self.files.iter().any(|file| {
            event.name.as_deref() == file.file_name()
                && file
                    .parent()
                    .and_then(|folder| self.folders.get(folder))
                    .is_some_and(|wd| *wd == event.wd)
                && (!made || file.is_symlink())
        })
    }
}

/// The file a symlink points to, through every link on the way, or the file
/// itself. Where it points even while nothing is there: a file deleted and
/// written again, as a checkout does, is still watched where it comes back.
fn pointed_to(path: &Path) -> PathBuf {
    let mut file = path.to_path_buf();
    // As many links as the kernel follows.
    for _ in 0..40 {
        let Ok(next) = std::fs::read_link(&file) else {
            break;
        };
        file = match file.parent() {
            Some(folder) => folder.join(next),
            None => next,
        };
    }

    // Its folder resolved, which is there even when the file is not.
    match (file.parent().map(std::fs::canonicalize), file.file_name()) {
        (Some(Ok(folder)), Some(name)) => folder.join(name),
        _ => file,
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use inotify::Inotify;

    use super::{Watching, files, pointed_to};

    #[test]
    fn a_link_is_followed_to_where_it_points_even_with_nothing_there() {
        let dir = std::env::temp_dir().join(format!("ashell-pointed-{}", std::process::id()));
        let (config, dotfiles) = (dir.join("config"), dir.join("dotfiles"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&dotfiles).unwrap();

        let file = config.join("bar.toml");
        let dir = std::fs::canonicalize(&dir).unwrap();
        let dotfiles = dir.join("dotfiles");
        assert_eq!(pointed_to(&file), dir.join("config/bar.toml"));

        // Relative, through a second link, and to a file not written yet.
        symlink("../dotfiles/link.toml", &file).unwrap();
        symlink("bar.toml", dotfiles.join("link.toml")).unwrap();
        assert_eq!(pointed_to(&file), dotfiles.join("bar.toml"));

        std::fs::write(dotfiles.join("bar.toml"), "").unwrap();
        assert_eq!(pointed_to(&file), dotfiles.join("bar.toml"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_imported_file_is_followed_from_when_it_comes_until_it_is_no_longer_imported() {
        let dir = std::env::temp_dir().join(format!("ashell-watching-{}", std::process::id()));
        let (config, shared) = (dir.join("config"), dir.join("shared"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        let dir = std::fs::canonicalize(&dir).unwrap();
        let (path, look) = (dir.join("config/bar.toml"), dir.join("shared/look.toml"));

        let mut inotify = Inotify::init().unwrap();
        let mut watching = Watching::new(inotify.watches());
        let mut buffer = [0; 1024];
        let mut touched = |inotify: &mut Inotify, watching: &Watching| {
            inotify
                .read_events(&mut buffer)
                .map(|events| events.into_iter().any(|e| watching.touches(&e.to_owned())))
                .unwrap_or(false)
        };

        watching.follow(files(&path, std::slice::from_ref(&look)));
        std::fs::write(&look, "theme = \"nord\"").unwrap();
        assert!(touched(&mut inotify, &watching));

        std::fs::write(shared.join("other.toml"), "").unwrap();
        assert!(!touched(&mut inotify, &watching));

        watching.follow(files(&path, &[]));
        std::fs::write(&look, "theme = \"dracula\"").unwrap();
        assert!(!touched(&mut inotify, &watching));

        std::fs::write(&path, "").unwrap();
        assert!(touched(&mut inotify, &watching));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
