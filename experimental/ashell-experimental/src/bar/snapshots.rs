use std::path::PathBuf;
use std::time::{Duration, Instant};

use guido::prelude::*;
use guido::testing::Headless;
use styletto::provide_ui;

use super::{NodeId, Opened, State, placed, view_with};
use crate::config::{self, Bar, Config, Panel};
use crate::modules::Modules;

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
    size: (u32, u32),
    scale: u32,
}

/// How a scene is drawn: its size, its scale and what shows round the bar,
/// guido's default with no `backdrop`.
struct Look {
    size: (u32, u32),
    scale: u32,
    backdrop: Option<Color>,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            size: (WIDTH, HEIGHT),
            scale: 1,
            backdrop: None,
        }
    }
}

impl Scene {
    fn new(text: &str) -> Option<Self> {
        crate::i18n::init();
        Self::build(text, Look::default(), Modules::new)
    }

    fn build(
        text: &str,
        look: Look,
        modules: impl Fn(Signal<Config>) -> Modules + 'static,
    ) -> Option<Self> {
        let Look {
            size,
            scale,
            backdrop,
        } = look;

        let Some(mut app) = Headless::new() else {
            eprintln!("no GPU adapter; nothing drawn");
            return None;
        };
        let output = app.connect_output("eDP-1");

        let config = config::parse(text).expect("valid TOML").config;
        let ui = config.appearance.ui(None);
        let bar = config.bars[0].clone();
        let state = State::new();

        let drawn = bar.clone();
        // The whole surface drawn, not only where the bar shows.
        let mut surface_config = SurfaceConfig::new()
            .width(0)
            .height(0)
            .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        if let Some(backdrop) = backdrop {
            surface_config = surface_config.background_color(backdrop);
        }

        let surface = app.surface(surface_config, move || {
            provide_ui(ui);
            provide_context(modules(create_signal(config.clone()).read_only()));

            view_with(&drawn, state)
        });
        app.configure(surface, size.0, size.1, scale as f32);
        app.enter_output(surface, output);

        let mut scene = Self {
            app,
            surface,
            bar,
            state,
            clock: Instant::now(),
            size,
            scale,
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

    fn frame(&self) -> ::image::RgbaImage {
        let pixels = self.app.read_frame(self.surface);
        let (width, height) = self.size;

        ::image::RgbaImage::from_raw(width * self.scale, height * self.scale, pixels).unwrap()
    }

    fn save(&self, name: &str) {
        let path = out_dir().join(format!("{name}.png"));

        self.frame().save(&path).unwrap();
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

/// A Tuesday morning in November, on London's clock.
fn site_now() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 11, 17)
        .unwrap()
        .and_hms_opt(9, 41, 0)
        .unwrap()
}

/// The site's theme, Tokyo Night's Moon or Day, and tempo at a fixed time.
fn site_config(mode: &str, bar: &str) -> String {
    format!(
        r#"
[appearance]
theme        = "tokyo night"
dark_flavor  = "moon"
light_flavor = "day"
mode         = "{mode}"

{bar}

[tempo]
timezones        = ["Europe/London", "America/New_York", "Asia/Tokyo"]
weather_location = {{ city = "London" }}
"#
    )
}

/// A bar as the docs' examples have it: its other zones placeholders.
fn site_common_bar(panels: &str) -> String {
    format!(
        r#"
[bar.main]
panels  = "{panels}"
padding = "s2"
start   = ["workspaces"]
center  = [["window", "clock"]]
end     = [["privacy", "status"]]
"#
    )
}

/// A bar of tempo's items alone.
const SITE_TEMPO_BAR: &str = r#"
[bar.main]
panels  = "merge"
padding = "s2"
hover   = true
start   = ["sky"]
center  = ["clock"]
end     = ["hours"]

[zone.sky]
items = ["weather"]

[zone.clock]
items   = ["date", "clock"]
details = ["forecast"]

[zone.hours]
items = [{ item = "forecast", hours = 4 }]
"#;

/// A bar of the settings module's items, each one shown.
const SITE_SETTINGS_BAR: &str = r#"
[bar.main]
padding = "s2"
end     = ["status"]

[zone.status]
items = [
    { item = "idle_inhibitor", show = "always" },
    { item = "power_profile", show = "always" },
    "bluetooth",
    { item = "brightness", show = "always" },
    "peripheral_battery",
    "battery",
    "battery_time",
]

[settings]
lock = "loginctl lock-session"
"#;

/// A frame read back premultiplied, as a PNG holds it: straight alpha.
fn unpremultiplied(mut frame: ::image::RgbaImage) -> ::image::RgbaImage {
    for pixel in frame.pixels_mut() {
        let alpha = u32::from(pixel[3]);

        if alpha > 0 && alpha < 255 {
            for channel in &mut pixel.0[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }
    }

    frame
}

/// `frame` as a lossless WebP, as small as libwebp makes one.
fn save_webp(frame: &::image::RgbaImage, path: &std::path::Path) {
    let mut config = webp::WebPConfig::new().unwrap();

    config.lossless = 1;
    config.method = 6;
    config.quality = 100.0;

    let encoded = webp::Encoder::from_rgba(frame.as_raw(), frame.width(), frame.height())
        .encode_advanced(&config)
        .unwrap();

    std::fs::write(path, &*encoded).unwrap();
}

/// What is drawn, with `margin` round it.
fn cut_to_drawn(frame: &::image::RgbaImage, margin: u32) -> ::image::RgbaImage {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);

    for (x, y, pixel) in frame.enumerate_pixels() {
        if pixel[3] > 0 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }

    let left = left.saturating_sub(margin);
    let top = top.saturating_sub(margin);
    let right = (right + margin).min(frame.width() - 1);
    let bottom = (bottom + margin).min(frame.height() - 1);

    ::image::imageops::crop_imm(frame, left, top, right - left + 1, bottom - top + 1).to_image()
}

/// A scene of the site, on a desktop of `size` with nothing behind, in each
/// theme, `show` putting it in the state pictured.
fn site_picture(name: &str, bar: &str, size: (u32, u32), cut: bool, show: fn(&mut Scene)) {
    let dir = out_dir().join("site");

    std::fs::create_dir_all(&dir).unwrap();

    for mode in ["dark", "light"] {
        let look = Look {
            size,
            scale: 2,
            backdrop: Some(Color::TRANSPARENT),
        };
        let Some(mut scene) = Scene::build(&site_config(mode, bar), look, |config| {
            Modules::fixed(config, site_now())
        }) else {
            return;
        };

        show(&mut scene);
        scene.run(Duration::from_millis(1200));

        let frame = unpremultiplied(scene.frame());
        let frame = if cut {
            cut_to_drawn(&frame, 24 * 2)
        } else {
            frame
        };
        let path = dir.join(format!("{name}-{mode}.webp"));

        save_webp(&frame, &path);
        println!("{}", path.display());
    }
}

fn open_calendar(scene: &mut Scene) {
    open(scene, "clock", Panel::Calendar);
}

fn open(scene: &mut Scene, zone: &str, panel: Panel) {
    let (group, zone) = zone_of(&scene.bar, zone);

    scene.state.toggle(Opened { group, zone, panel });
}

/// The pictures of the site: the ways a panel opens, for the panels' page,
/// tempo's items and calendar, and the settings items and panel, for their
/// own. The desktop they sit on is the page's.
#[test]
#[ignore = "draws the site's pictures, asserts nothing"]
fn site_pictures() {
    crate::i18n::init_for("en-US", "en_GB");

    for panels in ["dropdown", "merge", "morph"] {
        site_picture(
            &format!("panels-{panels}"),
            &site_common_bar(panels),
            (960, 540),
            false,
            open_calendar,
        );
    }

    site_picture("tempo-items", SITE_TEMPO_BAR, (960, 160), true, |scene| {
        scene.state.details.set(Some(zone_of(&scene.bar, "clock")));
    });
    site_picture(
        "tempo-calendar",
        SITE_TEMPO_BAR,
        (960, 540),
        false,
        open_calendar,
    );

    site_picture(
        "settings-items",
        SITE_SETTINGS_BAR,
        (960, 160),
        true,
        |_| {},
    );
    site_picture(
        "settings-panel",
        SITE_SETTINGS_BAR,
        (960, 540),
        true,
        |scene| {
            open(scene, "status", Panel::Settings);
        },
    );
}

/// The details as a pointer moves, with the events in the order guido sends
/// them for one move: every leave, then every enter, then the move.
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

        bar.state.moved_out();
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

        bar.state.moved_out();
        bar.state.hovered(workspaces.0, false);
        bar.state.hovered(window.0, true);
        bar.state.moved_on(window);
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

        bar.state.moved_out();
        bar.state.moved_on(clock);
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

        bar.state.moved_out();
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
