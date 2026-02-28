use freedesktop_icons::lookup;
use iced::widget::{image, svg};
use linicon_theme::get_icon_theme;
use log::debug;
use std::{
    collections::{BTreeSet, HashMap},
    env, fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

static ICON_CACHE: LazyLock<Mutex<HashMap<String, Option<XdgIcon>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SYSTEM_ICON_NAMES: LazyLock<BTreeSet<String>> = LazyLock::new(load_system_icon_names);
static SYSTEM_ICON_ENTRIES: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    SYSTEM_ICON_NAMES
        .iter()
        .map(|name| (name.clone(), normalize_icon_name(name)))
        .collect()
});

#[derive(Debug, Clone)]
pub enum XdgIcon {
    Image(image::Handle),
    Svg(svg::Handle),
    NerdFont(&'static str),
}

pub fn fallback_icon() -> XdgIcon {
    XdgIcon::NerdFont(crate::components::icons::StaticIcon::Point.get_str())
}

pub fn get_icon_from_name(icon_name: &str) -> Option<XdgIcon> {
    let mut cache = ICON_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(cached) = cache.get(icon_name) {
        return cached.clone();
    }
    let result = lookup_icon(icon_name);
    cache.insert(icon_name.to_string(), result.clone());
    result
}

fn lookup_icon(icon_name: &str) -> Option<XdgIcon> {
    if let Some(path) = find_icon_path(icon_name) {
        return icon_from_path(path);
    }

    if let Some(candidates) = similar_icon_names(icon_name) {
        for candidate in candidates {
            if let Some(path) = find_icon_path(&candidate) {
                return icon_from_path(path);
            }
        }
    }

    if let Some(prefix_candidate) = prefix_match_icon(icon_name)
        && let Some(path) = find_icon_path(&prefix_candidate)
    {
        return icon_from_path(path);
    }

    None
}

fn icon_from_path(path: PathBuf) -> Option<XdgIcon> {
    if path.extension().is_some_and(|ext| ext == "svg") {
        debug!("svg icon found. Path: {path:?}");

        Some(XdgIcon::Svg(svg::Handle::from_path(path)))
    } else {
        debug!("raster icon found. Path: {path:?}");

        Some(XdgIcon::Image(image::Handle::from_path(path)))
    }
}

fn find_icon_path(icon_name: &str) -> Option<PathBuf> {
    let base_lookup = lookup(icon_name).with_cache();

    match get_icon_theme() {
        Some(theme) => base_lookup.with_theme(&theme).find().or_else(|| {
            let fallback_lookup = lookup(icon_name).with_cache();
            fallback_lookup.find()
        }),
        None => base_lookup.find(),
    }
}

fn similar_icon_names(icon_name: &str) -> Option<Vec<String>> {
    if SYSTEM_ICON_NAMES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    let mut matches = Vec::new();

    for candidate in SYSTEM_ICON_NAMES.iter() {
        let candidate_normalized = normalize_icon_name(candidate);

        if candidate_normalized == normalized {
            continue;
        }

        if candidate_normalized.contains(&normalized)
            || normalized.contains(&candidate_normalized)
            || candidate_normalized.contains(&normalized.replace('-', ""))
        {
            matches.push(candidate.clone());
            if matches.len() >= 5 {
                break;
            }
        }
    }

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

fn normalize_icon_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn prefix_match_icon(icon_name: &str) -> Option<String> {
    if SYSTEM_ICON_ENTRIES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    let mut candidates: Vec<&(String, String)> = SYSTEM_ICON_ENTRIES.iter().collect();
    let chars: Vec<char> = normalized.chars().collect();

    for (idx, ch) in chars.iter().enumerate() {
        candidates.retain(|(_, name)| name.chars().nth(idx) == Some(*ch));

        if candidates.len() == 1 {
            return Some(candidates[0].0.clone());
        }

        if candidates.is_empty() {
            break;
        }
    }

    candidates.first().map(|(name, _)| name.clone())
}

fn load_system_icon_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for dir in icon_directories() {
        if !dir.is_dir() {
            continue;
        }

        collect_icon_names_recursive(&dir, &mut names);
    }

    names
}

fn collect_icon_names_recursive(dir: &Path, names: &mut BTreeSet<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    collect_icon_names_recursive(&path, names);
                } else if file_type.is_file()
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    names.insert(stem.to_string());
                }
            }
        }
    }
}

fn icon_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        let base = PathBuf::from(data_home);
        dirs.push(base.join("icons"));
        dirs.push(base.join("pixmaps"));
    }

    if let Ok(home) = env::var("HOME") {
        let base = PathBuf::from(home);
        dirs.push(base.join(".local/share/icons"));
        dirs.push(base.join(".local/share/pixmaps"));
    }

    let data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':') {
        if dir.is_empty() {
            continue;
        }
        let base = PathBuf::from(dir);
        dirs.push(base.join("icons"));
        dirs.push(base.join("pixmaps"));
    }

    dirs.push(PathBuf::from("/usr/share/icons"));
    dirs.push(PathBuf::from("/usr/share/pixmaps"));

    dirs.sort();
    dirs.dedup();
    dirs
}
