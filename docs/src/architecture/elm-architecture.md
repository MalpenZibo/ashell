# The Elm Architecture in ashell

## Model-View-Update (MVU)

ashell follows the [Elm Architecture](https://guide.elm-lang.org/architecture/), a pattern for building interactive applications with unidirectional data flow. In iced's terminology:

```
          ┌──────────────────────┐
          │     Subscription     │
          │  (external events)   │
          └──────────┬───────────┘
                     │ Message
                     ▼
┌────────────────────────────────────┐
│            update()                │
│   fn update(&mut self, msg)        │
│       -> Task<Message>             │
│                                    │
│   Mutates state, returns effects   │
└────────────────┬───────────────────┘
                 │ state changed
                 ▼
┌────────────────────────────────────┐
│             view()                 │
│   fn view(&self, id)               │
│       -> Element<Message>          │
│                                    │
│   Pure function of state           │
│   (immutable borrow)               │
└────────────────┬───────────────────┘
                 │ user interaction
                 │ Message
                 └──────► back to update()
```

### The Three Core Methods

In `src/app.rs`, the `App` struct implements these key methods:

**`App::new`** — Creates the initial state and returns any startup tasks:

```rust
pub fn new(
    (logger, config, config_path): (LoggerHandle, Config, PathBuf),
) -> impl FnOnce() -> (Self, Task<Message>) {
    move || {
        let (outputs, task) = Outputs::new(/* ... */);
        (App { /* all fields */ }, task)
    }
}
```

**`App::update`** — Processes a `Message` and returns a `Task<Message>` for side effects:

```rust
// Conceptual structure (simplified)
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Settings(msg) => { /* delegate to settings module */ }
        Message::ConfigChanged(config) => { /* hot-reload config */ }
        Message::ToggleMenu(menu_type, id, button_ref) => { /* open/close menu */ }
        // ... one arm per message variant
    }
}
```

**`App::view`** — Renders the UI for a given window. This is a pure function of the current state:

```rust
fn view(&self, id: Id) -> Element<Message> {
    // Determine which output this window belongs to
    // Render the bar with left/center/right module sections
    // Or render the menu popup if this is a menu surface
}
```

### Subscriptions

Subscriptions are long-lived event sources. They run in the background and produce `Message` values:

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch(vec![
        config::subscription(/* ... */),                    // Config file changes
        self.modules_subscriptions(/* ... */),              // All module subscriptions
        CompositorService::subscribe().map(/* ... */),      // Compositor events
        // ... more subscriptions
    ])
}
```

Each subscription is identified by a `TypeId` or a unique key, ensuring only one instance runs per subscription type.

## Daemon Mode

ashell uses iced's **daemon mode**, which supports multiple windows (surfaces). Unlike a standard iced application with a single window, the daemon can:

- Create and destroy windows dynamically (for multi-monitor support)
- Have different views per window (main bar vs. menu popup)
- Apply different themes and scale factors per window

The daemon is configured in `main.rs`:

```rust
iced::daemon(App::title, App::update, App::view)
    .subscription(App::subscription)
    .theme(App::theme)
    .style(App::style)
    .scale_factor(App::scale_factor)
    .font(/* embedded fonts */)
    .run_with(App::new(/* ... */))
```

## Why This Matters

The Elm Architecture provides several benefits for ashell:

- **Predictability**: All state changes flow through `update()`. There's no scattered mutation.
- **Debuggability**: You can inspect the `Message` that caused any state change.
- **Modularity**: Each module follows the same pattern, making it easy to add new ones.
- **No data races**: The single-threaded update loop eliminates shared mutable state concerns in the UI.
