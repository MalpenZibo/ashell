+++
title = "Getting Started"
sort_by = "weight"
template = "docs/section.html"
page_template = "docs/page.html"

[extra]
sidebar = "Introduction"
+++

ashell-experimental is a bar described by its
configuration, built on [GUIdo](https://github.com/MalpenZibo/guido), a
GPU-drawn UI toolkit for Wayland shells, and on
[styletto](https://github.com/MalpenZibo/styletto), the design system that
gives it its themes and components.

One grammar gives both today's ashell, islands along a bar with menus that
drop down, and an island that turns into its panels, and every middle ground
in between: they are two configuration files, not two modes.

{% <note kind="warning" title="Experimental"> %}
There are no releases: it is built from `main` and changes with it. The
layout, the islands, the panels and the details are in place; the time, the
weather, the calendar, the batteries, the power profile, Bluetooth, the idle
inhibitor, the brightness and the volume have their data, and the settings
panel has the battery, the power buttons, the sliders, the toggles, the
outputs and inputs and the Bluetooth devices. The other zones and panels
still show placeholders sized like the real thing. The
[stable ashell](https://malpenzibo.github.io/ashell/) is the one to use every
day.
{% </note> %}

## In a few lines

```toml
[bar.main]
shape   = true       # one island as long as the bar
padding = "s2"       # floating off the screen's edges
panels  = "merge"    # panels split off it, joined by a neck
hover   = true       # zones show their details on hover

start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
```

- A bar has three places, `start`, `center` and `end`, holding
  [nodes](@/docs/rolling/configuration/layout.md): a group of zones, a row of
  nodes, or nodes placed again.
- [`shape`](@/docs/rolling/configuration/islands.md) says which nodes have an
  island round them, and `background` whether the bar has a flat fill.
- [`panels`](@/docs/rolling/configuration/panels.md) says how a group opens
  its panels, and `hover` whether its zones show details.
- [Monitors](@/docs/rolling/configuration/bars.md#monitors) say which bars
  they have.

## Next steps

- [Install it](@/docs/rolling/installation.md) from source.
- Start from one of the [examples](@/docs/rolling/examples.md).
- [Configure it](@/docs/rolling/configuration/_index.md): an empty file is a
  complete bar, so write only what differs.
