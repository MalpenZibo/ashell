use std::rc::Rc;

use guido::prelude::*;
use styletto::foundations::control::Size;
use styletto::foundations::motion::{arriving, fading};
use styletto::foundations::tokens::{radius, size};
use styletto::primitives::island::{self, island};
use styletto::{centered_row, foundations::motion};

use super::items::{self, Opens};
use super::{
    Ctx, GROUP, Host, NodeId, Opened, Outline, Placed, Refs, State, at_rest, group_height,
    host_rect, hosting, hosting_details, panel_view, panel_width, placeholder, under_zone,
};
use crate::config::{Node, Open, Panel, Panels, Place, Position};
use crate::modules::Modules;

/// Opens a panel under a zone of a group.
fn opens(state: State, group: NodeId, zone: usize) -> Opens {
    Rc::new(move |panel| state.toggle(Opened { group, zone, panel }))
}

const CONTENT_OUT_MS: f32 = 120.0;
const DROP_MS: f32 = 220.0;
const RETRACT_MS: f32 = 420.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Shown {
    Rest,
    Panel(Panel),
}

pub(super) fn view(id: NodeId, ctx: &Ctx) -> Container {
    let state = ctx.state;
    let placed = &ctx.groups[&id];
    let (host, list) = (placed.group.host, placed.list);

    let group = placed.group.clone();
    let panels = group.panels;
    let merge = panels.merges();
    let morph = panels == Panels::Morph;
    let opens_here = placed.opens_from() == Host::Shape(id) && !morph;

    let shown = create_memo(move || match state.opened.get() {
        Some(opened) if opened.group == id && morph => Shown::Panel(opened.panel),
        _ => Shown::Rest,
    });

    let shape_ref = ctx.refs.nodes[&id];

    // Merging with what comes out of it, the island's shadow is the merged
    // outline's.
    let casts = merge && group.shape;
    let shape = shape(&group, shown, ctx);
    let shape = if casts {
        shape.shadow(Shadows::none())
    } else {
        shape
    };
    let shape = shape
        .widget_ref(shape_ref)
        .takes_input(true)
        .on_hover(move |inside| state.hovered(id, inside));

    // As wide as the group: what comes out of it is out of flow, so a panel
    // overflows it without moving the neighbours, and merges with the group
    // as its sibling.
    let holder = container().layout(ZStack::new().align(ctx.position.alignment()));
    // Out of the way of a group in its list whose panel takes its place.
    let morph_beside = move |other: &Placed| {
        other.group.panels == Panels::Morph && other.group.id != id && other.list == list
    };
    let holder = if ctx.groups.values().any(morph_beside) {
        let groups = ctx.groups.clone();
        let away = create_memo(move || {
            state
                .opened
                .get()
                .is_some_and(|opened| morph_beside(&groups[&opened.group]))
        });

        holder.opacity((move || if away.get() { 0.0 } else { 1.0 }).transition(motion::hover()))
    } else {
        holder
    };
    let place = ctx.clone();
    let outline = Outline::when(casts, Outline::Shared);

    let holder = hosting(
        holder,
        outline,
        ctx,
        move |opened| opened.group == id && opens_here,
        move |opened| {
            if merge {
                return split_off(opened, Host::Shape(id), outline, &place);
            }

            let group = at_rest(shape_ref.layout_rect().get_untracked(), place.position);
            let x = under_zone(opened, group.x + group.width / 2.0, &place);
            let (side, gap) = (place.position.side(), place.neck.panel);

            // From the edge of what hosts it, past the group's own if that
            // reaches further.
            let from = host_rect(host, &place);
            let past = match place.position {
                Position::Top => from.y + from.height - (group.y + group.height),
                Position::Bottom => group.y - from.y,
            };
            let edge = past.max(0.0);

            dropdown(
                opened.panel,
                place.modules,
                Translate::new(x, side * (edge + gap)),
                Translate::new(x, side * edge),
            )
            .align_self(place.position.hung(Place::Center))
        },
    );

    hosting_details(
        holder,
        Outline::Own,
        ctx,
        move |group, placed| group == id && placed.group.host == Host::None,
        move || shape_ref.layout_rect().get_untracked(),
    )
    .child(shape)
}

