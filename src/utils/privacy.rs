use std::thread;

use iced::{futures::SinkExt, Subscription};
use log::{debug, warn};
use pipewire::{context::Context, main_loop::MainLoop};

use crate::modules::privacy::{ApplicationNode, Media, PrivacyMessage};

pub fn subscription() -> Subscription<PrivacyMessage> {
    iced::subscription::channel(
        "privacy-dbus-connection-listener",
        100,
        |mut output| async move {
            enum PipewireEvent {
                AddNode(u32, Media, String),
                RemoveNode(u32),
            }

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<PipewireEvent>();

            thread::spawn(move || {
                let mainloop = MainLoop::new(None).unwrap();
                let context = Context::new(&mainloop).unwrap();
                let core = context.connect(None).unwrap();
                let registry = core.get_registry().unwrap();

                let _listener = registry
                    .add_listener_local()
                    .global({
                        let tx = tx.clone();
                        move |global| {
                            if let Some(props) = global.props {
                                if let Some(media) = props.get("media.class").filter(|v| {
                                    v == &"Stream/Input/Video" || v == &"Stream/Input/Audio"
                                }) {
                                    debug!("New global: {:?}", global);
                                    tx.send(PipewireEvent::AddNode(
                                        global.id,
                                        if media == "Stream/Input/Video" {
                                            Media::Video
                                        } else {
                                            Media::Audio
                                        },
                                        props.get("node.name").map_or_else(
                                            || "generic".to_string(),
                                            |name| name.to_lowercase(),
                                        ),
                                    ))
                                    .unwrap();
                                }
                            }
                        }
                    })
                    .global_remove({
                        let tx = tx.clone();
                        move |id| {
                            debug!("Remove global: {}", id);
                            tx.send(PipewireEvent::RemoveNode(id)).unwrap();
                        }
                    })
                    .register();

                mainloop.run();

                warn!("Pipewire mainloop exited");
            });

            let mut applications = vec![];
            loop {
                match rx.recv().await {
                    Some(PipewireEvent::AddNode(id, media, application)) => {
                        applications.push(ApplicationNode {
                            id,
                            media,
                            application,
                        });

                        output
                            .send(PrivacyMessage::Applications(applications.clone()))
                            .await
                            .unwrap();
                    }
                    Some(PipewireEvent::RemoveNode(id)) => {
                        applications.retain(|n| n.id != id);

                        output
                            .send(PrivacyMessage::Applications(applications.clone()))
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
        },
    )
}
