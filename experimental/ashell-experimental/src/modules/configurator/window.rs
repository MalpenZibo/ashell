//! The config window, two pages: the bar, its layout and edge, and the
//! look. On the first start they are steps, the first under a greeting;
//! opened again they are the settings' tabs. Each change is saved at once
//! and shown on the bars.

use std::path::Path;

use ashell_i18n::t;
use guido::prelude::*;
use styletto::components::appearance::{font_picker, layout_picker, mode_picker, theme_picker};
use styletto::components::button::{Fill, Variant, button};
use styletto::components::choices::{choices, tabs};
use styletto::components::list::page_indicator;
use styletto::components::settings::setting_group;
use styletto::components::tile::tile;
use styletto::foundations::tokens::{radius, size};
use styletto::foundations::typography::{
    BODY_LARGE, BODY_MEDIUM, BODY_SMALL, DISPLAY_SMALL, TITLE_LARGE, Type,
};
use styletto::primitives::island;
use styletto::primitives::text::{heading, typed};
use styletto::theme::config::UiConfig;
use styletto::{Icon, centered_row, themed};

use super::Shown;
use super::files::{self, LAYOUTS, Start, Status};
use crate::config::{self, Config, Position};

const WIDTH: u32 = 1000;
const HEIGHT: u32 = 680;

/// The pages, in the order of the steps and of the tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Bar,
    Look,
}

const PAGES: [Page; 2] = [Page::Bar, Page::Look];

impl Page {
    fn name(self) -> String {
        match self {
            Page::Bar => t!("config-page-bar"),
            Page::Look => t!("config-page-look"),
        }
    }
}

/// An icon for each of [`LAYOUTS`].
const ICONS: [Icon; LAYOUTS.len()] = [Icon::Monitor, Icon::Minus, Icon::Cloud, Icon::Droplet];

/// The edges, as the config's: `None` when it has no main bar to move.
fn edges() -> Vec<(String, Option<Position>)> {
    vec![
        (t!("config-edge-top"), Some(Position::Top)),
        (t!("config-edge-bottom"), Some(Position::Bottom)),
    ]
}

/// The window, drawn in the look being chosen: a new palette repaints it,
/// a new density or shape makes it again, the page and what is chosen
/// kept.
pub fn spawn(
    config: Signal<Config>,
    shown: Shown,
    close: impl Fn() + Copy + 'static,
) -> SurfaceHandle {
    let surface = SurfaceConfig::new()
        .width(WIDTH)
        .height(HEIGHT)
        .layer(Layer::Overlay)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .namespace("ashell-experimental-config")
        .background_color(Color::TRANSPARENT);

    spawn_surface(surface, move || {
        crate::i18n::provide_to_styletto();
        let state = State::new(config);
        // The first start begins at the bar; the settings at the look, the
        // more often changed.
        let page = create_signal(match shown {
            Shown::Steps => Page::Bar,
            Shown::Settings => Page::Look,
        });
        let drawn = create_memo(move || state.look.with(|look| look.ui(None)));

        themed(drawn, move || match shown {
            Shown::Steps => steps(state, page, close),
            Shown::Settings => settings(state, page, close),
        })
        .width(fill())
        .height(fill())
    })
}

/// What every page edits: the look, and the config's file.
#[derive(Clone, Copy)]
struct State {
    look: RwSignal<UiConfig>,
    status: RwSignal<Status>,
    /// Why the last change was not saved; empty when it was.
    note: RwSignal<String>,
}

impl State {
    fn new(config: Signal<Config>) -> Self {
        let look = create_signal(config.with_untracked(|c| c.appearance.clone()));
        let state = Self {
            look,
            status: create_signal(Status {
                start: Start::Own,
                position: None,
            }),
            note: create_signal(String::new()),
        };

        // The file changed elsewhere, or read again after a save: the
        // controls follow it.
        create_effect(move || {
            let read = config.with(|c| c.appearance.clone());

            if look.with_untracked(|look| *look != read) {
                look.set(read);
            }
            state.refresh();
        });

        // A change made here is saved, and comes back as the config read
        // again.
        create_effect(move || {
            let chosen = look.get();

            if config.with_untracked(|c| c.appearance != chosen) {
                state.save(files::save_look);
            }
        });

        state
    }

