//! The items of a zone's rows, as their modules draw them, with their
//! gestures.

use std::cell::Cell;
use std::rc::Rc;

use guido::prelude::*;
use styletto::centered_row;
use styletto::components::button::{pill, pill_text};
use styletto::foundations::control::Size;
use styletto::foundations::tokens::size;

use crate::config::{Action, Item, Open};
use crate::modules::{ItemView, Modules};

/// How far a wheel or a finger scrolls for one step of `scroll_up` or
/// `scroll_down`: a notch of the wheel, as guido counts it.
const SCROLL_STEP: f32 = 40.0;

/// Opens what an item or a zone opens: a panel under the zone the item is
/// in, or the config window.
pub(super) type Opens = Rc<dyn Fn(Open)>;

/// The zone's resting row, its items drawn by their modules, or `None` while
/// no module draws any of them: the zone is then its placeholder.
pub(super) fn zone(
    items: &[Item],
    open: Open,
    modules: Modules,
    opens: Opens,
    click: impl Fn() + 'static,
) -> Option<Container> {
    let (items, drawn) = row(items, open, modules, opens);

    drawn.then(|| container().child(pill().on_click(click).children(items)))
}

/// A zone's details: its items, as [`row`] has them.
pub(super) fn details(items: &[Item], open: Open, modules: Modules, opens: Opens) -> Container {
    container()
        .layout(centered_row(size::S1.px()))
        .children(row(items, open, modules, opens).0)
}

/// The items the modules draw, and whether there is any. While there is
/// none, a placeholder for each item; once there is one, those no module
/// draws yet are left out, so a zone being built shows only what is done.
fn row(items: &[Item], open: Open, modules: Modules, opens: Opens) -> (Vec<Container>, bool) {
    let views: Vec<_> = items
        .iter()
        .map(|item| (item, modules.item(item)))
        .collect();
    let any_drawn = views.iter().any(|(_, view)| view.is_some());

    let row = views
        .into_iter()
        .filter_map(|(item, view)| match view {
            Some(view) => Some(drawn(item, view, open, opens.clone())),
            None => (!any_drawn).then(|| placeholder(item, open, opens.clone())),
        })
        .collect();

    (row, any_drawn)
}

/// An item no module draws yet: its name where a pill's text would be, but
/// no pill of its own, so the zone round it is the one thing hovered. A
/// click on it opens its own panel, as the item will.
fn placeholder(item: &Item, zone_open: Open, opens: Opens) -> Container {
    let size = Size::Sm;
    let view = container()
        .height(size.height())
        .padding([0.0, size.inline()])
        .layout(centered_row(size.gap()))
        .child(pill_text(item.id.name().to_string()));

    match item.open {
        Some(open) if open != zone_open && open != Open::Nothing => {
            view.on_click(move || opens(open))
        }
        _ => view,
    }
}

fn drawn(item: &Item, view: ItemView, zone_open: Open, opens: Opens) -> Container {
    let ItemView { view, shown, act } = view;

    let run = move |action: Action| {
        let act = act.clone();
        let opens = opens.clone();

        move || match &action {
            Action::Named(name) => act(name),
            Action::Open(open) => opens(*open),
            Action::Run(command) => run_command(command.clone()),
            Action::None => {}
        }
    };

    let mut view = view.visible(shown);

    match item.open {
        Some(open) if open == zone_open => {}
        Some(Open::Nothing) => view = view.on_click(|| {}),
        Some(open) => view = view.on_click(run(Action::Open(open))),
        None => {}
    }

    let gestures = &item.gestures;

    if let Some(action) = gestures.right_click.clone() {
        view = view.on_right_click(run(action));
    }

    if let Some(action) = gestures.middle_click.clone() {
        view = view.on_middle_click(run(action));
    }

    if gestures.scroll_up.is_some() || gestures.scroll_down.is_some() {
        let up = gestures.scroll_up.clone().map(&run);
        let down = gestures.scroll_down.clone().map(&run);
        let gathered = Cell::new(0.0f32);

        view = view.on_scroll(move |_, delta, _| {
            // Down is positive, as for a scrolled list.
            let mut total = gathered.get() + delta;

            while total <= -SCROLL_STEP {
                total += SCROLL_STEP;
                if let Some(up) = &up {
                    up();
                }
            }

            while total >= SCROLL_STEP {
                total -= SCROLL_STEP;
                if let Some(down) = &down {
                    down();
                }
            }

            gathered.set(total);
        });
    }

    view
}

/// Runs `command` with the shell, off the UI thread.
fn run_command(command: String) {
    std::thread::spawn(move || {
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .status()
        {
            Ok(status) if !status.success() => log::warn!("`{command}` ended with {status}"),
            Ok(_) => {}
            Err(err) => log::warn!("`{command}` did not start: {err}"),
        }
    });
}
