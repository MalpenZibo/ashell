use super::{ReadOnlyService, ServiceEvent};
use ashell_services::logind::{Logind, ResumeEvent};
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use std::{any::TypeId, pin::pin};

#[derive(Debug, Clone)]
pub struct LogindService;

impl ReadOnlyService for LogindService {
    type UpdateEvent = ResumeEvent;
    type Error = String;

    fn update(&mut self, _event: Self::UpdateEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        Subscription::run_with(TypeId::of::<Self>(), |_| {
            channel(100, async move |mut output| {
                let handle = match Logind::connect().await {
                    Ok(handle) => handle,
                    Err(e) => {
                        let err = format!("Failed to connect to system bus: {e}");
                        let _ = output.send(ServiceEvent::Error(err)).await;
                        return;
                    }
                };

                let _ = output.send(ServiceEvent::Init(LogindService)).await;

                let mut resumes = pin!(handle.resumes());
                while let Some(event) = resumes.next().await {
                    let _ = output.send(ServiceEvent::Update(event)).await;
                }
            })
        })
    }
}
