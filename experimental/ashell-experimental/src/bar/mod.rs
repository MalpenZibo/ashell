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
use styletto::foundations::tokens::{Space, radius, size};
use styletto::primitives::island;

use crate::config::{Background, Bar, Holds, Host, Node, Padding, Panel, Place, Position};

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

/// How far what comes out of a bar's shapes stands off them, and how far the
/// shapes reach for each other, from the bar's `gap`. Between two edges `gap`
/// apart guido's smooth minimum keeps a neck only while `gap < reach / 2`, as
/// thick as `reach / 4 - gap / 2`, and swells them towards each other until
/// `gap` is `reach`.
#[derive(Clone, Copy, Debug)]
struct Neck {
    /// A panel's gap, a dropdown's too.
    panel: f32,
    /// The details' where they merge: while joined, half a panel's, for a
    /// thicker neck; once apart, a panel's. Alone they take a panel's.
    details: f32,
    /// The merge distance while the gap keeps a neck with it; past that, just
    /// more than the gap, so a shape comes out joined, its neck breaks as it
    /// arrives, and it swells little once apart.
    reach: f32,
}

impl Neck {
    fn of(gap: Space) -> Self {
        let panel = gap.px();
        let joined = panel < merge_distance() / 2.0;

        Self {
            panel,
            details: if joined { panel / 2.0 } else { panel },
            reach: if joined {
                merge_distance()
            } else {
                panel * 4.0 / 3.0
            },
        }
    }

