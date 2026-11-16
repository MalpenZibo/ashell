+++
title = "Examples"
description = "Complete configuration files to start from."
weight = 2
+++

Complete files to start from: copy one to `~/.config/ashell/ashell-experimental.toml`
and change what you like. They are also in
[`experimental/ashell-experimental/docs/examples`](https://github.com/MalpenZibo/ashell/tree/main/experimental/ashell-experimental/docs/examples),
where the tests read every one of them.

## A floating island

One island as long as the bar, floating off the screen's edges, its panels splitting off it. Its corners and border are the theme's.

```toml
[bar.main]
shape  = true
padding = "s2"
panels = "attached"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

## Today's ashell

The stable ashell's default, `modules.left = ["Workspaces"]`, `center = ["WindowTitle"]`, `right = [["Tempo", "Privacy", "Settings"]]`: an island round each group, menus dropping down.

```toml
[bar.main]
panels = "dropdown"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

## A solid bar

The stable ashell's default modules on a solid bar, as its `bar.surface = "solid"`: a flat background along the edge, and no island anywhere.

```toml
[bar.main]
background = "solid"
shape      = false
panels     = "dropdown"

start  = ["workspaces"]
center = ["window"]
end    = [["clock", "privacy", "status"]]
```

## An island that morphs

One island in the centre that turns into its panels, and beside it a group of its own for what runs hot, shown only then once items have their data.

```toml
[bar.main]
hover  = true
panels = "attached"

center = [
  { zones = ["workspaces", "window", "tray", "media", "clock", "updates", "status"], panels = "morph" },
  "hot",
]

[zone.hot]
items = [{ item = "system", show = "notable" }]
open  = "system"
```

## A middle ground

The island that morphs, with the tray at the end opening its menu as a detached panel.

```toml
[bar.main]
hover  = true
panels = "detached"

center = [{ zones = ["workspaces", "window", "media", "clock", "status"], panels = "morph" }]
end    = ["tray"]
```
