# Theme System

The theme system is defined in `src/theme.rs`. It wraps iced's built-in theming with ashell-specific tokens for spacing, radius, font sizes, and bar styles.

## AshellTheme Struct

```rust
pub struct AshellTheme {
    pub iced_theme: Theme,                                    // iced's built-in theme
    pub space: Space,                                         // Spacing tokens
    pub radius: Radius,                                       // Border radius tokens
    pub font_size: FontSize,                                  // Font size tokens
    pub bar_position: Position,                               // Top or Bottom
    pub bar_style: AppearanceStyle,                           // Islands, Solid, or Gradient
    pub opacity: f32,                                         // Bar opacity (0.0-1.0)
    pub menu: MenuAppearance,                                 // Menu-specific styling
    pub workspace_colors: Vec<AppearanceColor>,               // Per-workspace color cycling
    pub special_workspace_colors: Option<Vec<AppearanceColor>>, // Special workspace colors
    pub scale_factor: f64,                                    // DPI scale factor
}
```

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

## Bar Styles

ashell supports three visual styles:

- **`Solid`**: Flat background color across the entire bar width.
- **`Gradient`**: The background fades from solid to transparent, away from the bar's edge. The gradient direction is determined by the bar position (top = downward fade, bottom = upward fade).
- **`Islands`**: No continuous background. Each module (or module group) gets its own rounded container with the background color, creating a "floating islands" look.

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
    // Handles Islands vs Solid/Gradient backgrounds differently
}
```

## Theme Construction

The theme is built from the config's `Appearance` section:

```rust
impl AshellTheme {
    pub fn new(position: Position, appearance: &Appearance) -> Self {
        AshellTheme {
            iced_theme: Theme::custom_with_fn(/* ... */),
            space: Space::default(),
            radius: Radius::default(),
            font_size: FontSize::default(),
            bar_position: position,
            bar_style: appearance.style,
            opacity: appearance.opacity,
            // ...
        }
    }
}
```

The iced theme is created with `Theme::custom_with_fn()`, which builds a palette from the configured colors.
