use super::{App, Message};
use crate::{
    config::{self, ModuleName},
    ipc::{IpcCommand, IpcReply, IpcRequest, IpcResponse, NotificationInfo},
    modules::notifications::{self, CloseResult},
};
use iced::Task;

impl App {
    fn notifications_available(&self) -> bool {
        self.general_config
            .modules
            .contains(&ModuleName::Notifications)
            && self.notifications.is_available()
    }

    pub(super) fn handle_ipc(&mut self, request: IpcRequest) -> Task<Message> {
        let IpcRequest { command, reply } = request;
        if matches!(
            command,
            IpcCommand::NotificationsClear { .. }
                | IpcCommand::NotificationsInvoke { .. }
                | IpcCommand::NotificationsList
        ) && !self.notifications_available()
        {
            reply.send(IpcResponse::error(
                "Notifications module or service is unavailable",
            ));
            return Task::none();
        }

        match command {
            IpcCommand::NotificationsList => {
                reply.send(IpcResponse::Ok {
                    notifications: Some(
                        self.notifications
                            .list()
                            .map(NotificationInfo::from)
                            .collect(),
                    ),
                });
                Task::none()
            }
            IpcCommand::NotificationsClear { id } => {
                notification_reply(self.notifications.clear(id), reply)
            }
            IpcCommand::NotificationsInvoke { id, action } => {
                notification_reply(self.notifications.invoke(id, action), reply)
            }
            IpcCommand::LoadConfig { file } => {
                if !file.is_absolute() {
                    reply.send(IpcResponse::error("load-config expects an absolute path"));
                    return Task::none();
                }
                match config::read_config(&file) {
                    Ok(config) => {
                        self.config_path = file;
                        let task = self.update(Message::ConfigChanged(Box::new(config)));
                        reply.send(IpcResponse::ok());
                        task
                    }
                    Err(e) => {
                        reply.send(IpcResponse::error(format!(
                            "invalid config {}: {e}",
                            file.display()
                        )));
                        Task::none()
                    }
                }
            }
            IpcCommand::ToggleVisibility => {
                let task = self.update(Message::ToggleVisibility);
                reply.send(IpcResponse::ok());
                task
            }
            command @ (IpcCommand::VolumeUp { .. }
            | IpcCommand::VolumeDown { .. }
            | IpcCommand::VolumeToggleMute { .. }
            | IpcCommand::MicrophoneUp { .. }
            | IpcCommand::MicrophoneDown { .. }
            | IpcCommand::MicrophoneToggleMute { .. }
            | IpcCommand::BrightnessUp { .. }
            | IpcCommand::BrightnessDown { .. }
            | IpcCommand::ToggleAirplaneMode { .. }
            | IpcCommand::ToggleIdleInhibitor { .. }) => {
                let task = self.update(Message::IpcOsdCommand(command));
                reply.send(IpcResponse::ok());
                task
            }
        }
    }

    pub(super) fn finish_notifications_ipc(
        &mut self,
        result: CloseResult,
        reply: IpcReply,
    ) -> Task<Message> {
        let response = match &result.result {
            Ok(()) => IpcResponse::ok(),
            Err(error) => IpcResponse::error(error),
        };
        let task = self.update(Message::Notifications(
            notifications::Message::CloseFinished(result),
        ));
        reply.send(response);
        task
    }
}

fn notification_reply(task: anyhow::Result<Task<CloseResult>>, reply: IpcReply) -> Task<Message> {
    match task {
        Ok(task) => task.map(move |result| Message::IpcNotificationsFinished {
            result,
            reply: reply.clone(),
        }),
        Err(e) => {
            reply.send(IpcResponse::error(e));
            Task::none()
        }
    }
}
