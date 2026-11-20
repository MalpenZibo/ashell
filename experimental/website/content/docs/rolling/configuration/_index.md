+++
title = "Configuration"
sort_by = "weight"
weight = 10
template = "docs/section.html"
page_template = "docs/page.html"

[extra]
sidebar = "Overview"
group = "Configuration"
+++

The configuration is read from:

```bash
~/.config/ashell/ashell-experimental.toml
```

or `$XDG_CONFIG_HOME/ashell/ashell-experimental.toml` when `XDG_CONFIG_HOME`
is set. It is a [TOML](https://toml.io/en/) file, and an optional one: an
empty or missing file is a complete bar, so write only what differs.

{% <note kind="tip" title="Reloaded live"> %}
The file's folder is watched: when the file is written, or saved over by
renaming, the bars are made again with the new configuration. A save that
changes nothing does nothing. Deleting the file, or moving it away, gives
the default bar back. A symlinked file is followed to where it points, and
to the new file when the link is pointed elsewhere.
{% </note> %}

A file that does not parse is logged and ignored: the bars keep the last
good configuration, or show the default bar at startup. Keys that are not
known in the layout tables (`[bar.*]`, the nodes, `[zone.*]`,
`[monitor.*]`), and values that fall back to a default, are logged as
warnings; an item's other keys are kept as its options. Run with
`RUST_LOG=info` to see each reload.

## Three levels

How it looks, what is there and where, and how it works never mix in the same
table:

| Table | What it sets |
| ----- | ------------ |
| [`[appearance]`](@/docs/rolling/configuration/appearance.md) | Colours, fonts, density, corners, material and motion. |
| [`[bar.<name>]`](@/docs/rolling/configuration/bars.md) | A bar: its edge, its content, how it draws and opens. |
| [`[monitor."<monitor>"]`](@/docs/rolling/configuration/bars.md#monitors) | Which bars a monitor has. |
| [`[zone.<name>]`](@/docs/rolling/configuration/zones.md) | A zone's items, details and click. |
| [`[tempo]`](@/docs/rolling/configuration/tempo.md) | The time zones, and where the weather is for. |

## The levels of a bar

```text
bar                  an edge of the screen: where, and how things open
└── node             the bar's content is one node, the root
      ├── node       a group: zones side by side
      ├── node       a row: nodes side by side
      └── node       placed: nodes at the start, the centre and the end
            └── zone     a part of the bar with interactions
                  └── item   one piece of information
```

- [Layout](@/docs/rolling/configuration/layout.md): nodes, and how to write
  them short or long.
- [Islands and background](@/docs/rolling/configuration/islands.md): what is
  drawn behind the zones.
- [Panels and details](@/docs/rolling/configuration/panels.md): what a click
  and a pointer resting on a zone do.
- [Zones and items](@/docs/rolling/configuration/zones.md): the built-in
  zones, and how to make your own.
- [Tempo](@/docs/rolling/configuration/tempo.md): the time, the weather and
  the calendar.

## Options flow down

`shape`, `hover` and `panels` written on an outer level are the default for
everything inside it; written on an inner level they apply there and below:

```toml
[bar.main]
hover  = true        # every zone shows its details...
panels = "merge"     # ...and every group's panels split off

end = [
  { zones = ["tray"], panels = "dropdown" },  # but this group's drop down
  ["clock", "status"],
]
```
