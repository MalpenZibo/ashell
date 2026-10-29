# Module Registry and Routing

The module registry in `src/modules/mod.rs` connects module names to their implementations. It handles routing views and subscriptions.

## get_module_view

This method maps a `ModuleName` to its rendered view and interaction type:

```rust
fn get_module_view<'a>(
    &'a self,
    id: SurfaceId,
    module_name: &'a ModuleName,
) -> Option<(Element<'a, Message>, Option<ModulePress>)> {
    match module_name {
        ModuleName::Privacy => self
            .privacy
            .view()
            .map(|view| (view.map(Message::Privacy), None)), // No interaction
        ModuleName::Settings => Some((
            self.settings.view(id).map(Message::Settings),
            Some(ModulePress::menu(MenuType::Settings)),
        )),
        // ... one arm per module
    }
}
```

The return type is `Option<(Element, Option<ModulePress>)>`:
- `None` means the module has nothing to display (e.g., privacy module when no indicators are active)
- `Some((view, None))` renders the module without interaction
- `Some((view, Some(press)))` wraps the module in an interactive button

## ModulePress

Describes what happens when a user clicks or scrolls on a module. Every button
handler is a `ModuleAction` — either a plain message or a menu toggle (opening a
menu needs the button's on-screen position, so it is a separate variant):

```rust
struct ModulePress {
    on_press: Option<ModuleAction>,
    on_right_press: Option<ModuleAction>,
    on_middle_press: Option<ModuleAction>,
    on_scroll_up: Option<Box<Message>>,
    on_scroll_down: Option<Box<Message>>,
}

enum ModuleAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}
```

Build one with a chain of helpers instead of filling every field:

```rust
// Left-click only.
ModulePress::message(Message::KeyboardLayout(keyboard_layout::Message::ChangeLayout))

// Left-click opens a menu.
ModulePress::menu(MenuType::Settings)

// Menu on left-click plus extra handlers (e.g. Tempo).
ModulePress::menu(MenuType::Tempo)
    .right_message(Message::Tempo(tempo::Message::CycleFormat))
    .scroll_up(Message::Tempo(tempo::Message::CycleTimezone(
        tempo::TimezoneDirection::Forward,
    )))
    .scroll_down(Message::Tempo(tempo::Message::CycleTimezone(
        tempo::TimezoneDirection::Backward,
    )))
```

For modules that need independent left/right/middle actions (custom modules of
type `Button` and MediaPlayer), build the `ModulePress` struct directly; each
button takes an `Option<ModuleAction>`.

### Usage Notes

- **`ModulePress::message`**: Simple handler, no menu.
- **`ModulePress::menu`**: Opens/closes a popup menu on left-click.
- **`.right_message` / `.middle_message` / `.scroll_up` / `.scroll_down`**: Add
  extra handlers on top of the left-click one. The two message helpers only take
  a plain message; to toggle a menu from another button, build the `ModulePress`
  struct directly and pass `ModuleAction::menu(..)` for that button.

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

Wraps one module's view using the `module_item` builder (`src/components/module_item.rs`):
- If the module has a `ModulePress`, each button handler is attached with `set_handler` (`Button::Press`, `Button::RightPress`, `Button::MiddlePress`) and scroll handlers with `set_scroll_up` / `set_scroll_down`
- Otherwise, it's wrapped in a plain `container`
- When animations are enabled, the content is first wrapped in `animated_size`

### single_module_wrapper

Wraps a single module: its `build_module_item` result is passed to `module_group` (`src/components/module_group.rs`). With the `transparent` (islands) surface, `module_group` gives it a rounded background; with the `solid` surface it is passed through as-is.

### group_module_wrapper

Wraps a group of modules:
- All modules in the group are built with `build_module_item` and placed in a `Row`
- With the `transparent` (islands) surface, the entire group shares one rounded background container (`module_group`)
- Each module within the group still has its own click handler if applicable
- If no module in the group has anything to display, the group is omitted
