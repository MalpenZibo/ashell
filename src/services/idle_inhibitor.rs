use log::{debug, info, warn};
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use wayland_client::{
    Connection, Dispatch, DispatchError, EventQueue, Proxy, QueueHandle,
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_registry::{self, WlRegistry},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

/// Create a 1×1 transparent ARGB `WlBuffer` via `wl_shm`.
fn create_transparent_buffer(
    shm: &WlShm,
    handle: &QueueHandle<IdleInhibitorManagerData>,
) -> Option<WlBuffer> {
    let name = c"ashell-idle-shm";
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        warn!("memfd_create failed; cannot create shm buffer for idle inhibitor");
        return None;
    }
    // SAFETY: we just created the fd above and own it.
    let file = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };

    // 4 bytes = one ARGB8888 pixel (all zeros = fully transparent).
    const PIXEL_SIZE: i32 = 4;
    unsafe {
        libc::ftruncate(file.as_fd().as_raw_fd(), PIXEL_SIZE as libc::off_t);
    }

    let pool = shm.create_pool(file.as_fd(), PIXEL_SIZE, handle, ());
    let buffer = pool.create_buffer(0, 1, 1, PIXEL_SIZE, wl_shm::Format::Argb8888, handle, ());
    pool.destroy();

    Some(buffer)
}

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

            // First roundtrip: discover and bind globals (compositor, idle
            // manager, layer shell, shm). The compositor handler also creates
            // the wl_surface we'll reuse for both the layer role and the
            // inhibitor.
            obj.roundtrip()?;

            // Give the surface a layer-shell role so it becomes properly
            // mapped. Spec-compliant compositors like niri ignore idle
            // inhibitors on unmapped surfaces.
            obj.init_layer_surface();

            // Second roundtrip: receive the layer-surface configure event,
            // ack it, attach the buffer, and commit — the surface is now
            // mapped on an output with content.
            obj.roundtrip()?;

            if !obj.data.surface_ready {
                warn!(
                    "Idle inhibitor surface was not configured; inhibitor may not work on spec-compliant compositors"
                );
            }

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

    fn init_layer_surface(&mut self) {
        let Some(surface) = &self.data.surface else {
            return;
        };
        let Some((layer_shell, _)) = &self.data.layer_shell else {
            warn!("Layer shell not available; idle inhibitor surface will remain unmapped");
            return;
        };

        // Create a transparent pixel buffer so the surface has content.
        // Without a buffer, some compositors won't assign a scanout output
        // and will ignore the idle inhibitor.
        if let Some((shm, _)) = &self.data.shm {
            self.data.buffer = create_transparent_buffer(shm, &self.handle);
        }

        let layer_surface = layer_shell.get_layer_surface(
            surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "ashell-idle-inhibitor".to_string(),
            &self.handle,
            (),
        );
        layer_surface.set_size(1, 1);
        layer_surface
            .set_anchor(zwlr_layer_surface_v1::Anchor::Top | zwlr_layer_surface_v1::Anchor::Left);
        layer_surface.set_exclusive_zone(-1);
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::None);
        surface.commit();

        self.data.layer_surface = Some(layer_surface);
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
    shm: Option<(WlShm, u32)>,
    buffer: Option<WlBuffer>,
    idle_manager: Option<(ZwpIdleInhibitManagerV1, u32)>,
    idle_inhibitor_state: Option<ZwpIdleInhibitorV1>,
    layer_shell: Option<(ZwlrLayerShellV1, u32)>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    surface_ready: bool,
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
                } else if interface == ZwlrLayerShellV1::interface().name
                    && state.layer_shell.is_none()
                {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding LayerShell with name {name} and version {version}");
                    state.layer_shell = Some((proxy.bind(name, version, handle, ()), name));
                } else if interface == WlShm::interface().name && state.shm.is_none() {
                    debug!(target: "IdleInhibitor::WlRegistry::Event::Global", "Adding Shm with name {name} and version {version}");
                    state.shm = Some((proxy.bind(name, version, handle, ()), name));
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some((_, n)) = &state.compositor
                    && name == *n
                {
                    warn!(target: "IdleInhibitor::GlobalRemove", "Compositor was removed!");
                    state.compositor = None;
                    state.surface = None;
                    state.surface_ready = false;
                } else if let Some((_, n)) = &state.idle_manager
                    && name == *n
                {
                    warn!(target: "IdleInhibitor::GlobalRemove", "IdleInhibitManager was removed!");
                    state.idle_manager = None;
                } else if let Some((_, n)) = &state.layer_shell
                    && name == *n
                {
                    warn!(target: "IdleInhibitor::GlobalRemove", "LayerShell was removed!");
                    state.layer_shell = None;
                    state.layer_surface = None;
                    state.surface_ready = false;
                }
            }
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
    }
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

impl Dispatch<WlShm, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShm,
        _event: <WlShm as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShmPool, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlShmPool,
        _event: <WlShmPool as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &WlBuffer,
        _event: <WlBuffer as Proxy>::Event,
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
    }
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
    }
}

impl Dispatch<ZwlrLayerShellV1, ()> for IdleInhibitorManagerData {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrLayerShellV1,
        _event: <ZwlrLayerShellV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for IdleInhibitorManagerData {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, .. } => {
                proxy.ack_configure(serial);
                if let Some(surface) = &state.surface {
                    if let Some(buffer) = &state.buffer {
                        surface.attach(Some(buffer), 0, 0);
                    }
                    surface.commit();
                }
                state.surface_ready = true;
                debug!(target: "IdleInhibitor::LayerSurface", "Surface configured and committed");
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.layer_surface = None;
                state.surface_ready = false;
                warn!(target: "IdleInhibitor::LayerSurface", "Layer surface was closed");
            }
            _ => {}
        }
    }
}
