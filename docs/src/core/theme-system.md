# Theme System

The theme system is defined in `src/theme.rs`. It wraps iced's built-in theming with ashell-specific tokens for spacing, radius, font sizes, and bar styles.

## AshellTheme Struct

```rust
pub struct AshellTheme {
    surfaces: [SurfaceTheme; 4],                              // One theme per Surface
    pub palette: Palette,                                     // Ink colours, no `&Theme` needed
    pub space: Space,                                         // Spacing tokens
    pub radius: Radius,                                       // Border radius tokens
    pub font_size: FontSize,                                  // Font size tokens
    pub bar_position: Position,                               // Top or Bottom
    pub bar_surface: BarSurface,                              // transparent or solid
    pub bar_radius: BarRadius,                                // per-corner radius (CSS shorthand)
    pub bar_margin: BarMargin,                                // per-edge margin (CSS shorthand)
    pub bar_padding: BarPadding,                              // per-edge padding (CSS shorthand)
    pub menu: MenuAppearance,                                 // Menu-specific styling
    pub workspace_colors: Vec<AppearanceColor>,               // Per-workspace color cycling
    pub special_workspace_colors: Option<Vec<AppearanceColor>>, // Special workspace colors
    pub scale_factor: f64,                                    // DPI scale factor
    pub animations_enabled: bool,                             // From [animations].enabled
}
```

Each layer-shell surface is drawn with its own theme, so `appearance.opacity`
can vary per surface:

```rust
pub enum Surface { Bar, Menu, Osd, Notifications }

/// Everything that varies from one surface to the next.
pub struct SurfaceTheme {
    pub iced_theme: Theme,
    pub blur: bool,
}
```

Reach for one with `theme.surface(Surface::Menu)`. `App::theme(id)` picks the
surface via `HasOutput::surface()`.

### Paint vs ink

Opacity is carried by `Palette::background` only, so a fill has to pick it up
explicitly. The `Paint` type makes that choice a type, not a convention:

```rust
Paint::surface(theme, color)  // part of the surface: carries its opacity
Paint::opaque(color)          // drawn on the surface: keeps its contrast
```

The fill helpers (`card_style`, `surface_border`, the `button_style` family)
build their fills through `Paint`, and `Paint` (not `Color`) is what converts
into an iced `Background`. That matters because the mistake is otherwise invisible: an
un-opacified fill looks correct at the default `opacity = 1.0` and only goes
wrong once a surface is made translucent.

A colour that is *ink* (text, icons, accents) is not a `Paint` at all and
stays a plain `Color`. Marks that need to be subtle use a fixed ratio of the
foreground, `ink(theme, alpha)`, rather than a scaled background, so they read
the same at any opacity.

## Design Tokens

### Spacing

```rust
pub struct Space {
    pub xxs: f32,  // 4px
    pub xs: f32,   // 8px
    pub sm: f32,   // 12px
    pub md: f32,   // 16px
    pub lg: f32,   // 24px
    pub xl: f32,   // 32px
    pub xxl: f32,  // 48px
}
```

### Border Radius

```rust
pub struct Radius {
    pub sm: f32,   // 4px
    pub md: f32,   // 8px
    pub lg: f32,   // 16px
    pub xl: f32,   // 32px
}
```

### Font Sizes

```rust
pub struct FontSize {
    pub xxs: f32,  // 8px
    pub xs: f32,   // 10px
    pub sm: f32,   // 12px
    pub md: f32,   // 16px
    pub lg: f32,   // 20px
    pub xl: f32,   // 22px
    pub xxl: f32,  // 32px
}
```

## Bar Surface

The `[appearance.bar].surface` field controls where the background is painted:

- **`transparent`**: No continuous background. Each module (or module group) gets its own rounded container with the background color, creating a "floating islands" look. This is the default.
- **`solid`**: Flat background color across the entire bar width; module groups render pass-through so the bar reads as a single surface.

The bar surface can additionally be rounded (`radius`) and inset from the screen edges (`margin`); both use CSS shorthand over the radius/spacing scales. Additionally, margin can be an `f32` representing physical pixels. Margins are applied by the compositor outside the bar surface, so they are always screen pixels and are not multiplied by `scale_factor`. `padding` uses the same shorthand to inset the bar content without moving the surface, so a solid bar keeps its full-width background; being drawn inside the bar it does scale with `scale_factor`.

## Color System

Colors are defined through the `AppearanceColor` enum:

```toml
# Simple: just a hex color
background_color = "#1e1e2e"

# Complete: base + strong + weak + text variants
[appearance.primary_color]
base = "#cba6f7"
strong = "#dbbcff"
weak = "#a385d8"
text = "#1e1e2e"
```

Colors map to iced's `Extended` palette system with `base`, `strong`, `weak`, and `text` variants.

## Button Styles

`theme.rs` defines multiple button style methods used across the UI:

| Method | Used By |
|--------|---------|
| `module_button_style()` | Module buttons in the bar |
| `button_style(kind, hierarchy)` | General buttons (`ButtonKind`: Solid/Transparent/Outline; `ButtonHierarchy`: Primary/Secondary/Danger) |
| `quick_settings_button_style(active)` | Quick settings toggles |
| `quick_settings_submenu_button_style(active)` | Quick settings submenu toggles |
| `workspace_button_style(is_empty, is_urgent, is_active, colors)` | Workspace indicator buttons |

Each method returns a closure compatible with iced's button styling API:

```rust
pub fn module_button_style(&self) -> impl Fn(&Theme, Status) -> button::Style + use<> {
    // Transparent base with a hover highlight; the module-group
    // background is handled by `module_group`, not the button
}
```

## Theme Construction

The theme is built from the config's `Appearance` section:

```rust
impl AshellTheme {
    pub fn new(position: Position, appearance: &Appearance, animations: &AnimationsConfig) -> Self {
        base_theme_from_appearance(appearance, position, animations.enabled)
    }
}

fn base_theme_from_appearance(appearance: &Appearance, bar_position: Position, animations_enabled: bool) -> AshellTheme {
    AshellTheme {
        surfaces: Surface::ALL.map(/* one theme per surface */),
        space: Space::default(),
        radius: Radius::default(),
        font_size: FontSize::default(),
        bar_position,
        bar_surface: appearance.bar.surface,
        bar_radius: appearance.bar.radius,
        bar_margin: appearance.bar.margin,
        bar_padding: appearance.bar.padding,
        // ...
    }
}
```

The resulting `AshellTheme` is stored in a thread-local with `init_theme()` (at startup and on config reload) and read anywhere with `use_theme(|t| ...)`.

Each iced theme is created with `Theme::custom_with_fn()`, which builds a palette from the configured colors. The derived `palette::Extended` does not depend on the opacity, so it is generated once and shared by all four surface themes.