/// `outline`: how the holder it comes out in outlines what merges in it,
/// which says what it draws of its edge (see `rimmed`).
pub(super) fn split_off(opened: Opened, host: Host, outline: Outline, ctx: &Ctx) -> Container {
    let rect = host_rect(host, ctx);
    let middle = rect.x + rect.width / 2.0;
    let x = under_zone(opened, middle, ctx);
    let side = ctx.position.side();
    let group = ctx.refs.nodes[&opened.group].layout_rect().get_untracked();
    let gap = ctx.neck.panel;

    // Half the host at rest back from the edge it hangs from: its middle once
    // the details it may have grown with are back.
    let (left, width) = span(group.x, group.width, ctx.neck.reach);
    let from = Translate::new(left + width / 2.0 - middle, -side * (rect.height / 2.0));
    let to = Translate::new(x, side * gap);

    rimmed(
        panel_shape(opened.panel, ctx.modules, to, from, width),
        outline,
    )
    .align_self(ctx.position.hung(Place::Center))
}

/// The details of the groups in one list of nodes that have no shape of
/// their own, as one small shape that splits off `host`, what holds them,
/// from under the zone shown, and goes back into it as a panel does. Moving
/// to another zone of the list, it slides there and its row changes, as it
/// does in a shape. Its edge is on the zone's in `start` and `end`, so it
/// stays on the screen whatever its width, and it is centred on it in
/// `center`.
pub(super) fn split_details(
    shown: Memo<Option<(NodeId, usize)>>,
    host: Rect,
    outline: Outline,
    ctx: &Ctx,
) -> Container {
    let enter = || motion::easing(350.0).delay(150.0);
    let leave = || motion::easing(100.0);
    let (state, side, modules) = (ctx.state, ctx.position.side(), ctx.modules);
    let (refs, groups) = (ctx.refs.clone(), ctx.groups.clone());

    let Some(first) = state.details.get_untracked() else {
        return container();
    };
    let list = groups[&first.0].list;
    let place = list.1;
    let last = create_signal(first);
    // Only the zones of its list: leaving, it stays where it was while the
    // details move to another list.
    create_effect({
        let groups = groups.clone();
        move || {
            if let Some(zone) = shown.get()
                && groups[&zone.0].list == list
            {
                last.set(zone);
            }
        }
    });

    let align = ctx.position.hung(place);
    let zone_rect = move |zone: (NodeId, usize)| refs.zones[&zone].layout_rect().get_untracked();
    let x = {
        let zone_rect = zone_rect.clone();
        move |zone| {
            let under = zone_rect(zone);
            match place {
                Place::Start => under.x - host.x,
                Place::End => under.x + under.width - (host.x + host.width),
                Place::Center => under.x + under.width / 2.0 - (host.x + host.width / 2.0),
            }
        }
    };
    // Alone, with no neck to make thick, as far off as a panel.
    let gap = match outline {
        Outline::Own => ctx.neck.panel,
        Outline::Shared | Outline::Members => ctx.neck.details,
    };
    let (away, reach) = (side * gap, ctx.neck.reach);
    let inside = -side * (host.height / 2.0);
    let sliver = move |zone| {
        let under = zone_rect(zone);
        Length::exact(span(under.x, under.width, reach).1)
    };
    let toward = Translate::new(0.0, -side * size::S1.px());
    let start = Translate::new(x(first), inside);
    let back = x.clone();

    let row = rising(container().width(natural()), enter(), leave(), toward)
        .layout(ZStack::new().align(Alignment::CENTER))
        .children(keyed(
            move || [last.get()],
            |&shown| shown,
            move |(group, zone)| {
                let shown = &groups[&group].group.zones()[zone];
                let details = items::details(
                    &shown.details,
                    shown.open,
                    modules,
                    opens(state, group, zone),
                )
                .height(group_height())
                // As far in as a zone's text is in a group's row, past the
                // pill ends.
                .padding([0.0, size::S1.px() + Size::Sm.inline()]);

                arriving(details).width(natural())
            },
        ));

    rimmed(island::paint(container()), outline)
        .width(
            at_least(0.0)
                .transition(sliding())
                .entering_from(sliver(first))
                .exiting_to(move || sliver(last.get())),
        )
        .height(
            at_least(0.0)
                .transition(sliding())
                .entering_from(Length::exact(0.0))
                .exiting_to(Length::exact(0.0)),
        )
        .corners(radius::pill(group_height()))
        .layout(ZStack::new().align(Alignment::CENTER))
        .takes_input(true)
        .translate(
            (move || Translate::new(x(last.get()), away))
                .transition(sliding())
                .entering_from(start)
                .exiting_to(move || Translate::new(back(last.get()), inside)),
        )
        .align_self(align)
        .on_hover(move |inside| state.hovered(last.get_untracked().0, inside))
        // Not clipped: a shape that clips would not merge.
        .child(row)
}

