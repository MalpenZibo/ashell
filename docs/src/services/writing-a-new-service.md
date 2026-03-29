# Writing a New Service

This guide shows how to add a new backend service to ashell.

## Read-Only D-Bus Service

Most new services will use D-Bus. Here's a template:

### Step 1: Create the Service Files

```
src/services/my_service/
├── mod.rs    # Service logic
└── dbus.rs   # D-Bus proxy definitions
```

### Step 2: Define D-Bus Proxies (`dbus.rs`)

```rust
use zbus::proxy;

#[proxy(
    interface = "org.example.MyService1",
    default_service = "org.example.MyService",
    default_path = "/org/example/MyService"
)]
trait MyService1 {
    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn value(&self) -> zbus::Result<u32>;
}
```

### Step 3: Implement the Service (`mod.rs`)

```rust
use crate::services::{ReadOnlyService, ServiceEvent};
use iced::{Subscription, stream::channel};
use iced::futures::SinkExt;
use std::any::TypeId;

mod dbus;

// Define the update event
#[derive(Debug, Clone)]
pub enum UpdateEvent {
    StatusChanged(String),
    ValueError(u32),
}

// Define the service state
#[derive(Debug, Clone)]
pub struct MyService {
    pub status: String,
    pub value: u32,
}

impl ReadOnlyService for MyService {
    type UpdateEvent = UpdateEvent;
    type Error = String;

    fn update(&mut self, event: Self::UpdateEvent) {
        match event {
            UpdateEvent::StatusChanged(s) => self.status = s,
            UpdateEvent::ValueError(v) => self.value = v,
        }
    }

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with_id(
            TypeId::of::<Self>(),
            channel(10, async move |mut output| {
                // Connect to D-Bus
                let connection = zbus::Connection::system().await.unwrap();
                let proxy = dbus::MyService1Proxy::new(&connection).await.unwrap();

                // Send initial state
                let status = proxy.status().await.unwrap_or_default();
                let value = proxy.value().await.unwrap_or_default();
                let _ = output.send(ServiceEvent::Init(MyService { status, value })).await;

                // Watch for property changes
                let mut status_stream = proxy.receive_status_changed().await;
                loop {
                    use iced::futures::StreamExt;
                    if let Some(change) = status_stream.next().await {
                        if let Ok(new_status) = change.get().await {
                            let _ = output.send(
                                ServiceEvent::Update(UpdateEvent::StatusChanged(new_status))
                            ).await;
                        }
                    }
                }
            }),
        )
    }
}
```

### Step 4: Register in services/mod.rs

```rust
pub mod my_service;
```

### Step 5: Consume from a Module

In your module's subscription:

```rust
use crate::services::my_service::MyService;
use crate::services::{ReadOnlyService, ServiceEvent};

pub fn subscription(&self) -> Subscription<Message> {
    MyService::subscribe().map(|event| Message::ServiceUpdate(event))
}
```

## Bidirectional Service (with Commands)

If your service needs to accept commands, additionally implement the `Service` trait:

```rust
#[derive(Debug, Clone)]
pub enum Command {
    SetValue(u32),
}

impl Service for MyService {
    type Command = Command;

    fn command(&mut self, command: Self::Command) -> Task<ServiceEvent<Self>> {
        match command {
            Command::SetValue(val) => {
                self.value = val;
                Task::perform(
                    async move {
                        // Execute the D-Bus call
                        let connection = zbus::Connection::system().await.unwrap();
                        let proxy = dbus::MyService1Proxy::new(&connection).await.unwrap();
                        proxy.set_value(val).await.unwrap();
                        ServiceEvent::Update(UpdateEvent::ValueError(val))
                    },
                    |event| event,
                )
            }
        }
    }
}
```

## Non-D-Bus Service

For services that don't use D-Bus (e.g., file watching, IPC sockets):

```rust
fn subscribe() -> Subscription<ServiceEvent<Self>> {
    Subscription::run_with_id(
        TypeId::of::<Self>(),
        channel(10, async move |mut output| {
            // Your custom event source here
            // Could be: file watching, socket reading, periodic polling, etc.
            loop {
                let data = read_from_source().await;
                let _ = output.send(ServiceEvent::Update(data)).await;
            }
        }),
    )
}
```

## Using ThrottleExt

If your service produces events very rapidly, use the throttle adapter:

```rust
use crate::services::throttle::ThrottleExt;

// In your subscription loop:
let throttled_stream = event_stream.throttle(Duration::from_millis(100));
```

This prevents UI updates from overwhelming the rendering pipeline.
