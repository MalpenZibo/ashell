# Module Registry and Routing

The module registry in `src/modules/mod.rs` connects module names to their implementations. It handles routing views and subscriptions.

## get_module_view

This method maps a `ModuleName` to a `ModuleView`:

```rust
fn get_module_view<'a>(
    &'a self,
    id: SurfaceId,
    module_name: &'a ModuleName,
) -> Option<ModuleView<'a>> {
    match module_name {
        ModuleName::Privacy => self
            .privacy
            .view()
            .map(|view| ModuleView::plain(view.map(Message::Privacy))),
        ModuleName::Settings => Some(ModuleView::menu(
            self.settings.view(id).map(Message::Settings),
            MenuType::Settings,
        )),
        // ... one arm per module
    }
}
```

`None` means the module has nothing to display (for example, the privacy module when no indicators are active). `ModuleView::plain` renders content without interaction; `ModuleView::menu` / `ModuleView::message` attach an interaction.

## ModuleView and ModuleInteractions

`ModuleView` keeps the rendered content and its optional interactions together, instead of passing a tuple around:

```rust
struct ModuleView<'a> {
    content: Element<'a, Message>,
    interactions: Option<ModuleInteractions>,
}
```

`ModuleInteractions` stores one action per mouse button plus the two scroll directions:

```rust
struct ModuleInteractions {
    buttons: [Option<ModuleAction>; 3],
    scroll_up: Option<Box<Message>>,
    scroll_down: Option<Box<Message>>,
}

enum ModuleAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}
```

`ModuleAction`, `ModuleInteractions` and `ModuleView` live in `src/modules/interaction.rs`. `MouseButton` lives in `src/components/position_button.rs`, next to the widget that consumes it.

Build interactions with a chain of helpers instead of filling every field:

```rust
// Left-click only.
ModuleInteractions::message(Message::KeyboardLayout(keyboard_layout::Message::ChangeLayout))

// Left-click opens a menu.
ModuleInteractions::menu(MenuType::Settings)

// Menu on left-click plus extra handlers (e.g. Tempo).
ModuleInteractions::menu(MenuType::Tempo)
    .right_message(Message::Tempo(tempo::Message::CycleFormat))
    .scroll_up(Message::Tempo(tempo::Message::CycleTimezone(
        tempo::TimezoneDirection::Forward,
    )))
    .scroll_down(Message::Tempo(tempo::Message::CycleTimezone(
        tempo::TimezoneDirection::Backward,
    )))
```

For config-driven handlers, use `button` / `button_opt`:

```rust
ModuleInteractions::default()
    .button(
        MouseButton::Left,
        ModuleAction::message(Message::Custom(name.clone(), LaunchCommand)),
    )
    .button_opt(
        MouseButton::Right,
        config.on_right_click.as_ref().map(|_| {
            ModuleAction::message(Message::Custom(name.clone(), LaunchRightClickCommand))
        }),
    )
    .scroll_up_opt(config.on_scroll_up.as_ref().map(|_| scroll_up_message))
```

`button_opt(..., None)` clears a handler, which is useful when rebuilding interactions from config.

### PressHandler layering

- `ModuleAction` is app-level: a plain `Message` or a menu toggle.
- `PressHandler` is widget-level (`src/components/position_button.rs`): a plain message or a closure that receives the button's on-screen position.
- `ModuleAction::into_handler` is the single conversion point. `PositionButton` does not know about menus or `MenuType`.

Opening a menu needs the button position, so it becomes `PressHandler::WithPosition`; everything else stays `PressHandler::Message`.

## get_module_subscription

Maps each module to its subscriptions:

```rust
fn get_module_subscription(&self, module_name: &ModuleName) -> Option<Subscription<Message>> {
    match module_name {
        ModuleName::Privacy => Some(self.privacy.subscription().map(Message::Privacy)),
        ModuleName::Settings => Some(self.settings.subscription().map(Message::Settings)),
        // ...
    }
}
```

## modules_section

Builds the three bar sections (left, center, right):

```rust
pub fn modules_section<'a>(&'a self, id: SurfaceId) -> [Element<'a, Message>; 3] {
    [
        &self.general_config.modules.left,
        &self.general_config.modules.center,
        &self.general_config.modules.right,
    ]
    .map(|modules_def| {
        let mut row = Row::with_capacity(modules_def.len()) // ...
        for module_def in modules_def {
            row = row.push(match module_def {
                ModuleDef::Single(module) => self.single_module_wrapper(id, module),
                ModuleDef::Group(group) => self.group_module_wrapper(id, group),
            });
        }
        row.into()
    })
}
```

## Module Wrapping

### build_module_item

Wraps one module's `ModuleView` using the `module_item` builder (`src/components/module_item.rs`):
- `ModuleView::into_parts()` gives the content and optional `ModuleInteractions`.
- When animations are enabled, the content is first wrapped in `animated_size`.
- `ModuleInteractions::into_item` maps each `MouseButton` to `set_handler` and sets `set_scroll_up` / `set_scroll_down`.
- Without interactions, the content is wrapped in a plain `container`.

### single_module_wrapper

Wraps a single module: its `build_module_item` result is passed to `module_group` (`src/components/module_group.rs`). With the `transparent` (islands) surface, `module_group` gives it a rounded background; with the `solid` surface it is passed through as-is.

### group_module_wrapper

Wraps a group of modules:
- All modules in the group are built with `build_module_item` and placed in a `Row`
- With the `transparent` (islands) surface, the entire group shares one rounded background container (`module_group`)
- Each module within the group still has its own click handler if applicable
- If no module in the group has anything to display, the group is omitted
