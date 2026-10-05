# Anatomy of a Module

Modules follow a consistent pattern, though they don't implement a formal trait. Instead, they follow a convention that the `App` struct and `modules/mod.rs` rely on.

## The Module Pattern

Every module has:

### 1. A Message Enum

```rust
#[derive(Debug, Clone)]
pub enum Message {
    // Module-specific events
}
```

### 2. A Struct

```rust
pub struct MyModule {
    config: MyModuleConfig,
    // Module state
}
```

### 3. Constructor

```rust
impl MyModule {
    pub fn new(config: MyModuleConfig) -> Self {
        Self { config, /* ... */ }
    }
}
```

### 4. Update Method

```rust
pub fn update(&mut self, message: Message) -> /* Action or Task or () */ {
    match message {
        // Handle each message variant
    }
}
```

### 5. View Method

```rust
pub fn view(&self) -> Element<'_, Message> {
    // Return iced elements (theme values are read with `use_theme`)
}
```

### 6. Subscription Method

```rust
pub fn subscription(&self) -> Subscription<Message> {
    // Return event sources (timers, service events, etc.)
}
```

## Optional: Menu View

Modules with popup menus also implement:

```rust
pub fn menu_view(&self) -> Element<'_, Message> {
    // Return the menu popup content
}
```

## The Action Pattern

Some modules return an `Action` enum from `update()` instead of a plain `Task`. This allows modules to request operations they can't perform themselves. Each module defines its own `Action`; for example, the Settings module's:

```rust
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu(SurfaceId),
    RequestKeyboardWithCommand(SurfaceId, Task<Message>),
    ReleaseKeyboard(SurfaceId),
    ReleaseKeyboardWithCommand(SurfaceId, Task<Message>),
    OpenTooltipMenu(SurfaceId, MenuType, ButtonUIRef),
    CloseTooltipMenu(SurfaceId, MenuType),
}
```

The `App::update()` method interprets these actions. For example, `CloseMenu` tells the App to close the menu surface, which the module can't do directly.

Modules that use the Action pattern: **Settings**, **Tray**, **Updates**, **MediaPlayer**, **Tempo**, **Notifications**.

## Service Consumption

Modules consume services through their subscription:

```rust
pub fn subscription(&self) -> Subscription<Message> {
    CompositorService::subscribe().map(|event| Message::ServiceEvent(Box::new(event)))
}
```

The module's `Message` enum includes variants for service events:

```rust
pub enum Message {
    ServiceEvent(Box<ServiceEvent<CompositorService>>),
    // ...
}
```

And the `update()` method handles them:

```rust
Message::ServiceEvent(event) => match *event {
    ServiceEvent::Init(service) => {
        self.service = Some(service);
    }
    ServiceEvent::Update(event) => {
        if let Some(service) = &mut self.service {
            service.update(event);
        }
    }
    // ...
}
```

## Integration with App

Each module is integrated into the App through several touchpoints:

1. **Field in `App` struct** (`src/app.rs`)
2. **Variant in `Message` enum** (`src/app/message.rs`)
3. **Match arm in `App::update()`** (`src/app.rs`)
4. **Entry in `get_module_view()`** (`src/modules/mod.rs`)
5. **Entry in `get_module_subscription()`** (`src/modules/mod.rs`)
6. **`ModuleName` variant** (`src/config.rs`)
