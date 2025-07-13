use log::{debug, info, warn};
use wayland_client::{
    Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle,
    protocol::{
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};

pub struct IdleInhibitorManager {
    _connection: Connection,
    _display: WlDisplay,
    _registry: WlRegistry,
    event_queue: EventQueue<IdleInhibitorManagerData>,
    handle: QueueHandle<IdleInhibitorManagerData>,
    data: IdleInhibitorManagerData,
}

impl IdleInhibitorManager {
    pub fn new() -> Option<Self> {
        let init = || -> anyhow::Result<Self> {
            let connection = Connection::connect_to_env()?;
            let display = connection.display();
            let event_queue = connection.new_event_queue();
            let handle = event_queue.handle();
            let registry = display.get_registry(&handle, ());

            let mut obj = Self {
                _connection: connection,
                _display: display,
                _registry: registry,
                event_queue,
                handle,
                data: IdleInhibitorManagerData::default(),
            };

            obj.roundtrip()?;

            Ok(obj)
        };

        match init() {
            Ok(obj) => Some(obj),
            Err(err) => {
                warn!("Failed to initialize idle inhibitor: {err}");
                None
            }
        }
    }

    fn roundtrip(&mut self) -> anyhow::Result<usize, DispatchError> {
        self.event_queue.roundtrip(&mut self.data)
    }

    pub fn is_inhibited(&self) -> bool {
        self.data.idle_inhibitor_state.is_some()
    }

    pub fn toggle(&mut self) {
        let res = if self.is_inhibited() {
            self.set_inhibit_idle(false)
        } else {
            self.set_inhibit_idle(true)
        };

        if let Err(err) = res {
            warn!("Failed to toggle idle inhibitor: {err}");
        }
    }

    fn set_inhibit_idle(&mut self, inhibit_idle: bool) -> anyhow::Result<()> {
        let data = &self.data;
        let Some((idle_manager, _)) = &data.idle_manager else {
            warn!(target: "IdleInhibitor::set_inhibit_idle", "Tried to change idle inhibitor status without loaded idle inhibitor manager!");
            return Ok(());
        };

        if inhibit_idle {
            if data.idle_inhibitor_state.is_none() {
                let Some(surface) = &data.surface else {
                    warn!(target: "IdleInhibitor::set_inhibit_idle", "Tried to change idle inhibitor status without loaded WlSurface!");
                    return Ok(());
                };
                self.data.idle_inhibitor_state =
                    Some(idle_manager.create_inhibitor(surface, &self.handle, ()));

                self.roundtrip()?;
                info!(target: "IdleInhibitor::set_inhibit_idle", "Idle Inhibitor was ENABLED");
            }
        } else if let Some(state) = &self.data.idle_inhibitor_state {
            state.destroy();
            self.data.idle_inhibitor_state = None;

            self.roundtrip()?;
            info!(target: "IdleInhibitor::set_inhibit_idle", "Idle Inhibitor was DISABLED");
        }

        Ok(())
    }
}

#[derive(Default)]
struct IdleInhibitorManagerData {
    compositor: Option<(WlCompositor, u32)>,
    surface: Option<WlSurface>,
    idle_manager: Option<(ZwpIdleInhibitManagerV1, u32)>,
    idle_inhibitor_state: Option<ZwpIdleInhibitorV1>,
}

impl Dispatch<WlRegistry, ()> for IdleInhibitorManagerData {
    fn event(
        state: &mut Self,
        proxy: &WlRegistry,
        event: <WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        handle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == WlCompositor::interface().name && state.compositor.is_none() {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding Compositor with name {name} and version {version}");
                    let compositor: WlCompositor = proxy.bind(name, version, handle, ());

                    state.surface = Some(compositor.create_surface(handle, ()));
                    state.compositor = Some((compositor, name));
                } else if interface == ZwpIdleInhibitManagerV1::interface().name
                    && state.idle_manager.is_none()
                {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding IdleInhibitManager with name {name} and version {version}");
                    state.idle_manager = Some((proxy.bind(name, version, handle, ()), name));
                };
            }
            wl_registry::Event::GlobalRemove { name } => match &state.compositor {
                Some((_, compositor_name)) => {
                    if name == *compositor_name {
                        warn!(target: "IdleInhibitor::GlobalRemove", "Compositor was removed!");

                        state.compositor = None;
                        state.surface = None;
                    }
                }
                _ => {
                    if let Some((_, idle_manager_name)) = &state.idle_manager {
                        if name == *idle_manager_name {
                            warn!(target: "IdleInhibitor::GlobalRemove", "IdleInhibitManager was removed!");

                            state.idle_manager = None;
                        }
                    }
                }
            },
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlCompositor,
        _event: <WlCompositor as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<WlSurface, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlSurface,
        _event: <WlSurface as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitManagerV1,
        _event: <ZwpIdleInhibitManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
    } // This interface has no events.
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpIdleInhibitorV1,
        _event: <ZwpIdleInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    } // This interface has no events.
}
