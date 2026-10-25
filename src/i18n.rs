use ashell_i18n::Catalogs;

pub const CATALOGS: Catalogs = &[
    ("en-US", include_str!("../i18n/en-US/ashell.ftl")),
    ("fr-FR", include_str!("../i18n/fr-FR/ashell.ftl")),
    ("de-DE", include_str!("../i18n/de-DE/ashell.ftl")),
    ("ru-RU", include_str!("../i18n/ru-RU/ashell.ftl")),
    ("uk-UA", include_str!("../i18n/uk-UA/ashell.ftl")),
];
