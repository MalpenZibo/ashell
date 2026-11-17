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
use styletto::foundations::tokens::{radius, size};
use styletto::primitives::island;

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

/// How far shapes reach for each other to merge.
fn merge_distance() -> f32 {
    size::S8.px()
}

/// How far a shape stands off what it comes out of, and how far the two
/// reach for each other. Between two edges `gap` apart guido's smooth minimum
/// keeps a neck only while `gap < reach / 2`, as thick as `reach / 4 - gap / 2`.
#[derive(Clone, Copy, Debug)]
struct Neck {
    gap: f32,
    reach: f32,
}

impl Neck {
    /// A panel and its group: a `detached` one stands further off and its
    /// shapes reach less, so its neck breaks as it arrives; the others are
    /// joined.
    fn of(panels: Panels) -> Self {
        match panels {
            Panels::Detached => Self {
                gap: size::S3.px(),
                reach: size::S4.px(),
            },
            Panels::Morph | Panels::Attached | Panels::Dropdown => Self {
                gap: size::S2.px(),
                reach: merge_distance(),
            },
        }
    }

    /// The details split off what holds their group, with a thick neck.
    fn details() -> Self {
        Self {
            gap: size::S1.px(),
            reach: merge_distance(),
        }
    }

    #[cfg(test)]
    fn joined(self) -> bool {
        self.gap < self.reach / 2.0
    }
}

fn reserved(bar: &Bar) -> u32 {
    (bar.padding.top.px() + group_height() + bar.padding.bottom.px()).round() as u32
}

pub fn surface(bar: &Bar) -> SurfaceConfig {
    SurfaceConfig::new()
        // As big as the compositor gives, for good: a panel needs room up to
        // the far edge, and a size asked for may be more than it gives, which
        // cuts the bar off for a frame. The input and the buffer go only where
        // the bar shows.
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .buffer(BufferExtent::Content)
        .layer(Layer::Top)
        .exclusive_zone(reserved(bar))
        .exclusive_edge(bar.position.edge())
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

    fn edge(self) -> Edge {
        match self {
            Position::Top => Edge::Top,
            Position::Bottom => Edge::Bottom,
        }
    }

    fn alignment(self) -> Alignment {
        self.at(Place::Center)
    }

    /// Against the bar's edge, at `place` along it.
    fn at(self, place: Place) -> Alignment {
        match (self, place) {
            (Position::Top, Place::Start) => Alignment::TOP_START,
            (Position::Top, Place::Center) => Alignment::TOP_CENTER,
            (Position::Top, Place::End) => Alignment::TOP_END,
            (Position::Bottom, Place::Start) => Alignment::BOTTOM_START,
            (Position::Bottom, Place::Center) => Alignment::BOTTOM_CENTER,
            (Position::Bottom, Place::End) => Alignment::BOTTOM_END,
        }
    }

    /// `near` and `far` from the bar's edge, in order down the screen.
    fn near_first<T>(self, near: T, far: T) -> (T, T) {
        match self {
            Position::Top => (near, far),
            Position::Bottom => (far, near),
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
            Host::Shape(self.group.id)
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
    let host = host_rect(ctx.groups[&opened.group].group.host, ctx);
    let within = if host.width >= width {
        (host.x.max(bar.0), (host.x + host.width).min(bar.1))
    } else {
        bar
    };

    panel_center(zone, width, within) - middle
}

struct Refs {
    bar: WidgetRef,
    /// Every group, and every other node with a shape.
    nodes: HashMap<NodeId, WidgetRef>,
    zones: HashMap<(NodeId, usize), WidgetRef>,
}

/// The area of the surface that panels and details come out of, as the last
/// layout left it.
fn host_rect(host: Host, ctx: &Ctx) -> Rect {
    host_area(host, ctx, |widget| widget.layout_rect().get_untracked())
}

/// The same area, followed: what reads it moves with what it comes out of,
/// as a panel does with an island shrinking back once its details have gone.
fn following(host: Host, ctx: &Ctx) -> impl Fn() -> Rect + Clone + 'static {
    let ctx = ctx.clone();

    move || host_area(host, &ctx, |widget| widget.layout_rect().get())
}

fn host_area(host: Host, ctx: &Ctx, rect: impl Fn(&WidgetRef) -> Rect) -> Rect {
    match host {
        Host::Shape(id) => rect(&ctx.refs.nodes[&id]),
        Host::Background | Host::None => {
            let bar = rect(&ctx.refs.bar);
            let (y, _) = ctx.position.near_first(0.0, bar.height - ctx.reserved);

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
        nodes: HashMap::new(),
        zones: HashMap::new(),
    };
    for node in bar.nodes() {
        if node.shape || node.is_group() {
            refs.nodes.insert(node.id, create_widget_ref());
        }
    }

    for (&id, placed) in &groups {
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

    let p = bar.padding;
    let nodes = self::places(bar.places(), true, &ctx);
    let nodes = shaped(nodes, &bar.root, true, &ctx)
        .align_self(bar.position.alignment())
        .padding([p.top.px(), p.right.px(), p.bottom.px(), p.left.px()]);

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
    merge: Merge,
    ctx: &Ctx,
    opens_here: impl Fn(&Opened) -> bool + 'static,
    panel: impl Fn(Opened) -> Container + 'static,
) -> Container {
    let (state, groups) = (ctx.state, ctx.groups.clone());

    let open = create_memo(move || state.opened.get().filter(|opened| opens_here(opened)));
    let detached = create_signal(0usize);

    let holder = if merge != Merge::Off {
        merged(holder, merge, move || {
            let panels = if detached.get() > 0 {
                Panels::Detached
            } else {
                Panels::Attached
            };

            Neck::of(panels).reach
        })
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

            panel(opened).out_of_flow(true)
        },
    ))
}

/// Adds to `holder` the details shown, if `shows_here` says they come out of
/// `holder`: a small shape coming out of `host`, one for each list of nodes.
fn hosting_details(
    holder: Container,
    ctx: &Ctx,
    merge: Merge,
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
        move |_| {
            group::split_details(shown, host(), merge == Merge::Borderless, &place)
                .out_of_flow(true)
        },
    ))
}

