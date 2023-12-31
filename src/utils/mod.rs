use std::time::Duration;
use tokio::{spawn, task::JoinHandle, time::sleep};

pub mod launcher;
pub mod brightness;
pub mod audio;
pub mod net;
pub mod battery;

pub fn poll<F>(mut f: F, every: Duration) -> JoinHandle<()>
where
    F: FnMut() + Send + Sync + 'static,
{
    spawn(async move {
        loop {
            f();
            sleep(every).await;
        }
    })
}


