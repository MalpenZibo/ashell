# Module Registry and Routing

The module registry in `src/modules/mod.rs` connects module names to their implementations. It handles routing views and subscriptions.

## get_module_view

This method maps a `ModuleName` to its rendered view and interaction type:

```rust
fn get_module_view<'a>(
    &'a self,
    id: SurfaceId,
    module_name: &'a ModuleName,
) -> Option<(Element<'a, Message>, Option<OnModulePress>)> {
    match module_name {
        ModuleName::Privacy => self
            .privacy
            .view()
            .map(|view| (view.map(Message::Privacy), None)), // No interaction
        ModuleName::Settings => Some((
            self.settings.view(id).map(Message::Settings),
            Some(OnModulePress::ToggleMenu(MenuType::Settings)),
        )),
        // ... one arm per module
    }
}
```

The return type is `Option<(Element, Option<OnModulePress>)>`:
- `None` means the module has nothing to display (e.g., privacy module when no indicators are active)
- `Some((view, None))` renders the module without interaction
- `Some((view, Some(action)))` wraps the module in an interactive button

## OnModulePress

Defines what happens when a user clicks or interacts with a module:

```rust
pub enum OnModulePress {
    // Emit a specific message on left-click
    Action(Box<Message>),

    // Toggle a popup menu on left-click
    ToggleMenu(MenuType),

    // Toggle menu with right-click and scroll event handlers
    ToggleMenuWithExtra {
        menu_type: MenuType,
        on_right_press: Option<Box<Message>>,
        on_scroll_up: Option<Box<Message>>,
        on_scroll_down: Option<Box<Message>>,
    },

    // Independent handlers for each mouse button and scroll direction
    CustomAction {
        on_press: Option<ModuleButtonAction>,
        on_right_press: Option<ModuleButtonAction>,
        on_middle_press: Option<ModuleButtonAction>,
        on_scroll_up: Option<Box<Message>>,
        on_scroll_down: Option<Box<Message>>,
    },
}
```

`CustomAction` button handlers use `ModuleButtonAction`, which is either a plain message or a menu toggle (opening a menu needs the button's on-screen position, so it is a separate variant):

```rust
pub enum ModuleButtonAction {
    Message(Box<Message>),
    ToggleMenu(MenuType),
}
```

### Usage Notes

- **`Action`**: Simple left-click handler, no menu.
- **`ToggleMenu`**: Left-click opens/closes a popup menu.
- **`ToggleMenuWithExtra`**: For modules with a menu that also need right-click or scroll handlers (e.g., Tempo: left-click opens calendar, right-click cycles time format, scroll cycles timezones). Middle-click is not supported in this variant.
- **`CustomAction`**: For modules that need independent handlers for multiple mouse buttons and scroll events. Supports left-click, right-click, middle-click, scroll up, and scroll down; each button can emit a message or toggle a menu (`ModuleButtonAction`). Used by custom modules of type `Button` and by MediaPlayer, whose button actions are configurable.

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
- If the module has an `OnModulePress` action, its handlers are attached and the content is wrapped in a `PositionButton`
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
