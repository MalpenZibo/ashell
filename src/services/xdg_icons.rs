use freedesktop_icons::lookup;
use iced::widget::{image, svg};
use linicon_theme::get_icon_theme;
use log::debug;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use std::sync::RwLock;

const MAX_SIMILAR_ICON_CANDIDATES: usize = 5;

static ICON_CACHE: LazyLock<RwLock<HashMap<String, Option<XdgIcon>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SYSTEM_ICON_NAMES: LazyLock<BTreeSet<OsString>> = LazyLock::new(load_system_icon_names);
static SYSTEM_ICON_ENTRIES: LazyLock<Vec<(Cow<'static, str>, Cow<'static, str>)>> =
    LazyLock::new(|| {
        SYSTEM_ICON_NAMES
            .iter()
            .filter_map(|name| {
                let name_str = name.to_str()?;
                let normalized = normalize_icon_name(name_str);
                let normalized_cow = if normalized.as_ref() == name_str {
                    Cow::Borrowed(name_str)
                } else {
                    Cow::Owned(normalized.into_owned())
                };
                Some((Cow::Owned(name_str.to_string()), normalized_cow))
            })
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

    let cache = match ICON_CACHE.read() {
        Ok(c) => c,
        Err(_) => {
            return lookup_icon(icon_name);
        }
    };

    if let Some(cached) = cache.get(icon_name) {
        return cached.clone();
    }
    drop(cache); // Release read lock before write

    let mut cache = match ICON_CACHE.write() {
        Ok(c) => c,
        Err(_) => {
            return lookup_icon(icon_name);
        }
    };

    // Double-check after acquiring write lock (another thread may have populated it)
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

    if let Some(candidates) = find_similar_icon(icon_name) {
        for candidate in candidates {
            debug!("icon '{icon_name}': similar candidate '{candidate}'");
            if let Some(path) = find_icon_path(&candidate) {
                return icon_from_path(path);
            }
        }
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

fn find_similar_icon(icon_name: &str) -> Option<Vec<Cow<'static, str>>> {
    if SYSTEM_ICON_ENTRIES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);
    let normalized_no_separators = strip_icon_separators(normalized.as_ref());
    let mut matches: Vec<Cow<'static, str>> = Vec::with_capacity(MAX_SIMILAR_ICON_CANDIDATES);

    for (candidate_name, candidate_normalized) in SYSTEM_ICON_ENTRIES.iter() {
        if candidate_normalized.as_ref() == normalized.as_ref() {
            continue;
        }

        if candidate_normalized.as_ref().contains(normalized.as_ref())
            || normalized.as_ref().contains(candidate_normalized.as_ref())
            || candidate_normalized
                .as_ref()
                .contains(normalized_no_separators.as_ref())
        {
            matches.push(Cow::Borrowed(candidate_name.as_ref()));
            if matches.len() >= MAX_SIMILAR_ICON_CANDIDATES {
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

fn normalize_icon_name(name: &str) -> Cow<'_, str> {
    if name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Cow::Borrowed(name);
    }

    Cow::Owned(
        name.to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect(),
    )
}

fn strip_icon_separators(name: &str) -> Cow<'_, str> {
    if name.bytes().all(|byte| byte != b'-' && byte != b'_') {
        return Cow::Borrowed(name);
    }

    Cow::Owned(name.chars().filter(|ch| *ch != '-' && *ch != '_').collect())
}

fn prefix_match_icon(icon_name: &str) -> Option<Cow<'static, str>> {
    if SYSTEM_ICON_ENTRIES.is_empty() {
        return None;
    }

    let normalized = normalize_icon_name(icon_name);

    if let Some(exact) = SYSTEM_ICON_ENTRIES
        .iter()
        .find(|(_, norm)| norm.as_ref() == normalized.as_ref())
    {
        return Some(Cow::Borrowed(exact.0.as_ref()));
    }

    let mut candidates: Vec<_> = SYSTEM_ICON_ENTRIES.iter().collect();
    for (idx, ch) in normalized.chars().enumerate() {
        candidates.retain(|(_, name)| name.chars().nth(idx) == Some(ch));

        if candidates.len() == 1 {
            return Some(Cow::Borrowed(candidates[0].0.as_ref()));
        }

        if candidates.is_empty() {
            break;
        }
    }

    candidates
        .first()
        .map(|(name, _)| Cow::Borrowed(name.as_ref()))
}

fn find_desktop_icon(icon_name: &str) -> Option<PathBuf> {
    let normalized = normalize_icon_name(icon_name);
    let Some(icon_value) = DESKTOP_ICON_INDEX.get(normalized.as_ref()) else {
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

const MAX_DESKTOP_FILE_SIZE: u64 = 64 * 1024;

fn parse_desktop_file(path: &Path, map: &mut HashMap<String, String>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };

    if metadata.len() > MAX_DESKTOP_FILE_SIZE {
        debug!("desktop file too large: {}", path.display());
        return;
    }

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
        let key = normalize_icon_name(&wm).into_owned();
        map.entry(key).or_insert_with(|| icon.clone());
    }
    let parts: Vec<&str> = stem.split('.').collect();
    for start in 0..parts.len() {
        let key = normalize_icon_name(&parts[start..].join(".")).into_owned();
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

fn load_system_icon_names() -> BTreeSet<OsString> {
    let mut names = BTreeSet::new();

    for dir in icon_directories() {
        if !dir.is_dir() {
            continue;
        }

        collect_icon_names_recursive(&dir, &mut names);
    }

    names
}

fn collect_icon_names_recursive(dir: &Path, names: &mut BTreeSet<OsString>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    collect_icon_names_recursive(&path, names);
                } else if file_type.is_file()
                    && let Some(stem) = path.file_stem()
                {
                    names.insert(stem.to_os_string());
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
