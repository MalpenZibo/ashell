use std::collections::HashMap;
use std::ops::Deref;
use zbus::{Result, proxy, zvariant::OwnedValue};

pub struct MprisPlayerDbus<'a>(MprisPlayerProxy<'a>);

impl<'a> Deref for MprisPlayerDbus<'a> {
    type Target = MprisPlayerProxy<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait MprisPlayer {
    fn next(&self) -> Result<()>;
    fn play_pause(&self) -> Result<()>;
    fn previous(&self) -> Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> Result<String>;
    #[zbus(property)]
    fn metadata(&self) -> Result<HashMap<String, OwnedValue>>;
    #[zbus(property)]
    fn set_volume(&self, v: f64) -> Result<()>;
    #[zbus(property)]
    fn volume(&self) -> Result<f64>;
    #[zbus(property)]
    fn can_control(&self) -> Result<bool>;
}
