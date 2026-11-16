use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use guido::prelude::*;
use styletto::provide_ui;
use styletto::ui::Ui;

use crate::bar;
use crate::config::{self, Config};
use crate::modules::Modules;

pub fn run() {
    crate::i18n::init();

    App::new().run(|_| {
        let config = config::watched();
        provide_context(Modules::new(config));
        // A wallpaper is read again only when the appearance changes.
        let appearance = create_memo(move || config.with(|config| config.appearance.clone()));
        let ui = create_memo(move || appearance.with(|appearance| appearance.ui(None)));

        // On every change of the configuration the bars made are closed and
        // made again from it.
        create_effect(move || bars(config.get(), ui.get()));
    });
}

fn bars(config: Config, ui: Ui) {
    // `bar::surface` sizes the reserved space in the configured density.
    provide_ui(ui);

    let surfaces: Rc<RefCell<HashMap<(OutputId, usize), SurfaceHandle>>> = Rc::default();

    // Before the effect in `run` makes the bars again, the surfaces made
    // here are closed.
    on_cleanup({
        let surfaces = surfaces.clone();
        move || {
            for (_, surface) in surfaces.borrow_mut().drain() {
                surface.close();
            }
        }
    });

    // The bars are synced with the monitors every time guido's outputs change.
    create_effect(move || {
        let current = outputs().get();
        let wanted: Vec<(&OutputInfo, usize)> = current
            .iter()
            .flat_map(|info| {
                let on = config.bars_on(info.name.as_deref(), info.description.as_deref());
                on.iter().map(move |&index| (info, index))
            })
            .collect();
        let mut surfaces = surfaces.borrow_mut();

        // Every surface no longer wanted is closed.
        surfaces.retain(|&key, surface| {
            let kept = wanted.iter().any(|&(info, index)| (info.id, index) == key);
            if !kept {
                surface.close();
            }
            kept
        });

        // Every wanted bar is made, from its root node, unless it is already
        // open or its monitor has no size yet: it is made once it has one.
        for &(info, index) in &wanted {
            if surfaces.contains_key(&(info.id, index)) {
                continue;
            }
            if info.logical_size.is_none() {
                continue;
            }

            let bar = config.bars[index].clone();
            let surface_config = bar::surface(&bar).output(info.id);

            let surface = spawn_surface(surface_config, move || {
                provide_ui(ui);

                bar::view(&bar)
            });

            surfaces.insert((info.id, index), surface);
        }
    });
}
