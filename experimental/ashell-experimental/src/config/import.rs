//! `import = "path"` in a top-level table: the table read from a file of its
//! own, with the keys set beside the import written over it, table into
//! table. The file holds what the table would, without its header, so one
//! file can be shared, such as a look with allock's `[appearance]`.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

/// Follows the import of each top-level table of `file`, a relative path
/// from `folder`. An import is followed one level only. Returns every file
/// imported, those that cannot be read among them: they are watched for when
/// they come.
pub fn follow(file: &mut Table, folder: &Path, problems: &mut Vec<String>) -> Vec<PathBuf> {
    let mut imported = Vec::new();

    for (name, value) in file.iter_mut() {
        let Value::Table(table) = value else {
            continue;
        };
        let Some(import) = table.remove("import") else {
            continue;
        };
        let Some(import) = import.as_str() else {
            problems.push(format!("{name}.import: expected a path"));
            continue;
        };
        let path = folder.join(expand_home(import));
        let read = std::fs::read_to_string(&path)
            .map_err(|err| err.to_string())
            .and_then(|text| {
                toml::from_str::<Table>(&text).map_err(|err| err.message().to_string())
            });

        match read {
            Ok(mut base) => {
                if base.remove("import").is_some() {
                    problems.push(format!(
                        "{name}.import: {} imports a file in turn, not followed",
                        path.display()
                    ));
                }
                overlay(&mut base, std::mem::take(table));
                *table = base;
            }
            Err(err) => problems.push(format!(
                "{name}.import: {}: {err}; the table is read as written",
                path.display()
            )),
        }

        imported.push(path);
    }

    imported
}

/// `~/` as the home folder.
fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

/// `over`'s keys written onto `base`, table into table.
fn overlay(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(over)) => overlay(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Folder;

    /// `text`, read in `folder` with its imports followed.
    fn follow_in(folder: &Folder, text: &str) -> (Table, Vec<PathBuf>, Vec<String>) {
        let mut file: Table = toml::from_str(text).unwrap();
        let mut problems = Vec::new();
        let imported = follow(&mut file, folder, &mut problems);

        (file, imported, problems)
    }

    #[test]
    fn a_table_is_read_from_its_file_under_the_keys_set_beside_it() {
        let folder = Folder::new("import-overlay");
        folder.write(
            "look.toml",
            "theme = \"nord\"\nmaterial = \"solid\"\n[fonts]\nsans = \"Inter\"\nmono = \"Iosevka\"\n",
        );

        let (file, imported, problems) = follow_in(
            &folder,
            "[appearance]\nimport = \"look.toml\"\nmaterial = \"glass\"\n[appearance.fonts]\nmono = \"JetBrains Mono\"\n",
        );

        assert!(problems.is_empty());
        assert_eq!(imported, [folder.join("look.toml")]);
        assert_eq!(
            file["appearance"],
            toml::from_str::<Value>(
                "theme = \"nord\"\nmaterial = \"glass\"\n[fonts]\nsans = \"Inter\"\nmono = \"JetBrains Mono\"\n"
            )
            .unwrap()
        );
    }

    #[test]
    fn each_table_imports_its_own_file() {
        let folder = Folder::new("import-tables");
        folder
            .write("zones.toml", "[clock]\nitems = [\"clock\"]\n")
            .write("tempo.toml", "format = \"%H:%M\"\n");

        let (file, imported, problems) = follow_in(
            &folder,
            "[zone]\nimport = \"zones.toml\"\n[tempo]\nimport = \"tempo.toml\"\n[bar.main]\ncenter = [\"clock\"]\n",
        );

        assert!(problems.is_empty());
        assert_eq!(imported.len(), 2);
        assert_eq!(file["zone"]["clock"]["items"][0].as_str(), Some("clock"));
        assert_eq!(file["tempo"]["format"].as_str(), Some("%H:%M"));
        assert!(file["bar"]["main"].get("import").is_none());
    }

    #[test]
    fn a_file_that_cannot_be_read_leaves_the_table_as_written() {
        let folder = Folder::new("import-missing");
        folder.write("broken.toml", "theme = ");

        let (file, imported, problems) = follow_in(
            &folder,
            "[appearance]\nimport = \"nowhere.toml\"\ntheme = \"dracula\"\n[tempo]\nimport = \"broken.toml\"\n",
        );

        assert_eq!(file["appearance"]["theme"].as_str(), Some("dracula"));
        assert!(file["appearance"].get("import").is_none());
        assert_eq!(
            imported,
            [folder.join("nowhere.toml"), folder.join("broken.toml")]
        );
        assert_eq!(problems.len(), 2);
        assert!(problems[0].starts_with("appearance.import: "));
        assert!(problems[0].ends_with("; the table is read as written"));
        assert!(problems[1].starts_with("tempo.import: "));
    }

    #[test]
    fn an_import_is_followed_one_level_only() {
        let folder = Folder::new("import-nested");
        folder
            .write("look.toml", "import = \"other.toml\"\ntheme = \"nord\"\n")
            .write("other.toml", "material = \"glass\"\n");

        let (file, imported, problems) = follow_in(
            &folder,
            "[appearance]\nimport = \"look.toml\"\n[zone]\nimport = 3\n",
        );

        assert_eq!(
            file["appearance"],
            toml::from_str::<Value>("theme = \"nord\"").unwrap()
        );
        assert_eq!(imported, [folder.join("look.toml")]);
        assert_eq!(
            problems,
            [
                format!(
                    "appearance.import: {} imports a file in turn, not followed",
                    folder.join("look.toml").display()
                ),
                "zone.import: expected a path".to_string(),
            ]
        );
    }

    #[test]
    fn a_path_from_home_is_expanded() {
        let home = std::env::home_dir().unwrap();

        assert_eq!(expand_home("~/look.toml"), home.join("look.toml"));
        assert_eq!(
            expand_home("/etc/look.toml"),
            PathBuf::from("/etc/look.toml")
        );
        assert_eq!(expand_home("look.toml"), PathBuf::from("look.toml"));
    }
}
