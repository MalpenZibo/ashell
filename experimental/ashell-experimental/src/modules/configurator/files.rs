//! What the config window writes: the config, from a layout of the site's
//! examples, and the look, in the file its `[appearance]` imports.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ashell_i18n::t;
use styletto::theme::config::UiConfig;
use toml_edit::{DocumentMut, Item, Table};

use crate::config::{Position, folder, import};

pub struct Layout {
    pub name: fn() -> String,
    pub about: fn() -> String,
    example: &'static str,
    /// The place of `[bar.main]` whose end the config zone is put at.
    place: &'static str,
}

pub const LAYOUTS: [Layout; 4] = [
    Layout {
        name: || t!("config-layout-ashell"),
        about: || t!("config-layout-ashell-about"),
        example: include_str!("../../../docs/examples/ashell.toml"),
        place: "end",
    },
    Layout {
        name: || t!("config-layout-solid"),
        about: || t!("config-layout-solid-about"),
        example: include_str!("../../../docs/examples/solid.toml"),
        place: "end",
    },
    Layout {
        name: || t!("config-layout-floating"),
        about: || t!("config-layout-floating-about"),
        example: include_str!("../../../docs/examples/floating.toml"),
        place: "end",
    },
    Layout {
        name: || t!("config-layout-morph"),
        about: || t!("config-layout-morph-about"),
        example: include_str!("../../../docs/examples/morph.toml"),
        place: "center",
    },
];

impl Layout {
    /// Its example, with the config zone to open the window again, on the
    /// edge at `position`.
    fn text(&self, position: Position) -> String {
        let mut doc: DocumentMut = self.example.parse().expect("an example is valid TOML");
        if let Some(places) = doc["bar"]["main"][self.place].as_array_mut() {
            places.push("config");
        }
        set_position(&mut doc, position).expect("an example has a [bar.main]");

        doc.to_string()
    }
}

/// Where the look goes when the config imports none: beside it.
const LOOK: &str = "appearance.toml";

const LOOK_HEADER: &str = "# The look chosen in the config window of ashell-experimental, written\n\
# again on each change there.\n\n";

/// What the config is, its look aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// No file yet.
    Missing,
    /// One of [`LAYOUTS`], as the config window wrote it.
    Layout(usize),
    /// Written by hand: a layout would replace it.
    Own,
}

/// What the config is, and the edge of its main bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub start: Start,
    /// The top while there is no config; `None` when it cannot be read or
    /// has no `[bar.main]` to set it in.
    pub position: Option<Position>,
}

pub fn status(config: &Path) -> Status {
    let own = Status {
        start: Start::Own,
        position: None,
    };
    let text = match std::fs::read_to_string(config) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Status {
                start: Start::Missing,
                position: Some(Position::Top),
            };
        }
        Err(_) => return own,
    };
    let (Ok(doc), Some(written)) = (text.parse::<DocumentMut>(), without_look(&text)) else {
        return own;
    };
    let start = WRITTEN
        .iter()
        .position(|layout| layout.as_ref() == Some(&written))
        .map_or(Start::Own, Start::Layout);

    Status {
        start,
        position: edge(&doc),
    }
}

/// Each of [`LAYOUTS`] as a config written from it reads in [`status`],
/// worked out once.
static WRITTEN: LazyLock<Vec<Option<toml::Table>>> = LazyLock::new(|| {
    LAYOUTS
        .iter()
        .map(|layout| without_look(&layout.text(Position::Top)))
        .collect()
});

/// The config without its look and the main bar's edge, which the config window
/// sets apart from the layout.
fn without_look(text: &str) -> Option<toml::Table> {
    let mut table: toml::Table = toml::from_str(text).ok()?;
    table.remove("appearance");
    if let Some(main) = table
        .get_mut("bar")
        .and_then(|bar| bar.get_mut("main"))
        .and_then(toml::Value::as_table_mut)
    {
        main.remove("position");
    }

    Some(table)
}

/// The edge of the main bar, `None` when there is no `[bar.main]`. As the
/// config reads it: a value it can't read is the default.
fn edge(doc: &DocumentMut) -> Option<Position> {
    let main = doc.get("bar")?.get("main")?;

    Some(
        main.get("position")
            .and_then(Item::as_str)
            .and_then(|name| toml::Value::from(name).try_into().ok())
            .unwrap_or_default(),
    )
}

/// Moves the main bar to the edge at `position`; a missing config is written
/// as [`choose`] writes the first of [`LAYOUTS`].
pub fn save_position(config: &Path, position: Position, look: &UiConfig) -> Result<(), String> {
    let Some(mut doc) = read(config)? else {
        return write_layout(config, &LAYOUTS[0], position, None, look);
    };
    set_position(&mut doc, position).ok_or_else(|| t!("config-no-main-bar"))?;

    write(config, &doc.to_string())
}

/// `position` in the main bar's table, as the config reads it, the
/// default as no key;
/// `None` when there is no `[bar.main]`.
fn set_position(doc: &mut DocumentMut, position: Position) -> Option<()> {
    let main = doc.get_mut("bar")?.get_mut("main")?.as_table_like_mut()?;
    if position == Position::default() {
        main.remove("position");
    } else {
        let name = toml::Value::try_from(position).expect("a position is written as its name");
        main.insert("position", toml_edit::value(name.as_str()?));
    }

    Some(())
}