    #[cfg(test)]
    fn joined(self, gap: f32) -> bool {
        gap < self.reach / 2.0
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

    /// Hanging from the far side of the box, the side away from the bar's
    /// edge, at `place` along it: what comes out of the box follows its
    /// edge on every layout, so its own translate carries only its motion.
    fn hung(self, place: Place) -> Alignment {
        let far = match self {
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top,
        };

        far.at(place).to(self.at(place))
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

/// What the details are waiting to do.
///
/// Each event replaces or drops only what is its own: the moves that keep
/// coming on a zone with no details leave alone the close that the leave of
/// another group asked for, and a leave only what was asked for where the
/// pointer was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plan {
    /// Show a zone's details, once the pointer has stayed on it.
    Show((NodeId, usize)),
    /// Close a group's details, the pointer on one of its zones with none.
    Hide(NodeId),
    /// Close the details shown, unless the pointer is back in their group.
    Close,
}

#[derive(Clone, Copy)]
pub struct State {
    pub opened: RwSignal<Option<Opened>>,
    pub details: RwSignal<Option<(NodeId, usize)>>,
    inside: RwSignal<Option<NodeId>>,
    pending: RwSignal<Option<(TimerHandle, Plan)>>,
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

    /// On a zone with details. Its details come once the pointer stays on it.
    pub fn moved_on(&self, zone: (NodeId, usize)) {
        self.entered(zone.0);

        let shown = self.details.get_untracked();
        if shown == Some(zone) {
            self.cancel();
            return;
        }

        let wait = match shown {
            Some((group, _)) if group == zone.0 => ZONE_SWITCH,
            _ => DETAILS_DWELL,
        };
        self.plan(Plan::Show(zone), wait);
    }

    /// On a zone of `group` with no details: those shown there go, and so do
    /// another group's.
    pub fn moved_off(&self, group: NodeId) {
        self.entered(group);

        match self.details.get_untracked() {
            Some((shown, _)) if shown == group => self.plan_once(Plan::Hide(group), ZONE_SWITCH),
            Some(_) => self.plan_once(Plan::Close, DETAILS_CLOSE),
            None => {
                if matches!(self.planned(), Some(Plan::Show(_))) {
                    self.cancel();
                }
            }
        }
    }

    /// Off `zone`: its details no longer come.
    pub fn moved_out(&self, zone: (NodeId, usize)) {
        if self.planned() == Some(Plan::Show(zone)) {
            self.cancel();
            self.settle();
        }
    }

    fn hovered(&self, group: NodeId, inside: bool) {
        if inside {
            self.entered(group);
        } else {
            self.left(group);
        }
    }

    fn entered(&self, group: NodeId) {
        self.inside.set(Some(group));
    }

    fn left(&self, group: NodeId) {
        if self.inside.get_untracked() == Some(group) {
            self.inside.set(None);
        }
        if matches!(self.planned(), Some(Plan::Show((g, _))) if g == group) {
            self.cancel();
        }

        self.settle();
    }

    /// With nothing planned, the details shown close unless the pointer is
    /// in their group.
    fn settle(&self) {
        if self.away() && self.planned().is_none() {
            self.plan(Plan::Close, DETAILS_CLOSE);
        }
    }

    /// Whether details are shown and the pointer is not in their group.
    fn away(&self) -> bool {
        self.details
            .get_untracked()
            .is_some_and(|(group, _)| self.inside.get_untracked() != Some(group))
    }

    fn planned(&self) -> Option<Plan> {
        self.pending.get_untracked().map(|(_, plan)| plan)
    }

    fn plan_once(&self, plan: Plan, wait: Duration) {
        if self.planned() != Some(plan) {
            self.plan(plan, wait);
        }
    }

    fn plan(&self, plan: Plan, wait: Duration) {
        self.cancel();

        let state = *self;
        let timer = set_timeout(wait, move || {
            state.pending.set(None);
            state.carry_out(plan);
        });
        self.pending.set(Some((timer, plan)));
    }

    fn cancel(&self) {
        if let Some((timer, _)) = self.pending.get_untracked() {
            timer.cancel();
            self.pending.set(None);
        }
    }

    fn carry_out(&self, plan: Plan) {
        match plan {
            Plan::Show(zone) => {
                if self.opened.get_untracked().is_none() {
                    self.details.set(Some(zone));
                }
            }
            Plan::Hide(group) => {
                if self
                    .details
                    .get_untracked()
                    .is_some_and(|(g, _)| g == group)
                {
                    self.details.set(None);
                }
            }
            Plan::Close => {
                if self.away() {
                    self.details.set(None);
                }
            }
        }
    }
}

#[derive(Clone)]
struct Ctx {
    state: State,
    refs: Rc<Refs>,
    groups: Rc<HashMap<NodeId, Placed>>,
    position: Position,
    padding: Padding,
    neck: Neck,
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

/// The area of the surface that panels and details come out of, as it is at
/// rest.
fn host_rect(host: Host, ctx: &Ctx) -> Rect {
    let rect = |widget: &WidgetRef| widget.layout_rect().get_untracked();

    match host {
        Host::Shape(id) => at_rest(rect(&ctx.refs.nodes[&id]), ctx.position),
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

/// A shape as tall as at rest, against the bar's edge: the click that opens a
/// panel closes the details it grows with.
fn at_rest(shape: Rect, position: Position) -> Rect {
    let (grown, _) = position.near_first(0.0, shape.height - group_height());

    Rect {
        y: shape.y + grown,
        height: group_height(),
        ..shape
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
        neck: Neck::of(bar.gap),
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
/// Unless `outline` is `Own`, the shapes in `holder` merge into one, reaching
/// as far as the bar's gap says (see `Neck`).
fn hosting(
    holder: Container,
    outline: Outline,
    ctx: &Ctx,
    opens_here: impl Fn(&Opened) -> bool + 'static,
    panel: impl Fn(Opened) -> Container + 'static,
) -> Container {
    let state = ctx.state;
    let open = create_memo(move || state.opened.get().filter(|opened| opens_here(opened)));

    let reach = ctx.neck.reach;
    let holder = match outline {
        Outline::Own => holder,
        Outline::Shared => island::edge(holder.merge_children(reach)),
        Outline::Members => holder.merge_children(reach),
    };

    holder.children(keyed(
        move || open.get().into_iter().collect::<Vec<_>>(),
        |opened| (opened.group, opened.zone, opened.panel),
        move |opened| panel(opened).out_of_flow(true),
    ))
}

/// Adds to `holder` the details shown, if `shows_here` says they come out of
/// `holder`: a small shape coming out of `host`, one for each list of nodes.
fn hosting_details(
    holder: Container,
    outline: Outline,
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
        move |_| group::split_details(shown, host(), outline, &place).out_of_flow(true),
    ))
}

/// `holder` as what the panels and details of the groups `host` takes come
/// out of.
fn hosts(holder: Container, host: Host, outline: Outline, ctx: &Ctx) -> Container {
    let (place, groups) = (ctx.clone(), ctx.groups.clone());
    let holder = hosting(
        holder,
        outline,
        ctx,
        move |opened| groups[&opened.group].opens_from() == host,
        move |opened| group::split_off(opened, host, outline, &place),
    );

    let place = ctx.clone();
    hosting_details(
        holder,
        outline,
        ctx,
        move |_, placed| placed.group.host == host,
        move || host_rect(host, &place),
    )
}

/// How the shapes in a holder are outlined.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outline {
    /// Each its own: they don't merge.
    Own,
    /// One, round the shapes merged: its border and its shadow, which the
    /// shapes then cast none of.
    Shared,
    /// The borders of the shapes merged that have one, fading out along the
    /// necks.
    Members,
}

impl Outline {
    fn when(merges: bool, outline: Outline) -> Outline {
        if merges { outline } else { Outline::Own }
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
    let paint = island::paint(container())
        .width(fill())
        .height(ctx.reserved);

    // As tall as the strip, so what comes out of it hangs from its edge, and
    // with the surface's room for it.
    let start = ctx.position.at(Place::Start);
    let layer = container()
        .width(fill())
        .align_self(start)
        .layout(ZStack::new().align(start));

    hosts(
        layer,
        Host::Background,
        Outline::when(merges, Outline::Members),
        ctx,
    )
    .child(paint)
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

    hosts(holder, Host::Shape(id), Outline::Shared, ctx).child(shape)
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
    use guido::prelude::Rect;
    use guido::reactive::__internal::{dispose_owner, with_owner};
    use styletto::foundations::tokens::size;
    use styletto::provide_ui;
    use styletto::ui::{Density, Ui};

    use super::{Neck, at_rest, group_height};
    use crate::config::Position;

    #[test]
    fn a_shape_at_rest_keeps_the_end_against_the_bar() {
        let ((), scope) = with_owner(|| {
            provide_ui(Ui::default());
            let grown = Rect {
                x: 5.0,
                y: 10.0,
                width: 200.0,
                height: group_height() + 40.0,
            };

            let top = at_rest(grown, Position::Top);
            assert_eq!(
                (top.x, top.y, top.width, top.height),
                (5.0, 10.0, 200.0, group_height())
            );

            let bottom = at_rest(grown, Position::Bottom);
            assert_eq!(bottom.y, 50.0);
            assert_eq!(bottom.height, group_height());
        });
        dispose_owner(scope);
    }

    #[test]
    fn a_gap_below_s4_keeps_a_neck_and_one_from_s4_breaks_it() {
        for density in [Density::Compact, Density::Default, Density::Comfortable] {
            let ((), scope) = with_owner(|| {
                provide_ui(Ui::default().with_density(density));

                for gap in [size::S0, size::S0_5, size::S1, size::S2, size::S3] {
                    let neck = Neck::of(gap);
                    assert!(neck.joined(neck.panel), "{gap:?} at {density:?}");
                    assert!(neck.joined(neck.details), "{gap:?} at {density:?}");
                }
                for gap in [size::S4, size::S6, size::S8, size::S10] {
                    let neck = Neck::of(gap);
                    assert!(!neck.joined(neck.panel), "{gap:?} at {density:?}");
                    assert!(!neck.joined(neck.details), "{gap:?} at {density:?}");
                }
            });
            dispose_owner(scope);
        }
    }
}