    /// The layout lit: the config's, or the first while there is none.
    fn chosen(self) -> Option<usize> {
        match self.status.get().start {
            Start::Missing => Some(0),
            Start::Layout(index) => Some(index),
            Start::Own => None,
        }
    }

    fn choose(self, index: usize) {
        self.save(|path, look| files::choose(path, &LAYOUTS[index], look));
    }

    fn move_to(self, position: Position) {
        self.save(|path, look| files::save_position(path, position, look));
    }

    /// Writes the config with `save`, then reads back what it is.
    fn save(self, save: impl FnOnce(&Path, &UiConfig) -> Result<(), String>) {
        let Some(path) = config::path() else {
            return;
        };
        let result = save(&path, &self.look.get_untracked());

        self.note.set(result.err().unwrap_or_default());
        self.refresh();
    }

    fn refresh(self) {
        if let Some(path) = config::path() {
            self.status.set(files::status(&path));
        }
    }
}

/// The first start, a page at a time.
fn steps(state: State, page: RwSignal<Page>, close: impl Fn() + Copy + 'static) -> Container {
    let back = move || page.set(Page::Bar);
    // The bar lit is written, if it is not yet.
    let next = move || {
        if state.status.get_untracked().start == Start::Missing {
            state.choose(0);
        }
        page.set(Page::Look);
    };

    let content = match page.get() {
        Page::Bar => container()
            .width(fill())
            .height(fill())
            .layout(Flex::column().spacing(size::S6.px()))
            .child(intro(
                t!("config-welcome-title"),
                DISPLAY_SMALL,
                t!("config-welcome-about"),
                BODY_LARGE,
            ))
            .child(text(t!("config-start-from"), BODY_MEDIUM))
            .child(bar(state))
            .child(container().width(fill()).height(fill()))
            .child(footer(page, None::<fn()>, (t!("config-next"), next))),
        Page::Look => container()
            .width(fill())
            .height(fill())
            .layout(Flex::column().spacing(size::S4.px()))
            .child(intro(
                t!("config-look-title"),
                TITLE_LARGE,
                t!("config-look-about"),
                BODY_MEDIUM,
            ))
            .child(look(state))
            .child(footer(page, Some(back), (t!("config-done"), close))),
    };

    window()
        .child(
            container()
                .width(fill())
                .layout(Flex::row().main_alignment(MainAlignment::End))
                .child(close_button(close)),
        )
        .child(notes(state))
        .child(content)
}

/// Opened from the bar: the pages as tabs.
fn settings(state: State, page: RwSignal<Page>, close: impl Fn() + Copy + 'static) -> Container {
    let content = match page.get() {
        Page::Bar => bar(state),
        Page::Look => look(state),
    };

    window()
        .child(
            container()
                .width(fill())
                .layout(centered_row(size::S4.px()))
                .child(heading(t!("config-settings")))
                .child(
                    container().width(fill()).child(
                        tabs(PAGES.map(|each| (each.name(), each)).to_vec(), page)
                            .on_choose(move |each| page.set(each)),
                    ),
                )
                .child(close_button(close)),
        )
        .child(notes(state))
        .child(content)
}

/// A page's title and what it is about.
fn intro(title: String, title_look: Type, about: String, about_look: Type) -> Container {
    container()
        .width(fill())
        .layout(Flex::column().spacing(size::S2.px()))
        .child(typed(title, title_look.em()).color(styletto::look().eased(|c| c.text)))
        .child(text(about, about_look))
}

/// The island the window is.
fn window() -> Container {
    let padding = size::S6.px();

    island::border(island::paint(container()))
        .width(fill())
        .height(fill())
        .padding(padding)
        .corners(radius::shape(radius::around(radius::MEDIUM.px(), padding)))
        .layout(Flex::column().spacing(size::S4.px()))
}