/// Writes `layout` as the config, which is [`Start::Missing`] or a layout,
/// keeping its `[appearance]` and the main bar's edge.
pub fn choose(config: &Path, layout: &Layout, look: &UiConfig) -> Result<(), String> {
    let doc = read(config)?;
    let position = doc.as_ref().and_then(edge).unwrap_or_default();
    let kept = doc.and_then(|doc| doc.get("appearance").cloned());

    write_layout(config, layout, position, kept, look)
}

/// Writes `layout` on the edge at `position`, under the `appearance` kept,
/// or one importing `look`, saved beside the config unless a file is there
/// already, which it then imports as it is.
fn write_layout(
    config: &Path,
    layout: &Layout,
    position: Position,
    kept: Option<Item>,
    look: &UiConfig,
) -> Result<(), String> {
    let appearance = match kept {
        Some(appearance) => appearance,
        None => {
            let file = folder(config).join(LOOK);
            if !file.exists() {
                write(&file, &look_text(look)?)?;
            }
            Item::Table(importing(LOOK))
        }
    };

    write(config, &with_look(appearance, &layout.text(position)))
}

/// Saves `look` in the file the config's `[appearance]` imports. A config
/// whose look is written in it is made to import it from `appearance.toml`
/// beside it, unless a file is there already; a missing one is written as
/// [`choose`] writes the first of [`LAYOUTS`]. Keys written beside an import
/// win over it.
pub fn save_look(config: &Path, look: &UiConfig) -> Result<(), String> {
    let Some(mut doc) = read(config)? else {
        return choose(config, &LAYOUTS[0], look);
    };

    if let Some(file) = imported(&doc, config) {
        return write(&file, &look_text(look)?);
    }

    let file = folder(config).join(LOOK);
    if file.exists() {
        return Err(format!(
            "{}: {}",
            file.display(),
            t!("config-look-not-imported")
        ));
    }
    write(&file, &look_text(look)?)?;

    // The table where it was, or first.
    let text = match doc
        .get("appearance")
        .and_then(Item::as_table)
        .and_then(Table::position)
    {
        Some(position) => {
            let mut table = importing(LOOK);
            table.set_position(position);
            doc.insert("appearance", Item::Table(table));
            doc.to_string()
        }
        None => {
            doc.remove("appearance");
            with_look(Item::Table(importing(LOOK)), &doc.to_string())
        }
    };

    write(config, &text)
}

/// The config, `None` when it is not there yet.
fn read(config: &Path) -> Result<Option<DocumentMut>, String> {
    match std::fs::read_to_string(config) {
        Ok(text) => text
            .parse()
            .map(Some)
            .map_err(|err| format!("{}: {err}", config.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("{}: {err}", config.display())),
    }
}

fn write(file: &Path, text: &str) -> Result<(), String> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|err| format!("{}: {err}", folder.display()))?;
    }

    std::fs::write(file, text).map_err(|err| format!("{}: {err}", file.display()))
}

/// The file the `[appearance]` of `doc`, the config at `config`, imports.
fn imported(doc: &DocumentMut, config: &Path) -> Option<PathBuf> {
    let path = doc.get("appearance")?.get("import")?.as_str()?;

    Some(import::resolve(folder(config), path))
}

fn look_text(look: &UiConfig) -> Result<String, String> {
    toml::to_string(look)
        .map(|text| format!("{LOOK_HEADER}{text}"))
        .map_err(|err| err.to_string())
}

fn importing(path: &str) -> Table {
    let mut table = Table::new();
    table.insert("import", toml_edit::value(path));

    table
}

