//! The services of `ashell-services` as signals: each one connected once,
//! its latest snapshot kept in a signal, its commands run off the UI thread.

use std::future::Future;

use ashell_services::bluetooth::{Bluetooth, BluetoothCommand, BluetoothData};
use ashell_services::brightness::{Brightness, BrightnessCommand, BrightnessData};
use ashell_services::upower::{UPower, UPowerCommand, UPowerData};
use futures::stream::{FuturesUnordered, Stream, StreamExt};
use guido::prelude::*;

/// What every service offers: a connection, its snapshots, its commands.
pub trait Backend: Clone + Send + Sync + Sized + 'static {
    type Data: Clone + PartialEq + Default + Send + Sync + 'static;
    type Command: Send + 'static;

    fn connect() -> impl Future<Output = anyhow::Result<Self>> + Send;

    fn updates(&self) -> impl Stream<Item = Self::Data> + Send + 'static;

    fn execute(&self, command: Self::Command) -> impl Future<Output = anyhow::Result<()>> + Send;
}

macro_rules! backend {
    ($service:ty => $data:ty, $command:ty) => {
        impl Backend for $service {
            type Data = $data;
            type Command = $command;

            async fn connect() -> anyhow::Result<Self> {
                <$service>::connect().await
            }

            fn updates(&self) -> impl Stream<Item = $data> + Send + 'static {
                <$service>::updates(self)
            }

            async fn execute(&self, command: $command) -> anyhow::Result<()> {
                <$service>::execute(self, command).await
            }
        }
    };
}

backend!(Bluetooth => BluetoothData, BluetoothCommand);
backend!(Brightness => Option<BrightnessData>, BrightnessCommand);
backend!(UPower => UPowerData, UPowerCommand);

/// A service as the UI sees it: its latest snapshot, the default one until
/// the first comes, and where its commands go.
pub struct Linked<B: Backend> {
    pub data: Signal<B::Data>,
    pub commands: Service<B::Command>,
}

impl<B: Backend> Clone for Linked<B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Backend> Copy for Linked<B> {}

/// Connects `B` for as long as the scope lives.
///
/// Commands run side by side, polled by the task that reads the snapshots,
/// so a long one (a discovery) holds nothing up. Those sent before the
/// connection is up wait for it, and go with it if it fails.
pub fn link<B: Backend>() -> Linked<B> {
    let data = create_signal(B::Data::default());
    let writer = data.writer();
    let name = std::any::type_name::<B>();

    let commands = create_service::<B::Command, _, _>(move |mut rx, _| async move {
        let backend = match B::connect().await {
            Ok(backend) => backend,
            Err(err) => {
                log::error!("{name}: not connected: {err}");
                return;
            }
        };

        let mut updates = std::pin::pin!(backend.updates());
        let mut running = FuturesUnordered::new();

        loop {
            tokio::select! {
                update = updates.next() => match update {
                    Some(update) => writer.set(update),
                    None => break,
                },
                command = rx.recv() => match command {
                    Some(command) => {
                        let backend = backend.clone();

                        running.push(async move { backend.execute(command).await });
                    }
                    None => break,
                },
                Some(result) = running.next(), if !running.is_empty() => {
                    if let Err(err) = result {
                        log::warn!("{name}: a command failed: {err}");
                    }
                }
            }
        }
    });

    Linked {
        data: data.read_only(),
        commands,
    }
}

/// `data` for good, its commands going nowhere: for pictures.
#[cfg(test)]
pub fn fixed<B: Backend>(data: B::Data) -> Linked<B> {
    Linked {
        data: create_signal(data).read_only(),
        commands: create_service::<B::Command, _, _>(|_, _| async {}),
    }
}
