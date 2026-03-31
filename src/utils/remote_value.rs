use std::time::Duration;
use tokio::time::sleep;

/// A helper for ensuring responsive user interface,
/// when handling async state
#[derive(Debug, Default, Clone)]
pub struct Remote<Value> {
    /// Requested value is immediately displayed, ensuring responsive user interface
    requested: Option<Value>,
    /// Source of truth. Displayed shortly after the end of the user interaction
    received: Value,
    /// A handle for aborting the timeout task
    timeout: Option<iced_layershell::task::Handle>,
}

impl<Value: Default> Remote<Value> {
    pub fn new(value: Value) -> Self {
        Self {
            received: value,
            ..Default::default()
        }
    }
}

impl<Value> Remote<Value>
where
    Value: Copy + Send + 'static,
{
    pub fn receive(&mut self, value: Value) {
        self.received = value
    }

    pub fn value(&self) -> Value {
        self.requested.unwrap_or(self.received)
    }

    pub fn update(&mut self, message: Message<Value>) -> iced_layershell::Task<Message<Value>> {
        if let Some(handle) = self.timeout.take() {
            handle.abort();
        }
        match message {
            Message::Request(value) => {
                self.requested = Some(value);
                iced_layershell::Task::none()
            }
            Message::Timeout => self.start_timeout(),
            Message::RequestAndTimeout(value) => {
                self.requested = Some(value);
                self.start_timeout()
            }
            Message::ShowReceived => {
                self.requested = None;
                iced_layershell::Task::none()
            }
        }
    }

    fn start_timeout(&mut self) -> iced_layershell::Task<Message<Value>> {
        let (task, handle) = iced_layershell::Task::perform(
            async {
                sleep(Duration::from_secs(1)).await;
            },
            |_| Message::<Value>::ShowReceived,
        )
        .abortable();
        self.timeout = Some(handle);
        task
    }
}

#[derive(Debug, Clone)]
pub enum Message<Value> {
    /// Emitted during user interaction
    Request(Value),
    /// Starts the timeout after which received value is shown
    Timeout,
    /// Starts timeout after every request for interactions without an end event
    RequestAndTimeout(Value),
    /// Internal message - should only be triggered by the timeout expiring
    ShowReceived,
}

impl<Value: Copy> Message<Value> {
    pub fn value(&self) -> Option<Value> {
        match self {
            Self::Request(value) => Some(*value),
            Self::RequestAndTimeout(value) => Some(*value),
            _ => Option::None,
        }
    }
}
