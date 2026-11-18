use std::path::PathBuf;
use std::time::{Duration, Instant};

use guido::prelude::*;
use guido::testing::Headless;
use styletto::provide_ui;

use super::{NodeId, Opened, State, placed, view_with};
use crate::config::{self, Bar, Panel};

const WIDTH: u32 = 1440;
const HEIGHT: u32 = 700;
const FRAME: Duration = Duration::from_millis(16);

fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
    std::fs::create_dir_all(&dir).unwrap();

    dir
}

struct Scene {
    app: Headless,
    surface: SurfaceId,
    bar: Bar,
    state: State,
    clock: Instant,
}

impl Scene {
    fn new(text: &str) -> Option<Self> {
        let Some(mut app) = Headless::new() else {
            eprintln!("no GPU adapter; nothing drawn");
            return None;
        };
        let output = app.connect_output("eDP-1");

        let mut config = config::parse(text).expect("valid TOML").config;
        let ui = config.appearance.ui(None);
        let bar = config.bars.swap_remove(0);
        let state = State::new();

        let drawn = bar.clone();
        // The whole surface drawn, not only where the bar shows.
        let surface = app.surface(
            SurfaceConfig::new()
                .width(0)
                .height(0)
                .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT),
            move || {
                provide_ui(ui);

                view_with(&drawn, state)
            },
        );
        app.configure(surface, WIDTH, HEIGHT, 1.0);
        app.enter_output(surface, output);

        let mut scene = Self {
            app,
            surface,
            bar,
            state,
            clock: Instant::now(),
        };
        scene.run(Duration::from_millis(100));

        Some(scene)
    }

    fn step(&mut self) {
        self.clock += FRAME;
        self.app.step_at(self.clock);
    }

    fn run(&mut self, span: Duration) {
        for _ in 0..(span.as_millis() / FRAME.as_millis()).max(1) {
            self.step();
        }
    }

    fn save(&self, name: &str) {
        let pixels = self.app.read_frame(self.surface);
        let path = out_dir().join(format!("{name}.png"));

        ::image::RgbaImage::from_raw(WIDTH, HEIGHT, pixels)
            .unwrap()
            .save(&path)
            .unwrap();
        println!("{}", path.display());
    }

    fn record(&mut self, name: &str, span: Duration) {
        for frame in 0..span.as_millis() / FRAME.as_millis() {
            self.step();
            self.save(&format!("{name}-{frame:03}"));
        }
    }
}

const FLOATING: &str = include_str!("../../docs/examples/floating.toml");

fn floating(panels: &str, padding: bool) -> String {
    let padding = if padding { "s2" } else { "s0" };

    format!(
        r#"
[bar.main]
shape    = true
padding  = "{padding}"
panels   = "{panels}"
start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
"#
    )
}

/// `config` with `line` added to its bar.
fn with(config: &str, line: &str) -> String {
    config.replace("[bar.main]", &format!("[bar.main]\n{line}"))
}

const STRIP: &str = r#"
[bar.main]
background = "solid"
shape      = false
panels     = "merge"
hover      = true
start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
"#;

const FLAT: &str = r#"
[bar.main]
background = "solid"
shape      = false
start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
"#;

const CLUSTERS: &str = r#"
[bar.main]
panels = "merge"
start  = { nodes = ["workspaces"], shape = true }
center = { nodes = ["clock", "window"], shape = true }
end    = { nodes = [["privacy", "status"]], shape = true }
"#;

const HALVES: &str = r#"
[bar.main]
panels = "merge"
hover  = true
start  = { shape = true, start = ["workspaces"], end = ["window"] }
end    = { shape = true, start = ["media"], end = [["clock", "status"]] }
"#;

const BARE: &str = r#"
[bar.main]
shape  = false
start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
"#;

const HALVES_CLOCK_LEFT: &str = r#"
[bar.main]
panels = "merge"
start  = { shape = true, start = ["workspaces"], end = ["clock"] }
end    = { shape = true, start = ["media"], end = ["status"] }
"#;

