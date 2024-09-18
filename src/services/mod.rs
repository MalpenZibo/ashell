use iced::Subscription;

pub mod battery;
pub mod network;

pub trait Service: ReadOnlyService {
    type Command;

    fn command<Message>(&self, command: Self::Command) -> iced::Command<Message>;
}

pub trait ReadOnlyService {
    type Data;
    type Event;

    fn subscribe() -> Subscription<Self::Event>;
}