/// `holder` as what the panels and details of the groups `host` takes come
/// out of.
fn hosts(holder: Container, host: Host, merge: Merge, ctx: &Ctx) -> Container {
    let (place, groups) = (ctx.clone(), ctx.groups.clone());
    let holder = hosting(
        holder,
        merge,
        ctx,
        move |opened| groups[&opened.group].opens_from() == host,
        move |opened| {
            let bordered = merge == Merge::Borderless;

            group::split_off(opened, host, bordered, &place).align_self(place.position.alignment())
        },
    );

    let place = ctx.clone();
    hosting_details(
        holder,
        ctx,
        merge,
        move |_, placed| placed.group.host == host,
        move || host_rect(host, &place),
    )
}

/// How the shapes in a holder merge: not at all; into one outline with a
/// border all round it; or into one outline that keeps only the borders of
/// the shapes that have one, fading them out along the necks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Merge {
    Off,
    Outlined,
    Borderless,
}

impl Merge {
    fn when(merges: bool, merge: Merge) -> Merge {
        if merges { merge } else { Merge::Off }
    }
}

fn merged<M>(holder: Container, merge: Merge, reach: impl IntoSignal<f32, M>) -> Container {
    let holder = holder.merge_children(reach);

    if merge == Merge::Outlined {
        island::border(holder)
    } else {
        holder
    }
}

/// Draws the bar's solid background, from edge to edge. The panels and
/// details of the groups with no shape come out of it.
fn background(ctx: &Ctx) -> Container {
    let merges = ctx
        .groups
        .values()
        .any(|placed| placed.group.host == Host::Background);

    // No border of its own: the panels and details that come out of it have
    // theirs, which fade out along the necks.
    let fill = island::paint(container())
        .width(fill())
        .height(ctx.reserved);

    let layer = container()
        .width(guido::prelude::fill())
        .height(guido::prelude::fill())
        .layout(ZStack::new().align(ctx.position.at(Place::Start)));

    hosts(
        layer,
        Host::Background,
        Merge::when(merges, Merge::Borderless),
        ctx,
    )
    .child(fill)
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
    let shape_ref = ctx.refs.nodes[&id];

    // Not eased: it takes the width of what it holds, which is eased already.
    let shape = island::paint(container())
        .widget_ref(shape_ref)
        .width(if wide { fill() } else { natural() })
        .height(natural())
        .padding([0.0, size::S1.px()])
        .corners(radius::pill(group_height()))
        .child(content);

    let holder = container().layout(ZStack::new().align(ctx.position.alignment()));
    let holder = if wide { holder.width(fill()) } else { holder };

    hosts(holder, Host::Shape(id), Merge::Outlined, ctx)
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

#[cfg(test)]
mod tests {
    use guido::reactive::__internal::{dispose_owner, with_owner};
    use styletto::provide_ui;
    use styletto::ui::{Density, Ui};

    use super::Neck;
    use crate::config::Panels;

    #[test]
    fn attached_panels_and_details_keep_a_neck_and_detached_ones_lose_it() {
        for density in [Density::Compact, Density::Default, Density::Comfortable] {
            let ((), scope) = with_owner(|| {
                provide_ui(Ui::default().with_density(density));

                for panels in [Panels::Attached, Panels::Dropdown, Panels::Morph] {
                    assert!(Neck::of(panels).joined(), "{panels:?} at {density:?}");
                }
                assert!(Neck::details().joined(), "details at {density:?}");
                assert!(
                    !Neck::of(Panels::Detached).joined(),
                    "detached at {density:?}"
                );
            });
            dispose_owner(scope);
        }
    }
}