const ISLANDS: &str = r#"
[bar.main]
panels = "merge"
start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
"#;

const PUSHED: &str = r#"
[bar.main]
start  = ["workspaces"]
center = [["clock", "media"]]
end    = [["window"], ["tray", "updates", "keyboard"], ["workspaces", "clock"], ["privacy", "system", "status"]]
"#;

/// The group that has `zone`, and where it is in it.
fn zone_of(bar: &Bar, zone: &str) -> (NodeId, usize) {
    placed(bar)
        .into_iter()
        .find_map(|(id, placed)| {
            let index = placed.group.zones().iter().position(|z| z.name == zone)?;
            Some((id, index))
        })
        .expect("a group has the zone")
}

fn open_and_close(name: &str, config: &str, zone: &str, panel: Panel) {
    let Some(mut bar) = Scene::new(config) else {
        return;
    };

    let (group, zone) = zone_of(&bar.bar, zone);
    let opened = Opened { group, zone, panel };

    bar.save(&format!("{name}-rest"));

    bar.state.toggle(opened);
    bar.record(&format!("{name}-open"), Duration::from_millis(800));

    bar.state.close();
    bar.record(&format!("{name}-close"), Duration::from_millis(900));
}

fn details(name: &str, config: &str, zone: &str) {
    let Some(mut bar) = Scene::new(config) else {
        return;
    };

    bar.state.details.set(Some(zone_of(&bar.bar, zone)));
    bar.record(&format!("{name}-open"), Duration::from_millis(600));

    bar.state.details.set(None);
    bar.record(&format!("{name}-close"), Duration::from_millis(600));
}

/// The details shown, then a click on the zone: the details go as the panel
/// comes out, and the panel ends against the island as it is at rest.
fn details_then_panel(name: &str, config: &str, zone: &str, panel: Panel) {
    let Some(mut bar) = Scene::new(config) else {
        return;
    };

    let (group, zone) = zone_of(&bar.bar, zone);
    bar.state.details.set(Some((group, zone)));
    bar.run(Duration::from_millis(600));

    bar.state.toggle(Opened { group, zone, panel });
    bar.record(&format!("{name}-open"), Duration::from_millis(900));
}

fn details_moving(name: &str, config: &str, from: &str, to: &str) {
    let Some(mut bar) = Scene::new(config) else {
        return;
    };

    bar.state.details.set(Some(zone_of(&bar.bar, from)));
    bar.record(&format!("{name}-open"), Duration::from_millis(500));

    bar.state.details.set(Some(zone_of(&bar.bar, to)));
    bar.record(&format!("{name}-move"), Duration::from_millis(500));

    bar.state.details.set(None);
    bar.record(&format!("{name}-close"), Duration::from_millis(500));
}

#[test]
#[ignore = "draws PNGs to look at, asserts nothing"]
fn snapshots() {
    open_and_close("floating", FLOATING, "status", Panel::Settings);

    let calendar = |name, config: &str| open_and_close(name, config, "clock", Panel::Calendar);
    calendar("floating-center", &floating("merge", true));
    calendar("islands-center", ISLANDS);
    calendar(
        "floating-apart",
        &with(&floating("merge", true), "gap = \"s4\""),
    );
    calendar("floating-unmerged", &floating("dropdown", true));
    calendar("strip", STRIP);
    calendar("floating-flush", &floating("merge", false));
    calendar("flat", FLAT);

    open_and_close("clusters", CLUSTERS, "clock", Panel::Calendar);
    open_and_close("halves", HALVES, "clock", Panel::Calendar);
    open_and_close("halves-inner", HALVES_CLOCK_LEFT, "clock", Panel::Calendar);
    open_and_close("bare", BARE, "clock", Panel::Calendar);

    let status = |name, config: &str| details(name, config, "status");
    status("details-strip", &floating("merge", true));
    status(
        "details-strip-bottom",
        &with(&floating("merge", true), "position = \"bottom\""),
    );
    status("details-clusters", CLUSTERS);
    status("details-background", STRIP);
    status("details-islands", ISLANDS);

    status("details-halves", HALVES);
    status("details-pushed", PUSHED);
    details("details-pushed-window", PUSHED, "window");
    details_then_panel("details-then-panel", ISLANDS, "clock", Panel::Calendar);
    details_then_panel("details-then-panel-end", ISLANDS, "status", Panel::Settings);
    details_then_panel(
        "details-then-dropdown",
        &ISLANDS.replace("merge", "dropdown"),
        "clock",
        Panel::Calendar,
    );
    details_moving(
        "details-moving",
        &floating("merge", true),
        "window",
        "clock",
    );
    status("details-bare", BARE);
    status("details-apart", &with(STRIP, "gap = \"s4\""));
    details_moving("details-across", STRIP, "workspaces", "status");
}

