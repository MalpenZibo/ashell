//! Ported from ashell's `utils::remote_value`, minus the iced Task-based
//! timeout flow (`update`/`Message`): the guido UI layer manages slider
//! interaction state itself, services only need `new`/`receive`/`value`.

/// A helper for ensuring responsive user interface,
/// when handling async state
#[derive(Debug, Default, Clone)]
pub struct Remote<Value> {
    /// Requested value is immediately displayed, ensuring responsive user interface
    requested: Option<Value>,
    /// Source of truth. Displayed shortly after the end of the user interaction
    received: Value,
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
}