fn shape(group: &Rc<Node>, shown: Memo<Shown>, ctx: &Ctx) -> Container {
    let (id, state, position, modules) = (group.id, ctx.state, ctx.position, ctx.modules);
    let refs = ctx.refs.clone();

    let details_here = create_memo(move || {
        state
            .details
            .get()
            .and_then(|(g, zone)| (g == id).then_some(zone))
    });

    let content = {
        let group = group.clone();
        move |which: Shown| -> Container {
            match which {
                Shown::Rest => {
                    let inside = group.shape;
                    let open = create_memo(move || inside && details_here.get().is_some());
                    let group = group.clone();

                    let rest = rest(&group, state, &refs, modules);
                    let details = move || {
                        open.get()
                            .then(|| details(&group, state, details_here, position, modules))
                    };
                    let column =
                        container().layout(Flex::column().cross_alignment(CrossAlignment::Center));

                    match position {
                        Position::Top => column.child(rest).child(details),
                        Position::Bottom => column.child(details).child(rest),
                    }
                }
                Shown::Panel(panel) => panel_view(panel, modules),
            }
        }
    };

    if group.shape {
        island(shown, content, move || match shown.get() {
            Shown::Rest if details_here.get().is_none() => radius::pill(group_height()),
            _ => radius::LARGE.corners(),
        })
        .width(natural().transition(motion::spring()))
        .layout(ZStack::new().align(position.alignment()))
    } else {
        // Nothing behind the zones, not even a blur: what holds the group
        // draws it, and with nothing round the group there is none.
        container()
            .width(natural())
            .child(move || content(shown.get()))
    }
}

fn rest(group: &Node, state: State, refs: &Refs, modules: Modules) -> Container {
    let (id, hover) = (group.id, group.hover);
    let height = if group.shape {
        island::inner(GROUP)
    } else {
        group_height()
    };

    container()
        .height(height)
        .padding([0.0, size::S1.px()])
        .layout(centered_row(size::S0_5.px()))
        .children(
            group
                .zones()
                .iter()
                .enumerate()
                .map(|(index, zone)| {
                    let (open, has_details) = (zone.open, !zone.details.is_empty());
                    let click = move || {
                        if let Open::Panel(panel) = open {
                            state.toggle(Opened {
                                group: id,
                                zone: index,
                                panel,
                            });
                        }
                    };

                    let content =
                        items::zone(&zone.items, open, modules, opens(state, id, index), click)
                            .unwrap_or_else(|| {
                                container().child(placeholder::zone(&zone.name, click))
                            });

                    let mut pill = container()
                        .widget_ref(refs.zones[&(id, index)])
                        .child(content);

                    if hover {
                        pill = pill
                            .on_pointer_move(move |_, _| {
                                if has_details {
                                    state.moved_on((id, index));
                                } else {
                                    state.moved_off(id);
                                }
                            })
                            .on_hover(move |inside| {
                                if !inside {
                                    state.moved_out();
                                }
                            });
                    }

                    pill
                })
                .collect::<Vec<_>>(),
        )
}

fn details(
    group: &Rc<Node>,
    state: State,
    zone: Memo<Option<usize>>,
    position: Position,
    modules: Modules,
) -> Container {
    let id = group.id;
    let enter = || motion::easing(350.0);
    let leave = || motion::easing(150.0);
    let group = group.clone();
    let away = Translate::new(0.0, -position.side() * size::S1.px());
    let s2 = size::S2.px();
    let (top, bottom) = position.near_first(0.0, s2);
    let padding = [top, s2, bottom, s2];

    let row = container()
        .padding(padding)
        .height(
            at_least(0.0)
                .transition(leave())
                .exiting_to(Length::exact(0.0)),
        )
        .overflow(Overflow::Hidden);

    rising(row, enter(), leave(), away).child(move || {
        zone.get().map(|index| {
            let zone = &group.zones()[index];

            swapped_in(items::details(
                &zone.details,
                zone.open,
                modules,
                opens(state, id, index),
            ))
        })
    })
}

