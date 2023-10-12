use gdk4::{
    prelude::{ApplicationExt, ApplicationExtManual, DisplayExt, ListModelExtManual, MonitorExt},
    Display, Monitor,
};

use gtk4::{
    traits::{GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use gtk4_layer_shell::LayerShell;

use super::Node;

pub struct App {
    name: Option<&'static str>,
    css_path: String,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub application: Application,
    pub window: ApplicationWindow,
}

impl Context {
    pub fn open_surface<F: FnOnce(Context) -> Node>(
        &self,
        surface: Surface,
        declare_node: F,
    ) -> (ApplicationWindow, Node) {
        let window = surface.build(&self.application);

        let root = declare_node(Context {
            application: self.application.clone(),
            window: window.clone(),
        });

        window.set_child(Some(&root.component));

        window.show();

        (window, root)
    }
}

impl App {
    pub fn new(name: Option<&'static str>) -> Self {
        Self {
            name,
            css_path: String::new(),
        }
    }

    pub fn run<F: FnOnce(Context) -> Node + Copy + 'static>(
        self,
        main_surface: Surface,
        declare_node: F,
    ) {
        let application = gtk4::Application::new(self.name, Default::default());

        application.connect_startup(move |app| {
            // The CSS "magic" happens here.
            let provider = CssProvider::new();
            provider.load_from_data(grass::include!("./src/style.scss"));
            // We give the CssProvided to the default screen so the CSS rules we added
            // can be applied to our window.
            gtk4::style_context_add_provider_for_display(
                &Display::default().expect("Could not connect to a display."),
                &provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            let window = main_surface.build(app);

            let mut root = declare_node(Context {
                application: app.clone(),
                window: window.clone().into(),
            });

            root.handlers.clear();

            window.set_child(Some(&root.component));

            app.connect_activate(move |_| window.show());
        });

        application.run();
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SurfaceType {
    Window,
    Layer {
        anchors: (bool, bool, bool, bool),
        monitor: Option<&'static str>,
        exclusive: bool,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct Surface {
    width: Option<u32>,
    height: Option<u32>,
    r#type: SurfaceType,
}

impl Surface {
    pub fn window() -> Self {
        Surface {
            width: None,
            height: None,
            r#type: SurfaceType::Window,
        }
    }

    pub fn layer(
        exclusive: bool,
        anchors: (bool, bool, bool, bool),
        monitor: Option<&'static str>,
    ) -> Self {
        Surface {
            width: None,
            height: None,
            r#type: SurfaceType::Layer {
                anchors,
                monitor,
                exclusive,
            },
        }
    }

    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn build(self, app: &Application) -> ApplicationWindow {
        let window = gtk4::ApplicationWindow::new(app);
        window.set_default_size(
            self.width.map(|w| w as i32).unwrap_or(-1),
            self.height.map(|h| h as i32).unwrap_or(-1),
        );

        match self.r#type {
            SurfaceType::Layer {
                exclusive,
                monitor,
                anchors,
                ..
            } => {
                window.init_layer_shell();
                window.set_layer(gtk4_layer_shell::Layer::Overlay);

                if exclusive {
                    window.auto_exclusive_zone_enable();
                }

                if let Some(requested_monitor) = monitor {
                    let display = Display::default().expect("Failed to get default display");
                    let monitors = display.monitors();

                    let mut target: Option<Monitor> = None;
                    for m in monitors.iter::<Monitor>() {
                        let monitor = m.unwrap();
                        let connector = monitor.connector().unwrap();
                        if connector == requested_monitor {
                            target = Some(monitor);
                        }
                    }

                    if let Some(target) = target {
                        window.set_monitor(&target);
                    }
                }

                let (left, top, right, bottom) = anchors;
                if left {
                    window.set_anchor(gtk4_layer_shell::Edge::Left, true);
                }
                if top {
                    window.set_anchor(gtk4_layer_shell::Edge::Top, true);
                }
                if right {
                    window.set_anchor(gtk4_layer_shell::Edge::Right, true);
                }
                if bottom {
                    window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
                }
            }
            SurfaceType::Window => {}
        };

        window
    }
}
