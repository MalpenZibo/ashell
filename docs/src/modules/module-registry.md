# Module Registry and Routing

The module registry in `src/modules/mod.rs` connects module names to their implementations. It handles routing views and subscriptions.

## get_module_view

This method maps a `ModuleName` to its rendered view and interaction type:

```rust
fn get_module_view(&self, id: Id, module_name: &ModuleName)
    -> Option<(Element<Message>, Option<OnModulePress>)>
{
    match module_name {
        ModuleName::Privacy => Some((
            self.privacy.view(&self.theme).map(Message::Privacy),
            None,  // No interaction
        )),
        ModuleName::Settings => Some((
            self.settings.view(&self.theme).map(Message::Settings),
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

Defines what happens when a user clicks a module:

```rust
pub enum OnModulePress {
    // Emit a specific message on click
    Action(Box<Message>),

    // Toggle a popup menu
    ToggleMenu(MenuType),

    // Toggle menu + right-click and scroll handlers
    ToggleMenuWithExtra {
        menu_type: MenuType,
        on_right_press: Option<Box<Message>>,
        on_scroll_up: Option<Box<Message>>,
        on_scroll_down: Option<Box<Message>>,
    },
}
```

For example, the Tempo module uses `ToggleMenuWithExtra` to:
- Left-click: Open the calendar/weather menu
- Right-click: Cycle the time format
- Scroll: Cycle through timezones

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
pub fn modules_section(&self, id: Id, theme: &AshellTheme) -> [Element<Message>; 3] {
    [left, center, right].map(|modules_def| {
        let mut row = Row::new();
        for module_def in modules_def {
            row = row.push_maybe(match module_def {
                ModuleDef::Single(module) => self.single_module_wrapper(id, theme, module),
                ModuleDef::Group(group) => self.group_module_wrapper(id, theme, group),
            });
        }
        row.into()
    })
}
```

## Module Wrapping

### single_module_wrapper

Wraps a single module:
- If the module has an `OnModulePress` action, it's wrapped in a `PositionButton`
- Otherwise, it's wrapped in a plain `container`
- In `Islands` style, non-interactive modules get a rounded background

### group_module_wrapper

Wraps a group of modules:
- All modules in the group are placed in a `Row`
- In `Islands` style, the entire group shares one rounded background container
- Each module within the group still has its own click handler if applicable
