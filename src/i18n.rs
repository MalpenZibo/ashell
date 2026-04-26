use std::borrow::Cow;
use std::cell::RefCell;

use chrono::Locale;
use i18n_embed::{
    I18nAssets, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use log::warn;
use unic_langid::LanguageIdentifier;

const CATALOGS: &[(&str, &str)] = &[("en-US", include_str!("../i18n/en-US/ashell.ftl"))];

const FALLBACK_LANG: &str = "en-US";
const TRANSLATION_FILE: &str = "ashell.ftl";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
}

pub struct Localizer {
    pub chrono: Locale,
    loader: FluentLanguageLoader,
}

impl Default for Localizer {
    fn default() -> Self {
        // Cheap fallback for the thread_local seed — no env reads, no bundle
        // parse. `init_localizer` in `App::new` replaces it immediately with
        // the resolved value; this just keeps `t!()` callable before that
        // point and on any non-main thread that ever touches LOCALIZER.
        let loader = FluentLanguageLoader::new("ashell", en_us_langid());
        Self {
            chrono: Locale::en_US,
            loader,
        }
    }
}

impl Localizer {
    pub fn resolve(language: Option<&str>, region: Option<&str>) -> Self {
        let langid = resolve_language(language);
        let chrono = resolve_region(region);
        let loader = load_loader(&langid);
        Self { chrono, loader }
    }

    pub fn units(&self) -> UnitSystem {
        derive_units(self.chrono)
    }

    pub fn loader(&self) -> &FluentLanguageLoader {
        &self.loader
    }
}

thread_local! {
    pub(crate) static LOCALIZER: RefCell<Localizer> = RefCell::new(Localizer::default());
}

pub fn init_localizer(localizer: Localizer) {
    LOCALIZER.replace(localizer);
}

pub fn use_localizer<R, F: FnOnce(&Localizer) -> R>(f: F) -> R {
    LOCALIZER.with_borrow(f)
}

pub fn chrono_locale() -> Locale {
    use_localizer(|l| l.chrono)
}

pub fn unit_system() -> UnitSystem {
    use_localizer(|l| l.units())
}

#[macro_export]
macro_rules! t {
    ($($args:tt)*) => {
        $crate::i18n::use_localizer(|l| ::i18n_embed_fl::fl!(l.loader(), $($args)*))
    };
}

fn resolve_language(config: Option<&str>) -> LanguageIdentifier {
    env_chain(config, "LC_MESSAGES")
        .as_deref()
        .and_then(|s| normalize_to_bcp47(s).parse().ok())
        .unwrap_or_else(en_us_langid)
}

fn resolve_region(config: Option<&str>) -> Locale {
    env_chain(config, "LC_TIME")
        .as_deref()
        .and_then(chrono_locale_from_posix)
        .unwrap_or(Locale::en_US)
}

fn chrono_locale_from_posix(s: &str) -> Option<Locale> {
    let posix = normalize_to_posix(s);
    Locale::try_from(posix.as_str()).ok().or_else(|| {
        let (base, _) = posix.split_once('@')?;
        Locale::try_from(base).ok()
    })
}

fn env_chain(config: Option<&str>, category_var: &str) -> Option<String> {
    config
        .map(str::to_owned)
        .or_else(|| env_locale("LC_ALL"))
        .or_else(|| env_locale(category_var))
        .or_else(|| env_locale("LANG"))
}

fn derive_units(c: Locale) -> UnitSystem {
    match c {
        Locale::en_US => UnitSystem::Imperial,
        _ => UnitSystem::Metric,
    }
}

struct StaticCatalogs;

impl I18nAssets for StaticCatalogs {
    fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
        CATALOGS
            .iter()
            .find(|(lang, _)| expected_path(lang) == file_path)
            .map(|(_, src)| vec![Cow::Borrowed(src.as_bytes())])
            .unwrap_or_default()
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(CATALOGS.iter().map(|(lang, _)| expected_path(lang)))
    }
}

fn expected_path(lang: &str) -> String {
    format!("{lang}/{TRANSLATION_FILE}")
}

fn load_loader(langid: &LanguageIdentifier) -> FluentLanguageLoader {
    let loader = fluent_language_loader!();
    if let Err(e) = loader.load_languages(&StaticCatalogs, std::slice::from_ref(langid)) {
        warn!("i18n: failed to load language {langid}: {e}; using fallback");
        if let Err(e) = loader.load_fallback_language(&StaticCatalogs) {
            warn!("i18n: failed to load fallback language: {e}");
        }
    }
    loader
}

fn env_locale(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .filter(|s| !s.is_empty() && s != "C" && s != "POSIX")
}

fn normalize_to_bcp47(s: &str) -> String {
    let (body, variant) = split_variant(s);
    let (lang_region, _encoding) = split_encoding(body);
    let bcp = lang_region.replace('_', "-");

    let Some(script) = variant.and_then(script_subtag_for_variant) else {
        return bcp;
    };
    match bcp.split_once('-') {
        Some((lang, region)) => format!("{lang}-{script}-{region}"),
        None => format!("{bcp}-{script}"),
    }
}

fn normalize_to_posix(s: &str) -> String {
    let (body, variant) = split_variant(s);
    let (lang_region, _encoding) = split_encoding(body);
    let posix = lang_region.replace('-', "_");
    match variant {
        Some(v) => format!("{posix}@{v}"),
        None => posix,
    }
}

fn split_variant(s: &str) -> (&str, Option<&str>) {
    match s.split_once('@') {
        Some((body, variant)) => (body, Some(variant)),
        None => (s, None),
    }
}

fn split_encoding(s: &str) -> (&str, Option<&str>) {
    match s.split_once('.') {
        Some((body, enc)) => (body, Some(enc)),
        None => (s, None),
    }
}

fn script_subtag_for_variant(variant: &str) -> Option<&'static str> {
    if variant.eq_ignore_ascii_case("latin") {
        Some("Latn")
    } else if variant.eq_ignore_ascii_case("cyrillic") {
        Some("Cyrl")
    } else {
        None
    }
}

fn en_us_langid() -> LanguageIdentifier {
    FALLBACK_LANG.parse().expect("en-US is a valid langid")
}
