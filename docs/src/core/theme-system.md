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
    pub menu: MenuAppearance,                                 // Menu-specific styling
    pub workspace_colors: Vec<AppearanceColor>,               // Per-workspace color cycling
    pub special_workspace_colors: Option<Vec<AppearanceColor>>, // Special workspace colors
    pub scale_factor: f64,                                    // DPI scale factor
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
take a `Paint`, so a raw palette colour will not compile where a fill is
expected. That matters because the mistake is otherwise invisible: an
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
    pub xxs: u16,  // 4px
    pub xs: u16,   // 8px
    pub sm: u16,   // 12px
    pub md: u16,   // 16px
    pub lg: u16,   // 24px
    pub xl: u16,   // 32px
    pub xxl: u16,  // 48px
}
```

### Border Radius

```rust
pub struct Radius {
    pub sm: u16,   // 4px
    pub md: u16,   // 8px
    pub lg: u16,   // 16px
    pub xl: u16,   // 32px
}
```

### Font Sizes

```rust
pub struct FontSize {
    pub xxs: u16,  // 8px
    pub xs: u16,   // 10px
    pub sm: u16,   // 12px
    pub md: u16,   // 16px
    pub lg: u16,   // 20px
    pub xl: u16,   // 22px
    pub xxl: u16,  // 32px
}
```

## Bar Surface

The `[appearance.bar].surface` field controls where the background is painted:

- **`transparent`**: No continuous background. Each module (or module group) gets its own rounded container with the background color, creating a "floating islands" look. This is the default.
- **`solid`**: Flat background color across the entire bar width; module groups render pass-through so the bar reads as a single surface.

The bar surface can additionally be rounded (`radius`) and inset from the screen edges (`margin`); both use CSS shorthand over the radius/spacing scales.

## Color System

Colors are defined through the `AppearanceColor` enum:

```toml
# Simple: just a hex color
background = "#1e1e2e"

# Complete: base + strong + weak + text variants
[appearance.primary]
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
| `module_button_style(grouped)` | Module buttons in the bar |
| `ghost_button_style()` | Transparent buttons in menus |
| `quick_settings_button_style()` | Quick settings toggles |
| `workspace_button_style(index, active)` | Workspace indicator buttons |
| `menu_button_style()` | Items inside dropdown menus |

Each method returns a closure compatible with iced's button styling API:

```rust
pub fn module_button_style(&self, grouped: bool) -> impl Fn(&Theme, Status) -> button::Style {
    // Returns different styles for hovered, pressed, and default states
    // Handles transparent (islands) vs solid backgrounds differently
}
```

## Theme Construction

The theme is built from the config's `Appearance` section:

```rust
impl AshellTheme {
    pub fn new(position: Position, appearance: &Appearance) -> Self {
        AshellTheme {
            surfaces: Surface::ALL.map(/* one theme per surface */),
            space: Space::default(),
            radius: Radius::default(),
            font_size: FontSize::default(),
            bar_position: position,
            bar_surface: appearance.bar.surface,
            bar_radius: appearance.bar.radius,
            bar_margin: appearance.bar.margin,
            // ...
        }
    }
}
```

Each iced theme is created with `Theme::custom_with_fn()`, which builds a palette from the configured colors. The derived `palette::Extended` does not depend on the opacity, so it is generated once and shared by all four surface themes.
