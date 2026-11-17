use std::rc::Rc;

use guido::prelude::*;
use styletto::foundations::motion::arriving;
use styletto::foundations::tokens::{radius, size, stroke};
use styletto::primitives::island::{self, island};
use styletto::{centered_row, foundations::motion, ui};

use super::{
    Ctx, GROUP, Host, Merge, Neck, NodeId, Opened, Refs, State, group_height, host_rect, hosting,
    hosting_details, merge_distance, merged_shadow, placeholder, under_zone,
};
use crate::config::{Node, Open, Panel, Panels, Place, Position};

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
    let groups = ctx.groups.clone();

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
        .on_hover(move |inside| {
            if inside {
                state.entered(id);
            } else {
                state.left(id);
            }
        });

    // As wide as the group: a split panel overflows it, so the neighbours do
    // not move, and merges with the group as its sibling.
    let holder = container()
        .width(move || shape_ref.layout_rect().get().width)
        .layout(ZStack::new().align(ctx.position.alignment()))
        .opacity(
            (move || {
                let away = state.opened.get().is_some_and(|opened| {
                    let other = &groups[&opened.group];

                    other.group.panels == Panels::Morph && opened.group != id && other.list == list
                });

                if away { 0.0 } else { 1.0 }
            })
            .transition(motion::hover()),
        );
    let place = ctx.clone();

    let holder = hosting(
        holder,
        Merge::when(merge, Merge::Outlined),
        ctx,
        move |opened| opened.group == id && opens_here,
        move |opened| {
            let group = shape_ref.layout_rect().get_untracked();
            if merge {
                return split_off(opened, group, false, &place);
            }

            // From the edge of what hosts it, past the group's own if that
            // reaches further.
            let from = host_rect(host, &place);
            let past = match place.position {
                Position::Top => from.y + from.height - (group.y + group.height),
                Position::Bottom => group.y - from.y,
            };
            let edge = group.height + past.max(0.0);
            let x = under_zone(opened, group.x + group.width / 2.0, &place);
            let side = place.position.side();
            let gap = Neck::of(panels).gap;

            dropdown(
                opened.panel,
                Translate::new(x, side * (edge + gap)),
                Translate::new(x, side * edge),
            )
        },
    );

    let holder = hosting_details(
        holder,
        ctx,
        Merge::Off,
        move |group, placed| group == id && placed.group.host == Host::None,
        move || shape_ref.layout_rect().get_untracked(),
    )
    .child(shape);

    if casts {
        holder.shadow(merged_shadow())
    } else {
        holder
    }
}

/// `bordered`: with a border of its own, for a holder that draws none.
pub(super) fn split_off(opened: Opened, host: Rect, bordered: bool, ctx: &Ctx) -> Container {
    let middle = host.x + host.width / 2.0;
    let x = under_zone(opened, middle, ctx);
    let side = ctx.position.side();
    let group = ctx.refs.nodes[&opened.group].layout_rect().get_untracked();

    let (left, width) = span(group.x, group.width);
    let from = Translate::new(left + width / 2.0 - middle, side * (host.height / 2.0));
    let away = host.height + Neck::of(ctx.groups[&opened.group].group.panels).gap;

    let shape = panel_shape(opened.panel, Translate::new(x, side * away), from, width);

    with_border(shape, bordered)
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
    bordered: bool,
    ctx: &Ctx,
) -> Container {
    let enter = || motion::easing(350.0).delay(150.0);
    let leave = || motion::easing(100.0);
    let (state, side) = (ctx.state, ctx.position.side());
    let (refs, groups) = (ctx.refs.clone(), ctx.groups.clone());

    let Some(first) = state.details.get_untracked() else {
        return container();
    };
    let place = groups[&first.0].place();
    let last = create_signal(first);
    create_effect(move || {
        if let Some(zone) = shown.get() {
            last.set(zone);
        }
    });

    let align = ctx.position.at(place);
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
    let away = side * (host.height + Neck::details().gap);
    let inside = side * (host.height / 2.0);
    let sliver = move |zone| {
        let under = zone_rect(zone);
        Length::exact(span(under.x, under.width).1)
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
                let details = placeholder::details(&groups[&group].group.zones()[zone].details)
                    .height(group_height())
                    .padding([0.0, size::S1.px()]);

                arriving(details).width(natural())
            },
        ));

    with_border(shape_fill(container()), bordered)
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
        .on_hover(move |inside| {
            let (group, _) = last.get_untracked();
            if inside {
                state.entered(group);
            } else {
                state.left(group);
            }
        })
        // Not clipped: a shape that clips would not merge.
        .child(row)
}

