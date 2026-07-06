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
pub fn view(&self, theme: &AshellTheme) -> Element<Message> {
    // Return iced elements
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
pub fn menu_view(&self, theme: &AshellTheme) -> Element<Message> {
    // Return the menu popup content
}
```

## The Action Pattern

Some modules return an `Action` enum from `update()` instead of a plain `Task`. This allows modules to request operations they can't perform themselves:

```rust
pub enum Action {
    None,
    Command(Task<Message>),
    CloseMenu,
    RequestKeyboard,
    ReleaseKeyboard,
    ReleaseKeyboardWithCommand(Task<Message>),
}
```

The `App::update()` method interprets these actions. For example, `CloseMenu` tells the App to close the menu surface, which the module can't do directly.

Modules that use the Action pattern: **Settings**, **Tray**, **Updates**, **MediaPlayer**, **Tempo**.

## Service Consumption

Modules consume services through their subscription:

```rust
pub fn subscription(&self) -> Subscription<Message> {
    CompositorService::subscribe()
        .map(|event| Message::CompositorEvent(event))
}
```

The module's `Message` enum includes variants for service events:

```rust
pub enum Message {
    CompositorEvent(ServiceEvent<CompositorService>),
    // ...
}
```

And the `update()` method handles them:

```rust
Message::CompositorEvent(ServiceEvent::Init(service)) => {
    self.compositor = Some(service);
}
Message::CompositorEvent(ServiceEvent::Update(event)) => {
    if let Some(compositor) = &mut self.compositor {
        compositor.update(event);
    }
}
```

## Integration with App

Each module is integrated into the App through several touchpoints:

1. **Field in `App` struct** (`src/app.rs`)
2. **Variant in `Message` enum** (`src/app.rs`)
3. **Match arm in `App::update()`** (`src/app.rs`)
4. **Entry in `get_module_view()`** (`src/modules/mod.rs`)
5. **Entry in `get_module_subscription()`** (`src/modules/mod.rs`)
6. **`ModuleName` variant** (`src/config.rs`)
