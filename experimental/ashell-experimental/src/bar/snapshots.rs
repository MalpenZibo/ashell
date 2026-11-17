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

const STRIP: &str = r#"
[bar.main]
background = "solid"
shape      = false
panels     = "attached"
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
panels = "attached"
start  = { nodes = ["workspaces"], shape = true }
center = { nodes = ["clock", "window"], shape = true }
end    = { nodes = [["privacy", "status"]], shape = true }
"#;

const HALVES: &str = r#"
[bar.main]
panels = "attached"
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
panels = "attached"
start  = { shape = true, start = ["workspaces"], end = ["clock"] }
end    = { shape = true, start = ["media"], end = ["status"] }
"#;

const ISLANDS: &str = r#"
[bar.main]
panels = "attached"
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
    calendar("floating-center", &floating("attached", true));
    calendar("islands-center", ISLANDS);
    calendar("floating-detached", &floating("detached", true));
    calendar("floating-unmerged", &floating("dropdown", true));
    calendar("strip", STRIP);
    calendar("floating-flush", &floating("attached", false));
    calendar("flat", FLAT);

    open_and_close("clusters", CLUSTERS, "clock", Panel::Calendar);
    open_and_close("halves", HALVES, "clock", Panel::Calendar);
    open_and_close("halves-inner", HALVES_CLOCK_LEFT, "clock", Panel::Calendar);
    open_and_close("bare", BARE, "clock", Panel::Calendar);

    let status = |name, config: &str| details(name, config, "status");
    status("details-strip", &floating("attached", true));
    status(
        "details-strip-bottom",
        &floating("attached", true).replace("[bar.main]", "[bar.main]\nposition = \"bottom\""),
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
        &ISLANDS.replace("attached", "dropdown"),
        "clock",
        Panel::Calendar,
    );
    details_moving(
        "details-moving",
        &floating("attached", true),
        "window",
        "clock",
    );
}
