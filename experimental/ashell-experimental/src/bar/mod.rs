mod group;
mod placeholder;
mod places;
#[cfg(test)]
mod snapshots;

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use guido::prelude::*;
use styletto::foundations::control::Size;
use styletto::foundations::tokens::{radius, size, stroke};
use styletto::primitives::island;
use styletto::ui;

use crate::config::{Background, Bar, Holds, Host, Node, Padding, Panel, Panels, Place, Position};

const DETAILS_DWELL: Duration = Duration::from_millis(260);
const ZONE_SWITCH: Duration = Duration::from_millis(90);
const DETAILS_CLOSE: Duration = Duration::from_millis(350);

const GROUP: Size = Size::Md;

fn group_height() -> f32 {
    GROUP.height()
}

fn edge() -> f32 {
    size::S2.px()
}

fn details_gap() -> f32 {
    size::S1.px()
}

fn panel_gap(panels: Panels) -> f32 {
    match panels {
        Panels::Detached => size::S3.px(),
        Panels::Morph | Panels::Attached | Panels::Dropdown => size::S2.px(),
    }
}

fn reserved(bar: &Bar) -> u32 {
    (bar.padding.top.px() + group_height() + bar.padding.bottom.px()).round() as u32
}

pub fn surface(bar: &Bar) -> SurfaceConfig {
    let edge = match bar.position {
        Position::Top => Edge::Top,
        Position::Bottom => Edge::Bottom,
    };

    SurfaceConfig::new()
        // As big as the compositor gives, for good: a panel needs room up to
        // the far edge, and a size asked for may be more than it gives, which
        // cuts the bar off for a frame. The input and the buffer go only where
        // the bar shows.
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .buffer(BufferExtent::Content)
        .layer(Layer::Top)
        .exclusive_zone(reserved(bar))
        .exclusive_edge(edge)
        .keyboard_interactivity(KeyboardInteractivity::None)
        .click_through()
        .namespace("ashell-experimental")
        .background_color(Color::TRANSPARENT)
}

impl Position {
    fn side(self) -> f32 {
        match self {
            Position::Top => 1.0,
            Position::Bottom => -1.0,
        }
    }

    fn cross(self) -> CrossAlignment {
        match self {
            Position::Top => CrossAlignment::Start,
            Position::Bottom => CrossAlignment::End,
        }
    }

    fn alignment(self) -> Alignment {
        match self {
            Position::Top => Alignment::TOP_CENTER,
            Position::Bottom => Alignment::BOTTOM_CENTER,
        }
    }
}

pub type NodeId = usize;

/// A group as the bar places it.
#[derive(Clone)]
pub struct Placed {
    pub group: Rc<Node>,
    /// The list of nodes it is in: the node holding it, and where in it, the
    /// nearest place round the group.
    list: (NodeId, Place),
}

impl Placed {
    fn place(&self) -> Place {
        self.list.1
    }

    /// What its panels come out of: what hosts it if they merge with it,
    /// else the group itself.
    fn opens_from(&self) -> Host {
        if self.group.panels.merges() {
            self.group.host
        } else {
            Host::Own
        }
    }
}

