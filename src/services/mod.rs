use iced::Subscription;

pub mod battery;

pub trait Service {
    type Message;

    fn subscribe(&self) -> Subscription<Self::Message>;
}