/// Comes in and goes out fading, coming into focus and sliding from `away`.
fn rising(content: Container, enter: Transition, leave: Transition, away: Translate) -> Container {
    content
        .opacity(
            1.0.transition(fading(enter.clone(), leave.clone()))
                .entering_from(0.0)
                .exiting_to(0.0),
        )
        .blur(
            0.0.transition(fading(enter.clone(), leave.clone()))
                .entering_from(3.0)
                .exiting_to(3.0),
        )
        .translate(
            Translate::NONE
                .transition(fading(enter, leave))
                .entering_from(away)
                .exiting_to(away),
        )
}

fn swapped_in(content: Container) -> Container {
    let swap = || motion::easing(320.0);

    content
        .opacity(1.0.transition(swap()).entering_from(0.0))
        .blur(0.0.transition(swap()).entering_from(3.0))
        .translate(
            Translate::NONE
                .transition(swap())
                .entering_from(Translate::new(0.0, size::S0_5.px())),
        )
}

/// The sliver a shape comes out as and goes back as: its ends as far in as
/// the merge `reach`es, since nearer an end the merge still swells the shape
/// there when the panel is let go.
fn span(left: f32, width: f32, reach: f32) -> (f32, f32) {
    let inset = reach.min(width / 2.0);

    (left + inset, width - 2.0 * inset)
}

fn panel_shape(
    panel: Panel,
    modules: Modules,
    to: Translate,
    from: Translate,
    width: f32,
) -> Container {
    let content = panel_view(panel, modules).opacity(
        1.0.transition(fading(
            motion::easing(200.0).delay(260.0),
            motion::easing(100.0),
        ))
        .entering_from(0.0)
        .exiting_to(0.0),
    );

    // An exact width would be cut to what holds it.
    island::paint(container())
        .width(
            natural()
                .at_most(panel_width(panel))
                .transition(retracting())
                .entering_from(natural().at_most(width))
                .exiting_to(natural().at_most(width)),
        )
        .height(
            at_least(0.0)
                .transition(retracting())
                .entering_from(Length::exact(0.0))
                .exiting_to(Length::exact(0.0)),
        )
        .corners(radius::LARGE.corners())
        .takes_input(true)
        .translate(
            to.transition(retracting())
                .entering_from(from)
                .exiting_to(from),
        )
        // Not clipped: a shape that clips would not merge.
        .child(content)
}

fn dropdown(panel: Panel, modules: Modules, to: Translate, rim: Translate) -> Container {
    island::edge(island::paint(container()))
        .width(natural().at_most(panel_width(panel)))
        .height(
            at_least(0.0)
                .transition(dropping())
                .entering_from(Length::exact(0.0))
                .exiting_to(Length::exact(0.0)),
        )
        .overflow(Overflow::Hidden)
        .corners(radius::LARGE.corners())
        .takes_input(true)
        .translate(to.transition(dropping()).entering_from(rim).exiting_to(rim))
        .child(panel_view(panel, modules))
}

fn dropping<T>() -> impl Fn(Segment<T>) -> Transition + 'static {
    fading(motion::spring(), motion::easing(DROP_MS))
}

/// Out as a spring, back with an easing after the row has faded, and from one
/// zone to the next with an easing that does not overshoot.
fn sliding<T>() -> impl Fn(Segment<T>) -> Transition + 'static {
    move |s| {
        if s.is_retarget() {
            motion::easing(250.0)
        } else if s.is_exit() {
            motion::easing(250.0).delay(80.0)
        } else {
            motion::spring()
        }
    }
}

fn retracting<T>() -> impl Fn(Segment<T>) -> Transition + 'static {
    fading(
        motion::spring(),
        motion::easing(RETRACT_MS).delay(CONTENT_OUT_MS),
    )
}

/// A shape that comes out of a holder: on its own, with the island's border
/// and shadow; merged into an outline the holder draws, with neither; merged
/// into one that keeps the shapes' borders, with its border.
fn rimmed(shape: Container, outline: Outline) -> Container {
    match outline {
        Outline::Own => island::edge(shape),
        Outline::Shared => shape,
        Outline::Members => island::border(shape),
    }
}