/// Every group, with the list it is in.
pub fn placed(bar: &Bar) -> HashMap<NodeId, Placed> {
    fn walk(node: &Node, list: (NodeId, Place), all: &mut HashMap<NodeId, Placed>) {
        match &node.holds {
            Holds::Zones(_) => {
                let group = Rc::new(node.clone());

                all.insert(node.id, Placed { group, list });
            }
            Holds::Nodes(nodes) => {
                for inner in nodes {
                    walk(inner, (node.id, list.1), all);
                }
            }
            Holds::Placed(places) => {
                for (place, nodes) in Place::ALL.into_iter().zip(places) {
                    for inner in nodes {
                        walk(inner, (node.id, place), all);
                    }
                }
            }
        }
    }

    let mut all = HashMap::new();
    walk(&bar.root, (bar.root.id, Place::Center), &mut all);

    all
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opened {
    pub group: NodeId,
    pub zone: usize,
    pub panel: Panel,
}

#[derive(Clone, Copy)]
pub struct State {
    pub opened: RwSignal<Option<Opened>>,
    pub details: RwSignal<Option<(NodeId, usize)>>,
    inside: RwSignal<Option<NodeId>>,
    /// The next change of the details, waiting its time.
    pending: RwSignal<Option<TimerHandle>>,
}

impl State {
    fn new() -> Self {
        Self {
            opened: create_signal(None),
            details: create_signal(None),
            inside: create_signal(None),
            pending: create_signal(None),
        }
    }

    pub fn toggle(&self, opened: Opened) {
        self.details.set(None);

        let next = (self.opened.get_untracked() != Some(opened)).then_some(opened);
        self.opened.set(next);
    }

    pub fn close(&self) {
        self.opened.set(None);
    }

    pub fn moved_on(&self, zone: (NodeId, usize)) {
        self.entered(zone.0);
        self.moved();

        let shown = self.details.get_untracked();
        if shown == Some(zone) {
            return;
        }

        let wait = match shown {
            Some((group, _)) if group == zone.0 => ZONE_SWITCH,
            _ => DETAILS_DWELL,
        };

        let state = *self;
        let pending = set_timeout(wait, move || {
            if state.opened.get_untracked().is_none() {
                state.details.set(Some(zone));
            }
        });
        self.pending.set(Some(pending));
    }

    /// On a zone of `group` with no details: those shown there go.
    pub fn moved_off(&self, group: NodeId) {
        self.entered(group);
        self.moved();
        if !self.shows(group) {
            return;
        }

        let state = *self;
        let pending = set_timeout(ZONE_SWITCH, move || {
            if state.shows(group) {
                state.details.set(None);
            }
        });
        self.pending.set(Some(pending));
    }

    fn shows(&self, group: NodeId) -> bool {
        self.details
            .get_untracked()
            .is_some_and(|(g, _)| g == group)
    }

    pub fn moved(&self) {
        if let Some(pending) = self.pending.get_untracked() {
            pending.cancel();
        }
    }

    pub fn entered(&self, group: NodeId) {
        self.inside.set(Some(group));
    }

    pub fn left(&self, group: NodeId) {
        if self.inside.get_untracked() == Some(group) {
            self.inside.set(None);
        }
        self.moved();

        let state = *self;
        let pending = set_timeout(DETAILS_CLOSE, move || {
            let back = state.inside.get_untracked() == Some(group);

            if !back && state.shows(group) {
                state.details.set(None);
            }
        });
        self.pending.set(Some(pending));
    }
}

/// One more in `count` until the scope this runs in is disposed.
fn counted(count: RwSignal<usize>) {
    count.update(|n| *n += 1);
    on_cleanup(move || count.update(|n| *n -= 1));
}

#[derive(Clone)]
struct Ctx {
    state: State,
    refs: Rc<Refs>,
    groups: Rc<HashMap<NodeId, Placed>>,
    position: Position,
    padding: Padding,
    reserved: f32,
}

/// How far to shift the panel sideways so that it sits under the clicked zone.
fn under_zone(opened: Opened, middle: f32, ctx: &Ctx) -> f32 {
    let zone = ctx.refs.zones[&(opened.group, opened.zone)]
        .layout_rect()
        .get_untracked();
    let screen = ctx.refs.bar.layout_rect().get_untracked().width;
    let width = placeholder::panel_width();

    // Inside the ends of the shape round the group, when it is wide enough
    // to hold the panel, else inside the bar's.
    let bar = (
        ctx.padding.left.px().max(edge()),
        screen - ctx.padding.right.px().max(edge()),
    );
    let host = host_rect(ctx.groups[&opened.group].group.host, opened.group, ctx);
    let within = if host.width >= width {
        (host.x.max(bar.0), (host.x + host.width).min(bar.1))
    } else {
        bar
    };

    panel_center(zone, width, within) - middle
}

struct Refs {
    bar: WidgetRef,
    groups: HashMap<NodeId, WidgetRef>,
    /// The shapes of the nodes that are not groups.
    shapes: HashMap<NodeId, WidgetRef>,
    zones: HashMap<(NodeId, usize), WidgetRef>,
}

/// The area of the surface that the panels and details of `group` come out of.
fn host_rect(host: Host, group: NodeId, ctx: &Ctx) -> Rect {
    let rect = |widget: &WidgetRef| widget.layout_rect().get_untracked();

    match host {
        Host::Own => rect(&ctx.refs.groups[&group]),
        Host::Shape(id) => rect(&ctx.refs.shapes[&id]),
        Host::Background | Host::None => {
            let bar = rect(&ctx.refs.bar);
            let y = match ctx.position {
                Position::Top => 0.0,
                Position::Bottom => bar.height - ctx.reserved,
            };

            Rect {
                x: 0.0,
                y,
                width: bar.width,
                height: ctx.reserved,
            }
        }
    }
}

/// Centres the panel under the zone, then shifts it just enough to keep it
/// inside the bounds.
fn panel_center(zone: Rect, width: f32, (low, high): (f32, f32)) -> f32 {
    (zone.x + zone.width / 2.0)
        .min(high - width / 2.0)
        .max(low + width / 2.0)
}

pub fn view(bar: &Bar) -> Container {
    view_with(bar, State::new())
}

fn view_with(bar: &Bar, state: State) -> Container {
    let surface = expect_context::<SurfaceHandle>();
    let open = create_memo(move || state.opened.get().is_some());

    let groups = placed(bar);

    let mut refs = Refs {
        bar: create_widget_ref(),
        groups: HashMap::new(),
        shapes: HashMap::new(),
        zones: HashMap::new(),
    };
    for node in bar.nodes() {
        if node.shape && !node.is_group() {
            refs.shapes.insert(node.id, create_widget_ref());
        }
    }

    for (&id, placed) in &groups {
        refs.groups.insert(id, create_widget_ref());

        for zone in 0..placed.group.zones().len() {
            refs.zones.insert((id, zone), create_widget_ref());
        }
    }

    let ctx = Ctx {
        state,
        refs: Rc::new(refs),
        groups: Rc::new(groups),
        position: bar.position,
        padding: bar.padding,
        reserved: reserved(bar) as f32,
    };

    keyboard_follows(surface, open);

    let backdrop = container()
        .width(fill())
        .height(fill())
        .takes_input(move || open.get())
        .on_click(move || state.close());

    // Room on the inner side for what shapes cast: a surface sized to its
    // content cuts what is drawn past it.
    let shadows = bar.nodes().iter().any(|node| node.shape);
    let cast = island::shadow(Color::TRANSPARENT).outsets();
    let p = bar.padding;
    let room = |padding: f32, cast: f32| if shadows { padding.max(cast) } else { padding };
    let (top, bottom) = match bar.position {
        Position::Top => (p.top.px(), room(p.bottom.px(), cast.bottom)),
        Position::Bottom => (room(p.top.px(), cast.top), p.bottom.px()),
    };

    let nodes = self::places(bar.places(), true, &ctx);
    let nodes = shaped(nodes, &bar.root, true, &ctx)
        .align_self(bar.position.alignment())
        .padding([top, p.right.px(), bottom, p.left.px()]);

    container()
        .width(fill())
        .height(fill())
        .widget_ref(ctx.refs.bar)
        .layout(ZStack::new())
        .on_key_down(move |key, _, _| {
            if key == Key::Escape && state.opened.get_untracked().is_some() {
                state.close();
                EventResponse::Handled
            } else {
                EventResponse::Ignored
            }
        })
        .child(backdrop)
        .child((bar.background == Background::Solid).then(|| background(&ctx)))
        .child(nodes)
}

/// Adds to `holder` the open panel, if `opens_here` says it comes out of
/// `holder`. It is added first, so the shape added after it covers it.
///
/// With `merge`, the shapes in `holder` merge into one. While a `detached`
/// panel is out they merge only when very close, so its neck breaks as it
/// moves away. Once it has gone they merge as before, and the details that
/// come out of `holder` have their neck again.
fn hosting(
    holder: Container,
    merge: bool,
    ctx: &Ctx,
    opens_here: impl Fn(&Opened) -> bool + 'static,
    panel: impl Fn(Opened) -> Container + 'static,
) -> Container {
    let (state, groups) = (ctx.state, ctx.groups.clone());

    let open = create_memo(move || state.opened.get().filter(|opened| opens_here(opened)));
    let detached = create_signal(0usize);

    let holder = if merge {
        merged(holder, move || reach(detached.get() > 0))
    } else {
        holder
    };

    holder.children(keyed(
        move || open.get().into_iter().collect::<Vec<_>>(),
        |opened| (opened.group, opened.zone, opened.panel),
        move |opened| {
            if groups[&opened.group].group.panels == Panels::Detached {
                counted(detached);
            }

            panel(opened)
        },
    ))
}

/// Adds to `holder` the details shown, if `shows_here` says they come out of
/// `holder`: a small shape coming out of `host`, one for each list of nodes.
fn hosting_details(
    holder: Container,
    ctx: &Ctx,
    shows_here: impl Fn(NodeId, &Placed) -> bool + 'static,
    host: impl Fn() -> Rect + 'static,
) -> Container {
    let (state, groups) = (ctx.state, ctx.groups.clone());

    let shown = create_memo(move || {
        state
            .details
            .get()
            .filter(|&(group, _)| shows_here(group, &groups[&group]))
    });

    let groups = ctx.groups.clone();
    let list = create_memo(move || shown.get().map(|(group, _)| groups[&group].list));

    let place = ctx.clone();
    holder.children(keyed(
        move || list.get().into_iter().collect::<Vec<_>>(),
        |&list| list,
        move |_| group::split_details(shown, host(), &place),
    ))
}

/// How far the shapes reach for each other: less round a `detached` panel,
/// so the neck breaks as the panel gets where it goes.
fn reach(detached: bool) -> f32 {
    if detached {
        size::S4.px()
    } else {
        group::merge_distance()
    }
}

/// `holder` as what the panels and details of the groups `host` takes come
/// out of.
fn hosts(holder: Container, host: Host, merge: bool, ctx: &Ctx) -> Container {
    let (place, groups) = (ctx.clone(), ctx.groups.clone());
    let holder = hosting(
        holder,
        merge,
        ctx,
        move |opened| groups[&opened.group].opens_from() == host,
        move |opened| {
            let rect = host_rect(host, opened.group, &place);

            group::split_off(opened, rect, &place).align_self(place.position.alignment())
        },
    );

    let place = ctx.clone();
    hosting_details(
        holder,
        ctx,
        move |_, placed| placed.group.host == host,
        move || host_rect(host, 0, &place),
    )
}

fn merged<M>(holder: Container, reach: impl IntoSignal<f32, M>) -> Container {
    holder
        .merge_children(reach)
        .border(stroke::HAIRLINE, ui().colors.line)
}

/// Draws the bar's solid background, from edge to edge. The panels and
/// details of the groups with no shape come out of it.
fn background(ctx: &Ctx) -> Container {
    let (bar_ref, height) = (ctx.refs.bar, ctx.reserved);

    // Merging also draws a border round it, so only if something comes out
    // of it.
    let merge = ctx
        .groups
        .values()
        .any(|placed| placed.group.host == Host::Background);

    // Bigger than the surface at both ends and towards the screen's edge: the
    // surface cuts those sides, so the border drawn round the merged shapes
    // shows only on the inner one. That border is drawn inside the fill, so
    // the fill is taller by its width, leaving the bar all background.
    let beyond = group::merge_distance() * 2.0;
    let border = if merge { stroke::HAIRLINE } else { 0.0 };
    let side = ctx.position.side();
    let fill = group::shape_fill(container())
        .width(move || natural().at_least(bar_ref.layout_rect().get().width + 2.0 * beyond))
        .height(natural().at_least(height + border + beyond))
        .translate(Translate::new(-beyond, -side * beyond));

    let layer = container()
        .width(guido::prelude::fill())
        .height(guido::prelude::fill())
        .layout(ZStack::new().align(match ctx.position {
            Position::Top => Alignment::TOP_START,
            Position::Bottom => Alignment::BOTTOM_START,
        }));

    hosts(layer, Host::Background, merge, ctx).child(fill)
}

impl Place {
    fn main(self) -> MainAlignment {
        match self {
            Place::Start => MainAlignment::Start,
            Place::Center => MainAlignment::Center,
            Place::End => MainAlignment::End,
        }
    }

    /// The ends have room to fill; the centre's is its content's.
    fn fills(self) -> bool {
        self != Place::Center
    }
}

/// The three places side by side. `wide`: as wide as the room there is, the
/// centre in its middle (see `PlacesLayout`); else one after the other.
fn places(places: &[Vec<Node>; 3], wide: bool, ctx: &Ctx) -> Container {
    let lists: Vec<Container> = Place::ALL
        .into_iter()
        .zip(places)
        .map(|(place, nodes)| {
            let list = list(nodes, place, ctx);
            if wide && place.fills() {
                list.width(fill())
            } else {
                list
            }
        })
        .collect();

    let row = if wide {
        container().width(fill()).layout(places::PlacesLayout {
            spacing: size::S2.px(),
            cross: ctx.position.cross(),
        })
    } else {
        container().layout(
            Flex::row()
                .spacing(size::S2.px())
                .cross_alignment(ctx.position.cross()),
        )
    };

    row.children(lists)
}

fn list(nodes: &[Node], place: Place, ctx: &Ctx) -> Container {
    container()
        .layout(
            Flex::row()
                .spacing(size::S2.px())
                .main_alignment(place.main())
                .cross_alignment(ctx.position.cross()),
        )
        .children(
            nodes
                .iter()
                .map(|node| self::node(node, place, ctx))
                .collect::<Vec<_>>(),
        )
}

fn node(node: &Node, place: Place, ctx: &Ctx) -> Container {
    let wide = place.fills();
    let (content, wide) = match &node.holds {
        Holds::Zones(_) => return group::view(node.id, ctx),
        Holds::Nodes(nodes) => (list(nodes, place, ctx), false),
        Holds::Placed(places) => (self::places(places, wide, ctx), wide),
    };

    shaped(content, node, wide, ctx)
}

fn shaped(content: Container, node: &Node, wide: bool, ctx: &Ctx) -> Container {
    if node.shape {
        node_shape(content, node.id, wide, ctx)
    } else {
        content
    }
}

fn node_shape(content: Container, id: NodeId, wide: bool, ctx: &Ctx) -> Container {
    let shape_ref = ctx.refs.shapes[&id];

    // Not eased: it takes the width of what it holds, which is eased already.
    let shape = group::shape_fill(container())
        .widget_ref(shape_ref)
        .width(if wide { fill() } else { natural() })
        .height(natural())
        .padding([0.0, size::S1.px()])
        .corners(radius::pill(group_height()))
        .child(content);

    let holder = container().layout(ZStack::new().align(ctx.position.alignment()));
    let holder = if wide {
        holder.width(fill())
    } else {
        holder.width(move || shape_ref.layout_rect().get().width)
    };

    hosts(holder, Host::Shape(id), true, ctx)
        .child(shape)
        .shadow(merged_shadow())
}

/// The shadow of shapes that merge, cast by the merged outline: cast by one
/// of them, it is left out only from under that one, and shows through the
/// others where they are translucent.
fn merged_shadow() -> Shadows {
    island::shadow(Color::TRANSPARENT)
}

fn keyboard_follows(surface: SurfaceHandle, open: Memo<bool>) {
    create_effect(move || {
        surface.set_keyboard_interactivity(if open.get() {
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::None
        });
    });
}
