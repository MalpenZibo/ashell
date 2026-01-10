use super::{ReadOnlyService, ServiceEvent};
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt},
    stream::channel,
};
use std::any::TypeId;
use zbus::Connection;

#[derive(Debug, Clone)]
pub struct ResumeEvent;

#[derive(Debug, Clone)]
pub struct LogindService;

impl ReadOnlyService for LogindService {
    type UpdateEvent = ResumeEvent;
    type Error = String;

    fn update(&mut self, _event: Self::UpdateEvent) {}

    fn subscribe() -> Subscription<ServiceEvent<Self>> {
        let id = TypeId::of::<Self>();

        Subscription::run_with_id(
            id,
            channel(100, async move |mut output| {
                let connection = match Connection::system().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        let err = format!("Failed to connect to system bus: {e}");
                        let _ = output.send(ServiceEvent::Error(err)).await;
                        return;
                    }
                };

                let proxy = match Login1ManagerProxy::new(&connection).await {
                    Ok(p) => p,
                    Err(e) => {
                        let err = format!("Failed to create logind proxy: {e}");
                        let _ = output.send(ServiceEvent::Error(err)).await;
                        return;
                    }
                };

                let mut stream = match proxy.receive_prepare_for_sleep().await {
                    Ok(s) => s,
                    Err(e) => {
                        let err = format!("Failed to subscribe to PrepareForSleep: {e}");
                        let _ = output.send(ServiceEvent::Error(err)).await;
                        return;
                    }
                };

                let _ = output.send(ServiceEvent::Init(LogindService)).await;

                while let Some(signal) = stream.next().await {
                    if let Ok(args) = signal.args()
                        && !args.starting
                    {
                        let _ = output.send(ServiceEvent::Update(ResumeEvent)).await;
                    }
                }
            }),
        )
    }
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, starting: bool) -> ();
}
