use ashell_i18n::{Catalogs, Localizer, init_localizer};

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
    init_localizer(Localizer::resolve(None, None, CATALOGS));
}
