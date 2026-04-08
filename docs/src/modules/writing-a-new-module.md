# Writing a New Module

This guide walks through adding a new module to ashell, step by step.

## Step 1: Create the Module File

Create `src/modules/my_module.rs`:

```rust
use crate::theme::AshellTheme;
use iced::{Element, Subscription, widget::text};

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

    pub fn view(&self, _theme: &AshellTheme) -> Element<Message> {
        text(&self.value).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}
```

## Step 2: Register the Module Name

In `src/config.rs`, add your module to the `ModuleName` enum:

```rust
pub enum ModuleName {
    // ... existing variants
    MyModule,
}
```

Make sure the serde deserialization handles the string representation (the enum variant name is used as the TOML string).

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

In `src/app.rs`, add to the `Message` enum:

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
    self.my_module.view(&self.theme).map(Message::MyModule),
    None,  // Or Some(OnModulePress::ToggleMenu(MenuType::MyModule)) if you have a menu
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

1. Add a variant to `MenuType` in `src/menu.rs`:

```rust
pub enum MenuType {
    // ...
    MyModule,
}
```

2. Add a `menu_view()` method to your module.

3. Change `get_module_view` to return `Some(OnModulePress::ToggleMenu(MenuType::MyModule))`.

4. Handle the menu rendering in `App::view()` / `App::menu_wrapper()`.

## Step 11: Handle Config Reload (Optional)

If your module needs to respond to config changes, add a `ConfigReloaded` variant to your Message:

```rust
pub enum Message {
    ConfigReloaded(MyModuleConfig),
    // ...
}
```

And call it from `App::refesh_config()`.

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
