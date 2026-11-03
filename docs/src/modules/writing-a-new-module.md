# Writing a New Module

This guide walks through adding a new module to ashell, step by step.

## Step 1: Create the Module File

Create `src/modules/my_module.rs`:

```rust
use crate::{
    components::icons::{StaticIcon, icon},
    config::ModuleName,
    theme::use_theme,
};
use iced::{Element, Subscription, widget::{row, text}};

#[derive(Debug, Clone)]
pub enum Message {
    // Define your messages here
    Tick,
}

pub struct MyModule {
    // Your state here
    value: String,
}

impl MyModule {
    pub fn new() -> Self {
        Self {
            value: "Hello".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                // Handle the message
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let size = use_theme(|t| t.module_font_size(&ModuleName::MyModule, t.font_size.md));

        row![icon(StaticIcon::Point).size(size), text(&self.value).size(size)]
            .spacing(use_theme(|t| t.space.xxs))
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
```

### Follow the module styles

The view reads its text size through `module_font_size`, passing the size it uses on its own. Do the same for the module's main text and the icons next to it, so `font_size` in `[appearance.modules.my_module]` applies to it; secondary texts can keep a fixed size. Then add the module to the "Module Font Size" table on the website's Appearance page. The island style and `text_color` are applied around the module in `src/modules/mod.rs`, so there is nothing to do for them.

## Step 2: Register the Module Name

In `src/config.rs`, add your module to the `ModuleName` enum:

```rust
pub enum ModuleName {
    // ... existing variants
    MyModule,
}
```

Then add it to `BUILTIN_MODULES` in the same file (and bump the array length), with its layout name and its settings section name:

```rust
("MyModule", "my_module", ModuleName::MyModule),
```

The layout name is what users write in `[modules]`; any other string is parsed as `ModuleName::Custom`. The section name is also the module's key in `[appearance.modules]`, so `[appearance.modules.my_module]` styles it.

## Step 3: Add to Module Declarations

In `src/modules/mod.rs`, add the module declaration:

```rust
pub mod my_module;
```

## Step 4: Add to App Struct

In `src/app.rs`, add the field:

```rust
pub struct App {
    // ... existing fields
    pub my_module: MyModule,
}
```

## Step 5: Initialize in App::new

In `App::new()`:

```rust
(App {
    // ... existing fields
    my_module: MyModule::new(),
}, task)
```

## Step 6: Add Message Variant

In `src/app/message.rs`, add to the `Message` enum:

```rust
pub enum Message {
    // ... existing variants
    MyModule(modules::my_module::Message),
}
```

## Step 7: Wire Up in App::update

In `App::update()`, add the match arm:

```rust
Message::MyModule(msg) => {
    self.my_module.update(msg);
    Task::none()
}
```

## Step 8: Wire Up in Module Registry

In `src/modules/mod.rs`:

### get_module_view

```rust
ModuleName::MyModule => Some((
    self.my_module.view().map(Message::MyModule),
    None,  // Or Some(ModulePress::menu(MenuType::MyModule)) if you have a menu
)),
```

### get_module_subscription

```rust
ModuleName::MyModule => Some(
    self.my_module.subscription().map(Message::MyModule),
),
```

## Step 9: Add Config (Optional)

If your module needs configuration, add a config struct in `src/config.rs`:

```rust
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct MyModuleConfig {
    pub some_setting: String,
}

impl Default for MyModuleConfig {
    fn default() -> Self {
        Self {
            some_setting: "default_value".to_string(),
        }
    }
}
```

Add the field to the `Config` struct:

```rust
pub struct Config {
    // ...
    pub my_module: MyModuleConfig,
}
```

Then accept it in your module's constructor:

```rust
pub fn new(config: MyModuleConfig) -> Self { /* ... */ }
```

## Step 10: Add a Menu (Optional)

If your module needs a popup menu:

1. Add a variant to `MenuType` in `src/components/menu.rs`:

```rust
pub enum MenuType {
    // ...
    MyModule,
}
```

2. Add a `menu_view()` method to your module.

3. Change `get_module_view` to return `Some(ModulePress::menu(MenuType::MyModule))`.

4. Handle the menu rendering in `App::view()` / `App::menu_wrapper()`.

## Step 11: Handle Config Reload (Optional)

If your module needs to respond to config changes, add a `ConfigReloaded` variant to your Message:

```rust
pub enum Message {
    ConfigReloaded(MyModuleConfig),
    // ...
}
```

And call it from `App::refresh_config()`.

## Testing Your Module

1. Add your module to the config file:

```toml
[modules]
right = ["MyModule"]
```

2. Build and run:

```bash
make start
```

3. Edit the config to test hot-reload:

```bash
# Changes should appear without restarting ashell
```
