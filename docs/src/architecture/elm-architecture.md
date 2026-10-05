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
        let outputs = Outputs::new(/* ... */);
        // ... build modules, init theme and localizer
        (App { /* all fields */ }, warm_icons)
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

**`App::view`** — Renders the UI for a given surface. This is a pure function of the current state:

```rust
pub fn view(&'_ self, id: SurfaceId) -> Element<'_, Message> {
    // Determine which surface this id belongs to (Outputs::has)
    // Render the bar with left/center/right module sections
    // Or render the menu popup, toast or OSD for those surfaces
}
```

### Subscriptions

Subscriptions are long-lived event sources. They run in the background and produce `Message` values:

```rust
pub fn subscription(&self) -> Subscription<Message> {
    Subscription::batch(vec![
        Subscription::batch(self.modules_subscriptions(/* ... */)), // Module subscriptions
        config::subscription(&self.config_path),                     // Config file changes
        iced::output_events().map(Message::OutputEvent),             // Output events
        // ... more subscriptions (logind, ESC key, SIGUSR1, IPC)
    ])
}
```

iced identifies each subscription by its producer (plus the data passed to `Subscription::run_with`), ensuring only one instance runs per subscription.

## Multi-Surface Application

ashell runs as an iced_layershell **application** that manages multiple layer surfaces. It can:

- Create and destroy surfaces dynamically (for multi-monitor support, menus, toasts and the OSD)
- Have different views per surface (main bar vs. menu popup)
- Apply different themes per surface

The application is configured in `main.rs`:

```rust
iced::application(
    App::new((logger, config.clone(), config_path)),
    App::update,
    App::view,
)
.layer_shell(LayerShellSettings { /* initial bar surface */ })
.subscription(App::subscription)
.theme(App::theme)
.scale_factor(App::scale_factor)
.font(/* embedded fonts */)
.default_font(font)
.run()
```

## Why This Matters

The Elm Architecture provides several benefits for ashell:

- **Predictability**: All state changes flow through `update()`. There's no scattered mutation.
- **Debuggability**: You can inspect the `Message` that caused any state change.
- **Modularity**: Each module follows the same pattern, making it easy to add new ones.
- **No data races**: The single-threaded update loop eliminates shared mutable state concerns in the UI.