fn shape(group: &Rc<Node>, shown: Memo<Shown>, ctx: &Ctx) -> Container {
    let (id, state, position) = (group.id, ctx.state, ctx.position);
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

                    let rest = rest(&group, state, &refs);
                    let details =
                        move || open.get().then(|| details(&group, details_here, position));
                    let column =
                        container().layout(Flex::column().cross_alignment(CrossAlignment::Center));

                    match position {
                        Position::Top => column.child(rest).child(details),
                        Position::Bottom => column.child(details).child(rest),
                    }
                }
                Shown::Panel(panel) => placeholder::panel(panel),
            }
        }
    };

    if group.shape {
        island(shown, content, move || match shown.get() {
            Shown::Rest if details_here.get().is_none() => radius::pill(group_height()),
            _ => radius::LARGE.corners(),
        })
        .width(natural().transition(motion::spring()))
        .backdrop_blur(blur_behind())
        .layout(ZStack::new().align(position.alignment()))
    } else {
        // Its zone for the compositor only: drawn here, the blur would show
        // as a box on what holds the group.
        container()
            .width(natural())
            .backdrop_blur(BackdropBlur::new(0.0).sources(BackdropSources::COMPOSITOR))
            .child(move || content(shown.get()))
    }
}

fn rest(group: &Node, state: State, refs: &Refs) -> Container {
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
                    let mut pill =
                        container()
                            .widget_ref(refs.zones[&(id, index)])
                            .child(placeholder::zone(&zone.name, move || {
                                if let Open::Panel(panel) = open {
                                    state.toggle(Opened {
                                        group: id,
                                        zone: index,
                                        panel,
                                    });
                                }
                            }));

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
                                    state.moved();
                                }
                            });
                    }

                    pill
                })
                .collect::<Vec<_>>(),
        )
}

fn details(group: &Rc<Node>, zone: Memo<Option<usize>>, position: Position) -> Container {
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
        zone.get()
            .map(|zone| swapped_in(placeholder::details(&group.zones()[zone].details)))
    })
}

fn coming_and_going<T>(
    enter: Transition,
    leave: Transition,
) -> impl Fn(Segment<T>) -> Transition + 'static {
    move |s| {
        if s.is_exit() {
            leave.clone()
        } else {
            enter.clone()
        }
    }
}

/// Comes in and goes out fading, coming into focus and sliding from `away`.
fn rising(content: Container, enter: Transition, leave: Transition, away: Translate) -> Container {
    content
        .opacity(
            1.0.transition(coming_and_going(enter.clone(), leave.clone()))
                .entering_from(0.0)
                .exiting_to(0.0),
        )
        .blur(
            0.0.transition(coming_and_going(enter.clone(), leave.clone()))
                .entering_from(3.0)
                .exiting_to(3.0),
        )
        .translate(
            Translate::NONE
                .transition(coming_and_going(enter, leave))
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
/// the merge reaches, since nearer an end the merge still swells the shape
/// there when the panel is let go.
fn span(left: f32, width: f32) -> (f32, f32) {
    let inset = merge_distance().min(width / 2.0);

    (left + inset, width - 2.0 * inset)
}

fn panel_shape(panel: Panel, to: Translate, from: Translate, width: f32) -> Container {
    let content = placeholder::panel(panel).opacity(
        1.0.transition(coming_and_going(
            motion::easing(200.0).delay(260.0),
            motion::easing(100.0),
        ))
        .entering_from(0.0)
        .exiting_to(0.0),
    );

    // An exact width would be cut to what holds it.
    shape_fill(container())
        .width(
            natural()
                .at_most(placeholder::panel_width())
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

fn dropdown(panel: Panel, to: Translate, rim: Translate) -> Container {
    shape_fill(container())
        .width(natural().at_most(placeholder::panel_width()))
        .height(
            at_least(0.0)
                .transition(dropping())
                .entering_from(Length::exact(0.0))
                .exiting_to(Length::exact(0.0)),
        )
        .overflow(Overflow::Hidden)
        .corners(radius::LARGE.corners())
        .border(stroke::HAIRLINE, ui().colors.line)
        .shadow(island::shadow(Color::TRANSPARENT))
        .takes_input(true)
        .translate(to.transition(dropping()).entering_from(rim).exiting_to(rim))
        .child(placeholder::panel(panel))
}

fn dropping<T>() -> impl Fn(Segment<T>) -> Transition + 'static {
    coming_and_going(motion::spring(), motion::easing(DROP_MS))
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
    coming_and_going(
        motion::spring(),
        motion::easing(RETRACT_MS).delay(CONTENT_OUT_MS),
    )
}

fn with_border(shape: Container, bordered: bool) -> Container {
    if bordered {
        shape.border(stroke::HAIRLINE, ui().colors.line)
    } else {
        shape
    }
}

pub(super) fn shape_fill(container: Container) -> Container {
    let t = ui();

    container
        .background(t.colors.island)
        .backdrop_blur(blur_behind())
}

/// The blur behind what the bar draws. The compositor is asked for it even
/// with a solid material: a compositor rule blurs all of a surface that
/// declares no blur.
pub(super) fn blur_behind() -> BackdropBlur {
    let radius = ui().material.blur();
    let sources = if radius > 0.0 {
        BackdropSources::all()
    } else {
        BackdropSources::COMPOSITOR
    };

    BackdropBlur::new(radius).sources(sources)
}
