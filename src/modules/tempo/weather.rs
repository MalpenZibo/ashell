use ashell_services::weather::{
    TemperatureUnit, WeatherLocation as Location, WeatherQuery, WindSpeedUnit as WindUnit,
};
use iced::{
    Border, Degrees, Element, Length, Padding, Rotation, Theme,
    alignment::{Horizontal, Vertical},
    core::svg::Handle,
    widget::{Column, MouseArea, Row, Svg, column, container, row, svg, text},
};

use crate::{
    components::scrollable,
    config::{WeatherLocation, WindSpeedUnit},
    theme::{Paint, use_theme},
};
use ashell_i18n::{UnitSystem, chrono_locale, language_subtag, t, unit_system};

use super::{Message, Tempo};

impl Tempo {
    pub(super) fn weather_query(&self) -> Option<WeatherQuery> {
        let location = match self.config.weather_location.clone()? {
            WeatherLocation::Current => Location::Current,
            WeatherLocation::City(city) => Location::City(city),
            WeatherLocation::Coordinates(lat, lon) => Location::Coordinates(lat, lon),
        };

        Some(WeatherQuery {
            location,
            temperature_unit: match unit_system() {
                UnitSystem::Metric => TemperatureUnit::Celsius,
                UnitSystem::Imperial => TemperatureUnit::Fahrenheit,
            },
            wind_speed_unit: match self.config.resolved_wind_speed_unit() {
                WindSpeedUnit::Kmh => WindUnit::Kmh,
                WindSpeedUnit::Mph => WindUnit::Mph,
                WindSpeedUnit::Ms => WindUnit::Ms,
            },
            language: language_subtag(),
        })
    }

