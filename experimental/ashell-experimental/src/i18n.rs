use std::collections::HashMap;

use ashell_i18n::{Catalogs, Localizer, init_localizer, use_localizer};

const CATALOGS: Catalogs = &[
    (
        "en-US",
        include_str!("../i18n/en-US/ashell_experimental.ftl"),
    ),
    (
        "fr-FR",
        include_str!("../i18n/fr-FR/ashell_experimental.ftl"),
    ),
    (
        "de-DE",
        include_str!("../i18n/de-DE/ashell_experimental.ftl"),
    ),
    (
        "ru-RU",
        include_str!("../i18n/ru-RU/ashell_experimental.ftl"),
    ),
    (
        "uk-UA",
        include_str!("../i18n/uk-UA/ashell_experimental.ftl"),
    ),
];

/// The language and the formats from the environment, for the thread that
/// calls it.
pub fn init() {
    set(Localizer::resolve(None, None, CATALOGS));
}

/// `language` and the formats of `region`, whatever the environment says.
#[cfg(test)]
pub fn init_for(language: &str, region: &str) {
    set(Localizer::resolve(Some(language), Some(region), CATALOGS));
}

/// Arguments as they are: Fluent's isolating marks may draw as boxes.
fn set(localizer: Localizer) {
    localizer.loader().set_use_isolating(false);
    init_localizer(localizer);
}

/// styletto's words, from the same catalogs, for what is built from here
/// on.
pub fn provide_to_styletto() {
    styletto::provide_translations(|key, args| {
        let args: HashMap<&str, &str> = args.iter().copied().collect();

        use_localizer(|l| l.loader().has(key).then(|| l.loader().get_args(key, args)))
    });
}

#[cfg(test)]
mod tests {
    use super::CATALOGS;

    #[test]
    fn every_catalog_has_styletto_s_words() {
        for (language, catalog) in CATALOGS {
            for (key, _) in styletto::words::KEYS {
                assert!(
                    catalog
                        .lines()
                        .any(|line| line.starts_with(&format!("{key} = "))),
                    "{language} lacks {key}"
                );
            }
        }
    }
}
