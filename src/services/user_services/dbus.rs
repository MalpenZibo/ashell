use zbus::zvariant::OwnedObjectPath;

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait Systemd1Manager {
    /// ListUnitFiles returns an array of (path, state) pairs.
    fn list_unit_files(&self) -> zbus::Result<Vec<(String, String)>>;

    /// ListUnits returns an array of
    /// (name, description, load_state, active_state, sub_state,
    ///  followed_by, object_path, queued_job_id, job_type, job_object_path).
    #[allow(clippy::type_complexity)]
    fn list_units(
        &self,
    ) -> zbus::Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            OwnedObjectPath,
            u32,
            String,
            OwnedObjectPath,
        )>,
    >;

    /// LoadUnit asks systemd to load a unit by name and returns its object path.
    fn load_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;

    /// StartUnit starts a unit with the given mode.
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;

    /// StopUnit stops a unit with the given mode.
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;

    /// Subscribe to systemd signals so we receive PropertiesChanged.
    fn subscribe(&self) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
pub trait Systemd1Unit {
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn description(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn load_state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn sub_state(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn unit_file_state(&self) -> zbus::Result<String>;
}