    pub(super) fn weather<'a>(&'a self) -> Option<Element<'a, Message>> {
        let (space, font_size, radius) = use_theme(|t| (t.space, t.font_size, t.radius));
        let locale = chrono_locale();
        let units = unit_system();
        let temp = units.temperature_symbol();
        let wind = self.config.resolved_wind_speed_unit().symbol();
        let location_visible = self.location_visible;
        self.weather_data.as_ref().map(|data| {
            let location = &data.place;
            let inner_element: Element<'a, Message> = if location_visible {
                let display_time = self.time_str(
                    &self.weather_time_format(),
                    self.current_timezone_index,
                    Some(data.current.time),
                );
                text(format!(
                    "{}{} - {}",
                    location.city,
                    if location.region.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", location.region)
                    },
                    display_time
                ))
                .size(font_size.sm)
                .into()
            } else {
                container(text("•••••").size(font_size.sm))
                    .style(move |theme: &Theme| container::Style {
                        background: Some(
                            Paint::surface(theme, theme.extended_palette().background.strong.color)
                                .into(),
                        ),
                        border: Border::default().rounded(radius.sm),
                        ..Default::default()
                    })
                    .into()
            };

            let location_element: Element<'a, Message> = MouseArea::new(inner_element)
                .on_press(Message::ToggleLocationVisibility)
                .into();

            column!(
                container(
                    row!(
                        weather_icon(data.current.code, data.current.is_day)
                            .height(font_size.xxl)
                            .width(Length::Shrink),
                        column!(
                            location_element,
                            text(weather_description(data.current.code)),
                            row!(
                                text(format!("{}{temp}", data.current.temperature)),
                                text(t!(
                                    "tempo-feels-like",
                                    value = data.current.feels_like.round(),
                                    unit = temp,
                                ))
                                .size(font_size.sm)
                            )
                            .align_y(Vertical::Bottom)
                            .spacing(space.sm),
                        )
                        .width(Length::FillPortion(2))
                        .spacing(space.xs),
                        column!(
                            row!(
                                svg(Handle::from_memory(include_bytes!(
                                    "../../../assets/weather_icon/drop.svg"
                                )))
                                .width(Length::Shrink)
                                .height(font_size.xl),
                                column!(
                                    text(t!("tempo-humidity"))
                                        .size(font_size.xs)
                                        .align_x(Horizontal::Right)
                                        .width(Length::Fill),
                                    text(format!("{}%", data.current.humidity))
                                        .align_x(Horizontal::Right)
                                        .size(font_size.xs)
                                        .width(Length::Fill),
                                )
                                .spacing(space.xxs)
                            )
                            .align_y(Vertical::Center)
                            .spacing(space.sm),
                            row!(
                                svg(Handle::from_memory(include_bytes!(
                                    "../../../assets/weather_icon/wind.svg"
                                )))
                                .height(font_size.xl)
                                .width(Length::Shrink)
                                .rotation(Rotation::Floating(
                                    Degrees(data.current.wind_direction as f32 + 90.).into()
                                )),
                                column!(
                                    text(t!("tempo-wind"))
                                        .size(font_size.xs)
                                        .align_x(Horizontal::Right)
                                        .width(Length::Fill),
                                    text(format!("{} {wind}", data.current.wind_speed.round()))
                                        .align_x(Horizontal::Right)
                                        .size(font_size.xs)
                                        .width(Length::Fill),
                                )
                                .spacing(space.xxs)
                            )
                            .align_y(Vertical::Center)
                            .spacing(space.sm),
                        )
                        .width(Length::Fill)
                        .spacing(space.xs),
                    )
                    .spacing(space.lg)
                    .align_y(Vertical::Center)
                    .width(Length::Fill),
                )
                .padding(space.md)
                .style(crate::theme::card_style(radius.lg)),
                container(
                    scrollable(
                        Row::with_children({
                            data.hours
                                .iter()
                                .filter(|hour| hour.time > data.current.time)
                                .take(23)
                                .map(|hour| {
                                    let hour_time = &hour.time;
                                    // Entries fall on the hour in UTC, so in a :30 or
                                    // :45 zone they need the minutes.
                                    let on_the_hour = self.time_str(
                                        "%M",
                                        self.current_timezone_index,
                                        Some(*hour_time),
                                    ) == "00";
                                    let display_time = self.time_str(
                                        &if on_the_hour {
                                            self.weather_hour_format()
                                        } else {
                                            self.weather_time_format()
                                        },
                                        self.current_timezone_index,
                                        Some(*hour_time),
                                    );
                                    column!(
                                        text(format!("{}{temp}", hour.temperature.round())),
                                        weather_icon(hour.code, hour.is_day)
                                            .height(font_size.lg)
                                            .width(Length::Shrink),
                                        text(display_time).size(font_size.sm)
                                    )
                                    .spacing(space.xs)
                                    .align_x(Horizontal::Center)
                                    .into()
                                })
                                .collect::<Vec<_>>()
                        })
                        .spacing(space.sm)
                        .padding(Padding::default().bottom(space.sm))
                    )
                    .horizontal()
                )
                .padding(space.sm)
                .style(crate::theme::card_style(radius.lg)),
                Column::with_children(data.days.iter().skip(1).enumerate().map(|(index, day)| {
                    let last_index = data.days.len().saturating_sub(2);
                    container(
                        row!(
                            text(day.date.format_localized("%a, %d %b", locale).to_string())
                                .width(Length::FillPortion(5)),
                            row!(
                                weather_icon(day.code, true)
                                    .height(font_size.lg)
                                    .width(Length::Fixed(font_size.lg)),
                                text(format!(
                                    "{}{temp}/{}{temp}",
                                    day.temperature_max.round(),
                                    day.temperature_min.round()
                                ))
                            )
                            .spacing(space.xxs)
                            .align_y(Vertical::Center)
                            .width(Length::FillPortion(5)),
                            column!(
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../../assets/weather_icon/drop.svg"
                                    )))
                                    .height(font_size.sm)
                                    .width(Length::Fixed(font_size.sm)),
                                    text(format!("{}%", day.humidity)).size(font_size.sm)
                                )
                                .spacing(space.xxs)
                                .align_y(Vertical::Center),
                                row!(
                                    svg(Handle::from_memory(include_bytes!(
                                        "../../../assets/weather_icon/wind.svg"
                                    )))
                                    .height(font_size.sm)
                                    .width(Length::Fixed(font_size.sm))
                                    .rotation(
                                        Rotation::Floating(
                                            Degrees(day.wind_direction as f32 + 90.).into()
                                        )
                                    ),
                                    text(format!("{} {wind}", day.wind_speed_max.round()))
                                        .size(font_size.sm)
                                )
                                .spacing(space.xxs)
                                .align_y(Vertical::Center)
                            )
                            .width(Length::FillPortion(3))
                        )
                        .spacing(space.sm)
                        .align_y(Vertical::Center),
                    )
                    .padding(space.sm)
                    .style(crate::theme::card_style(iced::border::Radius {
                        top_left: if index == 0 { radius.lg } else { radius.sm },
                        top_right: if index == 0 { radius.lg } else { radius.sm },
                        bottom_right: if index == last_index {
                            radius.lg
                        } else {
                            radius.sm
                        },
                        bottom_left: if index == last_index {
                            radius.lg
                        } else {
                            radius.sm
                        },
                    }))
                    .into()
                }))
                .spacing(space.xxs)
            )
            .spacing(space.sm)
            .into()
        })
    }
}