fn close_button(close: impl Fn() + Copy + 'static) -> impl Widget {
    button().icon(Icon::X).fill(Fill::Ghost).on_click(close)
}

/// The buttons that go back and on, and between them which step it is at.
fn footer(
    page: RwSignal<Page>,
    back: Option<impl Fn() + Copy + 'static>,
    (label, on): (String, impl Fn() + Copy + 'static),
) -> Container {
    let side = |end| {
        container()
            .width(fill())
            .layout(Flex::row().main_alignment(end))
    };

    container()
        .width(fill())
        .layout(centered_row(size::S2.px()))
        .child(side(MainAlignment::Start).children(back.map(|back| {
            button()
                .label(t!("config-back"))
                .fill(Fill::Ghost)
                .on_click(back)
        })))
        .child(page_indicator(PAGES.len(), move || page.get() as usize))
        .child(
            side(MainAlignment::End)
                .child(button().label(label).variant(Variant::Accent).on_click(on)),
        )
}

/// Why the last change was not saved.
fn notes(state: State) -> Container {
    container().width(fill()).child(move || {
        let text = state.note.get();

        (!text.is_empty())
            .then(|| typed(text, BODY_SMALL).color(styletto::look().eased(|c| c.danger.text)))
    })
}

/// The bar page: a tile for each layout, the config's lit, two to a line,
/// and the edge. A config of the user's own is not replaced, and is moved
/// only when it has a main bar.
fn bar(state: State) -> Container {
    let own = move || state.status.get().start == Start::Own;
    let layout = move |index: usize| {
        let layout = &LAYOUTS[index];

        container().width(fill()).child(
            tile(ICONS[index], (layout.name)())
                .subtitle((layout.about)())
                .active(move || state.chosen() == Some(index))
                .on_toggle(move |on| {
                    if on && !own() {
                        state.choose(index);
                    }
                }),
        )
    };
    let tiles = container()
        .width(fill())
        .layout(Flex::column().spacing(size::S2.px()))
        .children((0..LAYOUTS.len()).step_by(2).map(move |first| {
            container()
                .width(fill())
                .layout(Flex::row().spacing(size::S2.px()))
                .children((first..LAYOUTS.len().min(first + 2)).map(layout))
        }))
        .child(move || own().then(|| text(t!("config-own"), BODY_SMALL)));
    let edges = choices(edges(), move || state.status.get().position)
        .enabled(move || state.status.get().position.is_some())
        .on_choose(move |edge: Option<Position>| {
            if let Some(edge) = edge {
                state.move_to(edge);
            }
        });

    container()
        .width(fill())
        .layout(Flex::column().spacing(size::S4.px()))
        .child(setting_group(t!("config-group-layout")).child(tiles))
        .child(setting_group(t!("config-group-edge")).child(edges))
}

/// The look page: the theme on the left; on the right the shapes and the
/// fonts. Each column scrolls on its own.
fn look(state: State) -> Container {
    let look = state.look;

    let theme = setting_group(t!("config-group-theme"))
        .child(mode_picker(look))
        .child(theme_picker(look));

    let right = container()
        .width(fill())
        .layout(Flex::column().spacing(size::S4.px()))
        .child(setting_group(t!("config-group-shapes")).child(layout_picker(look)))
        .child(setting_group(t!("config-group-fonts")).child(font_picker(look)));

    container()
        .width(fill())
        .height(fill())
        .layout(Flex::row().spacing(size::S6.px()))
        .child(column(theme))
        .child(column(right))
}

fn column(content: impl Widget + 'static) -> Container {
    container()
        .width(fill())
        .height(fill())
        .scroll(Scroll::vertical())
        .child(content)
}

/// Quiet text, wrapped to the width.
fn text(text: String, typeset: Type) -> Container {
    container()
        .width(fill())
        .child(typed(text, typeset).color(styletto::look().eased(|c| c.sub)))
}
