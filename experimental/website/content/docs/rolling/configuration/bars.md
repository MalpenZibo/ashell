+++
title = "Bars and monitors"
weight = 1
+++

Every bar has a name, `[bar.<name>]`, which the monitors use to say which
bars they have. A file with none has one, `main`, with the defaults.

```toml
[bar.main]
position   = "top"
background = "transparent"
shape      = true
padding    = "s2"
hover      = true
panels     = "attached"

start  = ["workspaces"]
center = [["window", "clock"]]
end    = [["privacy", "status"]]
```

## `[bar.<name>]`

| Key | Values | Default | What it sets |
| --- | ------ | ------- | ------------ |
| `position` | `top`, `bottom` | `top` | The edge of the screen. |
| `start`, `center`, `end` | nodes | `[]` | What the bar holds at each place: see [Layout](@/docs/rolling/configuration/layout.md). |
| `shape` | `true`, `false` | `false` | One island as long as the bar, and the default for the nodes inside: see [Islands](@/docs/rolling/configuration/islands.md). |
| `background` | `transparent`, `solid` | `transparent` | A flat fill from edge to edge, in the island's colour and material. |
| `padding` | space steps | `s2` | Between the screen's edges and the outermost shapes. |
| `hover` | `true`, `false` | `false` | Zones show their details while the pointer rests on them. |
| `panels` | `dropdown`, `attached`, `detached`, `morph` | `dropdown` | How groups open their panels: see [Panels](@/docs/rolling/configuration/panels.md). |

`shape`, `hover` and `panels` are defaults for every node of the bar, which
can say otherwise for itself.

### `padding`

A step of styletto's space scale, written as CSS writes a padding: one for
every side, `[vertical, horizontal]`, or `[top, right, bottom, left]`.

| Step | `s0` | `s0_5` | `s1` | `s2` | `s3` | `s4` | `s6` | `s8` | `s10` |
| ---- | ---- | ------ | ---- | ---- | ---- | ---- | ---- | ---- | ----- |
| px | 0 | 2 | 4 | 8 | 12 | 16 | 24 | 32 | 40 |

The pixels are those of the default density: up to `s8` the steps follow
`[appearance] density`. `s0` puts the shapes against the screen's edges. The
background always runs from edge to edge; the padding moves only what is
drawn on it.

## Monitors

A bar is a copy, one to one, on every monitor that has it, and a monitor has
at most one bar per edge. `[monitor."<monitor>"]` gives a monitor its bars:

```toml
[bar.main]
center = ["clock"]

[bar.media]
position = "bottom"
center   = ["media"]

[monitor."Dell U2720Q"]
bars = ["main", "media"]

[monitor."DP-2"]
bars = []              # no bar here

[monitor."*"]
bars = ["main"]
```

| Key | Values | What it sets |
| --- | ------ | ------------ |
| `bars` | a list of bar names | The bars on this monitor; `[]` for none. |

- A monitor is named by its connector (`"DP-2"`) or by any part of its
  description (`"Dell U2720Q"`), which stays the same through a dock. The
  first entry that matches a monitor decides.
- `"*"` is every monitor no other entry matches: a laptop on its own, a
  projector.
- With no `"*"`, those monitors have every bar, the first on each edge. So a
  file with no `[monitor]` at all puts every bar on every monitor, and a
  monitor that is not connected never leaves the others without a bar.
- Two bars on the same edge in one list, or a name with no bar, are reported
  and the bar left out.

Plugging a monitor in or out, or saving the file, applies these rules again.