fn weather_icon_bytes(code: u32, is_day: bool) -> &'static [u8] {
    match (code, is_day) {
        (0, true) => include_bytes!("../../../assets/weather_icon/clear-day.svg"),
        (0, false) => include_bytes!("../../../assets/weather_icon/clear-night.svg"),
        (1, true) => include_bytes!("../../../assets/weather_icon/cloudy-1-day.svg"),
        (1, false) => include_bytes!("../../../assets/weather_icon/cloudy-1-night.svg"),
        (2, true) => include_bytes!("../../../assets/weather_icon/cloudy-3-day.svg"),
        (2, false) => include_bytes!("../../../assets/weather_icon/cloudy-3-night.svg"),
        (3, _) => include_bytes!("../../../assets/weather_icon/cloudy.svg"),
        (45, _) | (48, _) => include_bytes!("../../../assets/weather_icon/fog.svg"),
        (51, true) => include_bytes!("../../../assets/weather_icon/rainy-1-day.svg"),
        (51, false) => include_bytes!("../../../assets/weather_icon/rainy-1-night.svg"),
        (53, true) | (56, true) => include_bytes!("../../../assets/weather_icon/rainy-2-day.svg"),
        (53, false) | (56, false) => {
            include_bytes!("../../../assets/weather_icon/rainy-2-night.svg")
        }
        (55, true) | (57, true) => include_bytes!("../../../assets/weather_icon/rainy-3-day.svg"),
        (55, false) | (57, false) => {
            include_bytes!("../../../assets/weather_icon/rainy-3-night.svg")
        }
        (61, _) => include_bytes!("../../../assets/weather_icon/rainy-1.svg"),
        (63, _) | (66, _) => include_bytes!("../../../assets/weather_icon/rainy-2.svg"),
        (65, _) | (67, _) => include_bytes!("../../../assets/weather_icon/rainy-3.svg"),
        (71, _) | (77, _) => include_bytes!("../../../assets/weather_icon/snowy-1.svg"),
        (73, _) => include_bytes!("../../../assets/weather_icon/snowy-2.svg"),
        (75, _) => include_bytes!("../../../assets/weather_icon/snowy-3.svg"),
        (80, true) => include_bytes!("../../../assets/weather_icon/showers-rainy-1-day.svg"),
        (80, false) => include_bytes!("../../../assets/weather_icon/showers-rainy-1-night.svg"),
        (81, true) => include_bytes!("../../../assets/weather_icon/showers-rainy-2-day.svg"),
        (81, false) => include_bytes!("../../../assets/weather_icon/showers-rainy-2-night.svg"),
        (82, true) => include_bytes!("../../../assets/weather_icon/showers-rainy-3-day.svg"),
        (82, false) => include_bytes!("../../../assets/weather_icon/showers-rainy-3-night.svg"),
        (85, true) => include_bytes!("../../../assets/weather_icon/snowy-2-day.svg"),
        (85, false) => include_bytes!("../../../assets/weather_icon/snowy-2-night.svg"),
        (86, true) => include_bytes!("../../../assets/weather_icon/snowy-3-day.svg"),
        (86, false) => include_bytes!("../../../assets/weather_icon/snowy-3-night.svg"),
        (95, _) => include_bytes!("../../../assets/weather_icon/isolated-thunderstorms.svg"),
        (96, _) => include_bytes!("../../../assets/weather_icon/scattered-thunderstorms.svg"),
        (99, _) => include_bytes!("../../../assets/weather_icon/severe-thunderstorm.svg"),
        _ => include_bytes!("../../../assets/weather_icon/unknown.svg"),
    }
}

pub fn weather_icon<'a>(code: u32, is_day: bool) -> Svg<'a> {
    svg(Handle::from_memory(weather_icon_bytes(code, is_day)))
}

pub fn weather_description(code: u32) -> String {
    match code {
        0 => t!("weather-clear-sky"),
        1 => t!("weather-mainly-clear"),
        2 => t!("weather-partly-cloudy"),
        3 => t!("weather-overcast"),
        45 => t!("weather-fog"),
        48 => t!("weather-fog-rime"),
        51 => t!("weather-drizzle-light"),
        53 => t!("weather-drizzle-moderate"),
        55 => t!("weather-drizzle-dense"),
        56 => t!("weather-drizzle-freezing-light"),
        57 => t!("weather-drizzle-freezing-dense"),
        61 => t!("weather-rain-slight"),
        63 => t!("weather-rain-moderate"),
        65 => t!("weather-rain-heavy"),
        66 => t!("weather-rain-freezing-light"),
        67 => t!("weather-rain-freezing-heavy"),
        71 => t!("weather-snow-slight"),
        73 => t!("weather-snow-moderate"),
        75 => t!("weather-snow-heavy"),
        77 => t!("weather-snow-grains"),
        80 => t!("weather-rain-showers-slight"),
        81 => t!("weather-rain-showers-moderate"),
        82 => t!("weather-rain-showers-violent"),
        85 => t!("weather-snow-showers-slight"),
        86 => t!("weather-snow-showers-heavy"),
        95 => t!("weather-thunderstorm"),
        96 => t!("weather-thunderstorm-hail-slight"),
        99 => t!("weather-thunderstorm-hail-heavy"),
        _ => t!("weather-unknown"),
    }
}
