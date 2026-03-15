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
static DESKTOP_ICON_INDEX: LazyLock<HashMap<String, String>> =
    LazyLock::new(build_desktop_icon_index);

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
    if icon_name.is_empty() {
        return None;
    }
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
        debug!("icon '{icon_name}': direct match at {path:?}");
        return icon_from_path(path);
    }
    debug!("icon '{icon_name}': no direct match");

    if let Some(path) = find_similar_icon(icon_name) {
        debug!("icon '{icon_name}': similar match at {path:?}");
        return icon_from_path(path);
    }
    debug!("icon '{icon_name}': no similar match");

    if let Some(path) = find_desktop_icon(icon_name) {
        debug!("icon '{icon_name}': desktop index match at {path:?}");
        return icon_from_path(path);
    }
    debug!("icon '{icon_name}': no desktop index match");

    if let Some(prefix_candidate) = prefix_match_icon(icon_name)
        && let Some(path) = find_icon_path(&prefix_candidate)
    {
        debug!("icon '{icon_name}': prefix match '{prefix_candidate}' at {path:?}");
        return icon_from_path(path);
    }
    debug!("icon '{icon_name}': no prefix match — unresolved");

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

fn find_similar_icon(icon_name: &str) -> Option<PathBuf> {
    if SYSTEM_ICON_NAMES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    if normalized.is_empty() {
        return None;
    }
    let normalized_no_dash = normalized.replace('-', "");

    for candidate in SYSTEM_ICON_NAMES.iter() {
        let candidate_normalized = normalize_icon_name(candidate);

        if candidate_normalized == normalized {
            continue;
        }

        if candidate_normalized.contains(&normalized)
            || normalized.contains(&candidate_normalized)
            || candidate_normalized.contains(&normalized_no_dash)
        {
            debug!("icon '{icon_name}': similar candidate '{candidate}'");
            if let Some(path) = find_icon_path(candidate) {
                return Some(path);
            }
        }
    }

    None
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

fn find_desktop_icon(icon_name: &str) -> Option<PathBuf> {
    let normalized = normalize_icon_name(icon_name);
    let Some(icon_value) = DESKTOP_ICON_INDEX.get(&normalized) else {
        debug!("icon '{icon_name}': normalized '{normalized}' not in desktop index");
        return None;
    };
    debug!("icon '{icon_name}': desktop index '{normalized}' → '{icon_value}'");

    if icon_value.starts_with('/') {
        let path = PathBuf::from(icon_value);
        if path.exists() {
            Some(path)
        } else {
            debug!("icon '{icon_name}': absolute path '{icon_value}' does not exist");
            None
        }
    } else {
        find_icon_path(icon_value)
    }
}

fn build_desktop_icon_index() -> HashMap<String, String> {
    let mut map = HashMap::new();

    for dir in desktop_application_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                parse_desktop_file(&path, &mut map);
            }
        }
    }

    map
}

fn parse_desktop_file(path: &Path, map: &mut HashMap<String, String>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    let mut icon_value: Option<String> = None;
    let mut wm_class: Option<String> = None;

    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("Icon=") {
            icon_value = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("StartupWMClass=") {
            wm_class = Some(val.trim().to_string());
        }
    }

    let Some(icon) = icon_value else {
        return;
    };

    if let Some(wm) = wm_class {
        map.entry(normalize_icon_name(&wm))
            .or_insert_with(|| icon.clone());
    }
    // Index the full stem, plus each dot-suffix for reverse-DNS names like
    // "com.ultimaker.cura" → also insert "ultimaker.cura" and "cura".
    let parts: Vec<&str> = stem.split('.').collect();
    for start in 0..parts.len() {
        let key = normalize_icon_name(&parts[start..].join("."));
        map.entry(key).or_insert_with(|| icon.clone());
    }
}

fn desktop_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(data_home) = env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    }

    if let Ok(home) = env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for dir in data_dirs.split(':') {
        if !dir.is_empty() {
            dirs.push(PathBuf::from(dir).join("applications"));
        }
    }

    dirs.push(PathBuf::from("/usr/local/share/applications"));
    dirs.push(PathBuf::from("/usr/share/applications"));

    dirs.sort();
    dirs.dedup();
    dirs
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
