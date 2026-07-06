# Walkthrough: The Clock Module (Historical)

> **Note**: The Clock module has been removed from the codebase. This walkthrough is preserved as a teaching example of the module pattern. For current patterns, see [Anatomy of a Module](anatomy-of-a-module.md).

The Clock module was the simplest module in ashell at just 60 lines. It's an ideal example for understanding the module pattern.

## Complete Source (annotated)

```rust
use crate::{config::ClockModuleConfig, theme::AshellTheme};
use chrono::{DateTime, Local};
use iced::{Element, Subscription, time::every, widget::text};
use log::warn;
use std::time::Duration;

// 1. Message enum — defines what events this module handles
#[derive(Debug, Clone)]
pub enum Message {
    Update,    // Fired by the timer subscription
}

// 2. Module struct — holds the module's state
pub struct Clock {
    config: ClockModuleConfig,      // Format string from config
    date: DateTime<Local>,          // Current time
}

impl Clock {
    // 3. Constructor — creates the module from config
    pub fn new(config: ClockModuleConfig) -> Self {
        warn!("Clock module is deprecated. Please migrate to the Tempo module.");
        Self {
            config,
            date: Local::now(),
        }
    }

    // 4. Update — handles messages, mutates state
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Update => {
                self.date = Local::now();
            }
        }
    }

    // 5. View — renders the UI (pure function of state)
    pub fn view(&'_ self, _: &AshellTheme) -> Element<'_, Message> {
        text(self.date.format(&self.config.format).to_string()).into()
    }

    // 6. Subscription — event source (timer)
    pub fn subscription(&self) -> Subscription<Message> {
        // Smart interval: 1s if format includes seconds, 5s otherwise
        let second_specifiers = ["%S", "%T", "%X", "%r", "%:z", "%s"];
        let interval = if second_specifiers
            .iter()
            .any(|&spec| self.config.format.contains(spec))
        {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        };

        every(interval).map(|_| Message::Update)
    }
}
```

## Key Observations

### Simplicity

The entire module is:
- **1 enum** (Message) with 1 variant
- **1 struct** (Clock) with 2 fields
- **4 methods**: `new`, `update`, `view`, `subscription`

### No Service Dependency

The Clock module doesn't use any service. It gets the time directly via `chrono::Local::now()`. More complex modules would subscribe to a service instead.

### Smart Subscription Interval

The subscription adjusts its frequency based on the configured format string. If the format includes seconds (`%S`, `%T`, etc.), it ticks every second. Otherwise, it ticks every 5 seconds to save resources.

### No Action Pattern

`update()` returns `()` (nothing). It simply mutates state. More complex modules (like Settings) return an `Action` enum to request App-level operations.

## Integration Points

In `src/app.rs`:

```rust
pub struct App {
    pub clock: Clock,    // Field
    // ...
}

pub enum Message {
    Clock(modules::clock::Message),   // Variant
    // ...
}
```

In `App::update()`:

```rust
Message::Clock(msg) => {
    self.clock.update(msg);
    Task::none()
}
```

In `src/modules/mod.rs`:

```rust
// get_module_view
ModuleName::Clock => Some((
    self.clock.view(&self.theme).map(Message::Clock),
    None,   // No click interaction
)),

// get_module_subscription
ModuleName::Clock => Some(self.clock.subscription().map(Message::Clock)),
```

In `src/config.rs`:

```rust
pub enum ModuleName {
    Clock,
    // ...
}
```
