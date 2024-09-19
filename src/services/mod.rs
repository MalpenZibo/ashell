use iced::Subscription;

pub mod network;
pub mod upower;

#[derive(Debug, Clone)]
pub enum ServiceEvent<S: ReadOnlyService> {
    Init(S),
    Update(S::UpdateEvent),
    Error(S::Error),
}

pub trait Service: ReadOnlyService {
    type Command;

    fn command(
        &self,
        command: Self::Command,
    ) -> iced::Command<ServiceEvent<Self>>;
}

pub trait ReadOnlyService: Sized {
    type UpdateEvent;
    type Error: Clone;

    fn update(&mut self, event: Self::UpdateEvent);

    fn subscribe() -> Subscription<ServiceEvent<Self>>;
}