/// The details as a pointer moves, with the events in the order guido sends
/// them: the moves as it walks the tree, then every leave, then every enter.
mod details_follow_the_pointer {
    use std::time::Duration;

    use super::{STRIP, Scene, zone_of};

    fn showing(bar: &mut Scene, zone: (usize, usize)) {
        bar.state.hovered(zone.0, true);
        bar.state.details.set(Some(zone));
        bar.run(Duration::from_millis(50));
    }

    #[test]
    fn they_close_when_the_pointer_moves_on_to_a_zone_of_another_group_with_none() {
        let Some(mut bar) = Scene::new(STRIP) else {
            return;
        };
        let (clock, privacy) = (zone_of(&bar.bar, "clock"), zone_of(&bar.bar, "privacy"));
        showing(&mut bar, clock);

        bar.state.moved_off(privacy.0);
        bar.state.hovered(clock.0, false);
        bar.state.hovered(privacy.0, true);
        for _ in 0..30 {
            bar.state.moved_off(privacy.0);
            bar.step();
        }

        assert_eq!(bar.state.details.get_untracked(), None);
    }

    #[test]
    fn they_come_for_a_zone_reached_from_another_group_in_one_move() {
        let Some(mut bar) = Scene::new(STRIP) else {
            return;
        };
        let (workspaces, window) = (zone_of(&bar.bar, "workspaces"), zone_of(&bar.bar, "window"));
        bar.state.hovered(workspaces.0, true);

        bar.state.moved_on(window);
        bar.state.moved_out(workspaces);
        bar.state.hovered(workspaces.0, false);
        bar.state.hovered(window.0, true);
        bar.run(Duration::from_millis(400));

        assert_eq!(bar.state.details.get_untracked(), Some(window));
    }

    #[test]
    fn they_move_to_the_next_zone_of_the_group_in_one_move() {
        let Some(mut bar) = Scene::new(STRIP) else {
            return;
        };
        let (window, clock) = (zone_of(&bar.bar, "window"), zone_of(&bar.bar, "clock"));
        showing(&mut bar, window);

        bar.state.moved_on(clock);
        bar.state.moved_out(window);
        bar.run(Duration::from_millis(200));

        assert_eq!(bar.state.details.get_untracked(), Some(clock));
    }

    #[test]
    fn they_close_once_the_pointer_has_left_and_stay_if_it_comes_back() {
        let Some(mut bar) = Scene::new(STRIP) else {
            return;
        };
        let clock = zone_of(&bar.bar, "clock");
        showing(&mut bar, clock);

        bar.state.moved_out(clock);
        bar.state.hovered(clock.0, false);
        bar.run(Duration::from_millis(200));
        bar.state.hovered(clock.0, true);
        bar.run(Duration::from_millis(500));
        assert_eq!(bar.state.details.get_untracked(), Some(clock));

        bar.state.hovered(clock.0, false);
        bar.run(Duration::from_millis(500));
        assert_eq!(bar.state.details.get_untracked(), None);
    }
}
