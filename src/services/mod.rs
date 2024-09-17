use iced::Subscription;

pub mod battery;
pub mod network;

pub trait Service {
    type Data;
    type Message;

    fn subscribe() -> Subscription<Self::Message>;
}
