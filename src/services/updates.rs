use guido::prelude::*;
use log::error;
use std::process::Stdio;
use std::time::Duration;

use crate::config::UpdatesModuleConfig;

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, PartialEq, guido::SignalFields)]
pub struct UpdatesData {
    pub updates: Vec<Update>,
    pub is_checking: bool,
}

impl Default for UpdatesData {
    fn default() -> Self {
        Self {
            updates: Vec::new(),
            is_checking: true,
        }
    }
}

#[derive(Clone)]
pub enum UpdatesCmd {
    CheckNow,
    RunUpdate,
}

async fn check_updates_now(check_cmd: &str) -> Vec<Update> {
    let result = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(check_cmd)
        .stdout(Stdio::piped())
        .output()
        .await;

    match result {
        Ok(output) => {
            let cmd_output = String::from_utf8_lossy(&output.stdout);
            let mut updates = Vec::new();
            for line in cmd_output.split('\n') {
                if line.is_empty() {
                    continue;
                }
                let data: Vec<&str> = line.split(' ').collect();
                if data.len() < 4 {
                    continue;
                }
                updates.push(Update {
                    package: data[0].to_string(),
                    from: data[1].to_string(),
                    to: data[3].to_string(),
                });
            }
            updates
        }
        Err(e) => {
            error!("Error checking updates: {e:?}");
            vec![]
        }
    }
}

async fn run_update(update_cmd: &str) {
    if update_cmd.is_empty() {
        return;
    }
    let _ = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(update_cmd)
        .output()
        .await;
}

pub fn start_updates_service(
    writers: UpdatesDataWriters,
    config: UpdatesModuleConfig,
) -> Service<UpdatesCmd> {
    let check_cmd = config.check_cmd;
    let update_cmd = config.update_cmd;
    let interval = config.interval;

    create_service::<UpdatesCmd, _, _>(move |mut rx, ctx| {
        let check_cmd = check_cmd.clone();
        let update_cmd = update_cmd.clone();
        async move {
            // Initial check
            writers.set(UpdatesData {
                updates: Vec::new(),
                is_checking: true,
            });
            let updates = check_updates_now(&check_cmd).await;
            writers.set(UpdatesData {
                updates,
                is_checking: false,
            });

            while ctx.is_running() {
                tokio::select! {
                    cmd = rx.recv() => {
                        match cmd {
                            Some(UpdatesCmd::CheckNow) => {
                                writers.is_checking.set(true);
                                let updates = check_updates_now(&check_cmd).await;
                                writers.set(UpdatesData { updates, is_checking: false });
                            }
                            Some(UpdatesCmd::RunUpdate) => {
                                run_update(&update_cmd).await;
                                // Re-check after update
                                writers.is_checking.set(true);
                                let updates = check_updates_now(&check_cmd).await;
                                writers.set(UpdatesData { updates, is_checking: false });
                            }
                            None => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(interval)) => {
                        writers.is_checking.set(true);
                        let updates = check_updates_now(&check_cmd).await;
                        writers.set(UpdatesData { updates, is_checking: false });
                    }
                }
            }
        }
    })
}
