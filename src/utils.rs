use std::time::Duration;

use tokio::{spawn, task::JoinHandle, time::sleep};

pub fn poll(f: impl Fn() + Send + Sync + 'static, every: Duration) -> JoinHandle<()> {
    spawn(async move {
        loop {
            f();
            sleep(every).await;
        }
    })
}