/// `rest` under an `[appearance]` of `appearance`.
fn with_look(appearance: Item, rest: &str) -> String {
    let mut doc = DocumentMut::new();
    doc.insert("appearance", appearance);

    format!("{doc}\n{rest}")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::testing::Folder;

    /// Where `folder`'s config is.
    fn config_in(folder: &Folder) -> PathBuf {
        folder.join("ashell-experimental.toml")
    }

    /// The file the config imports its look from.
    fn imported_by(config: &Path) -> Option<PathBuf> {
        imported(&read(config).ok()??, config)
    }

    fn start(config: &Path) -> Start {
        status(config).start
    }

    fn position(config: &Path) -> Option<Position> {
        status(config).position
    }

    fn nord() -> UiConfig {
        toml::from_str("theme = \"nord\"").unwrap()
    }

    #[test]
    fn a_first_look_writes_the_first_layout_importing_it() {
        let folder = Folder::new("configurator-first");
        assert_eq!(start(&config_in(&folder)), Start::Missing);

        save_look(&config_in(&folder), &nord()).unwrap();

        assert_eq!(start(&config_in(&folder)), Start::Layout(0));
        assert_eq!(imported_by(&config_in(&folder)), Some(folder.join(LOOK)));
        let look: UiConfig = toml::from_str(&folder.read(LOOK)).unwrap();
        assert_eq!(look, nord());
    }

    #[test]
    fn a_layout_keeps_the_look_and_a_config_of_ones_own_is_not_one() {
        let folder = Folder::new("configurator-layout");
        choose(&config_in(&folder), &LAYOUTS[0], &nord()).unwrap();
        std::fs::write(folder.join(LOOK), "theme = \"dracula\"").unwrap();

        choose(&config_in(&folder), &LAYOUTS[1], &UiConfig::default()).unwrap();

        assert_eq!(start(&config_in(&folder)), Start::Layout(1));
        assert_eq!(folder.read(LOOK), "theme = \"dracula\"");
        assert!(
            folder
                .read("ashell-experimental.toml")
                .starts_with("[appearance]\nimport = \"appearance.toml\"\n")
        );

        std::fs::write(config_in(&folder), "[bar.main]\nstart = [\"clock\"]\n").unwrap();
        assert_eq!(start(&config_in(&folder)), Start::Own);
    }

    #[test]
    fn an_existing_look_file_is_imported_by_a_new_config_as_it_is() {
        let folder = Folder::new("configurator-existing");
        std::fs::write(folder.join(LOOK), "theme = \"dracula\"").unwrap();

        choose(&config_in(&folder), &LAYOUTS[1], &nord()).unwrap();

        assert_eq!(folder.read(LOOK), "theme = \"dracula\"");
        assert_eq!(imported_by(&config_in(&folder)), Some(folder.join(LOOK)));
    }

    #[test]
    fn a_look_written_in_the_config_moves_to_a_file_it_imports() {
        let folder = Folder::new("configurator-inline");
        std::fs::write(
            config_in(&folder),
            "# My bar.\n[bar.main]\nstart = [\"clock\"]\n\n[appearance]\ntheme = \"nord\"\n\n[tempo]\nformat = \"%H\"\n",
        )
        .unwrap();

        save_look(&config_in(&folder), &nord()).unwrap();

        assert_eq!(
            folder.read("ashell-experimental.toml"),
            "# My bar.\n[bar.main]\nstart = [\"clock\"]\n\n[appearance]\nimport = \"appearance.toml\"\n\n[tempo]\nformat = \"%H\"\n"
        );
        assert_eq!(
            toml::from_str::<UiConfig>(&folder.read(LOOK)).unwrap(),
            nord()
        );

        // Now imported, the file is written again.
        save_look(&config_in(&folder), &UiConfig::default()).unwrap();
        assert_eq!(
            toml::from_str::<UiConfig>(&folder.read(LOOK)).unwrap(),
            UiConfig::default()
        );
    }

    #[test]
    fn a_look_file_not_imported_is_left_alone() {
        let folder = Folder::new("configurator-alone");
        std::fs::write(config_in(&folder), "[appearance]\ntheme = \"nord\"\n").unwrap();
        std::fs::write(folder.join(LOOK), "mine").unwrap();

        assert!(save_look(&config_in(&folder), &UiConfig::default()).is_err());
        assert_eq!(folder.read(LOOK), "mine");
        assert_eq!(
            folder.read("ashell-experimental.toml"),
            "[appearance]\ntheme = \"nord\"\n"
        );
    }

    #[test]
    fn the_edge_is_kept_by_a_layout_and_left_out_of_it() {
        let folder = Folder::new("configurator-edge");
        assert_eq!(position(&config_in(&folder)), Some(Position::Top));

        save_position(&config_in(&folder), Position::Bottom, &nord()).unwrap();
        assert_eq!(start(&config_in(&folder)), Start::Layout(0));
        assert_eq!(position(&config_in(&folder)), Some(Position::Bottom));

        choose(&config_in(&folder), &LAYOUTS[3], &nord()).unwrap();
        assert_eq!(start(&config_in(&folder)), Start::Layout(3));
        assert_eq!(position(&config_in(&folder)), Some(Position::Bottom));

        save_position(&config_in(&folder), Position::Top, &nord()).unwrap();
        assert!(!folder.read("ashell-experimental.toml").contains("position"));

        std::fs::write(config_in(&folder), "[bar.side]\nstart = [\"clock\"]\n").unwrap();
        assert_eq!(position(&config_in(&folder)), None);
        assert!(save_position(&config_in(&folder), Position::Bottom, &nord()).is_err());
    }

    #[test]
    fn every_layout_reads_without_a_problem_on_either_edge() {
        for layout in &LAYOUTS {
            for edge in [Position::Top, Position::Bottom] {
                let text = layout.text(edge);
                let written: toml::Table = toml::from_str(&text).unwrap();
                let places = written["bar"]["main"][layout.place].as_array().unwrap();
                assert!(
                    places.iter().any(|place| place.as_str() == Some("config")),
                    "{}: no config zone",
                    (layout.name)()
                );

                let parsed = crate::config::parse(&text).unwrap();
                assert!(
                    parsed.problems.is_empty(),
                    "{}: {:?}",
                    (layout.name)(),
                    parsed.problems
                );
                assert_eq!(parsed.config.bars[0].position, edge);
            }
        }
    }
}
