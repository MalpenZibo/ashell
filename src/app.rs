use gdk4::Display;
use gtk4::Application;
use gtk4::{prelude::*, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use crate::reactive_gtk::{Node, AsyncContext};

pub struct LayerOption {
    pub r#type: Layer,
    pub exclusive_zone: bool,
    pub top_anchor: bool,
    pub bottom_anchor: bool,
    pub left_anchor: bool,
    pub right_anchor: bool,
}

impl LayerOption {
    fn setup_window(&self, window: &gtk4::ApplicationWindow) {
        window.init_layer_shell();
        window.set_layer(self.r#type);

        if self.exclusive_zone {
            window.auto_exclusive_zone_enable();
        }

        let anchors = [
            (Edge::Left, self.left_anchor),
            (Edge::Right, self.right_anchor),
            (Edge::Top, self.top_anchor),
            (Edge::Bottom, self.bottom_anchor),
        ];

        for (anchor, state) in anchors {
            window.set_anchor(anchor, state);
        }
    }
}

pub struct App {
    title: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    layer_option: Option<LayerOption>,
    gtk_application: Application,
}

impl App {
    pub fn new(name: &str) -> Self {
        let gtk_application = Application::builder().application_id(name).build();

        Self {
            title: None,
            width: None,
            height: None,
            layer_option: None,
            gtk_application,
        }
    }

    pub fn set_title(mut self, title: String) -> Self {
        self.title = Some(title);

        self
    }

    pub fn set_width(mut self, width: i32) -> Self {
        self.width = Some(width);

        self
    }

    pub fn set_height(mut self, height: i32) -> Self {
        self.height = Some(height);

        self
    }

    pub fn set_layer_option(mut self, layer_option: LayerOption) -> Self {
        self.layer_option = Some(layer_option);

        self
    }

    pub fn run<N: Into<Node>, F: Fn(AppCtx) -> N + 'static>(self, root: F) {
        let build_ui = move |app: &Application| {
            let window = gtk4::ApplicationWindow::new(app);
            window.set_title(self.title.as_deref());
            window.set_default_size(self.width.unwrap_or(-1), self.height.unwrap_or(-1));

            if let Some(layer_option) = &self.layer_option {
                layer_option.setup_window(&window);
            }

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

            let mut root = root(AppCtx(app.clone())).into();

            root.get_ctx().forget();

            window.set_child(Some(root.get_widget()));

            // Present window
            window.present();
        };

        // Connect to "activate" signal of `app`
        self.gtk_application.connect_activate(build_ui);

        // Run the application
        self.gtk_application.run();
    }
}

pub struct AppCtx(Application);

#[derive(Clone)]
pub struct CloseHandle(Rc<dyn Fn()>);

impl Deref for CloseHandle {
    type Target = Rc<dyn Fn()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppCtx {
    pub fn open_window<N: Into<Node>>(
        &self,
        root: impl FnOnce(CloseHandle) -> N,
        layer: Option<LayerOption>,
    ) -> CloseHandle {
        let window = gtk4::ApplicationWindow::new(&self.0);
        if let Some(layer) = layer {
            layer.setup_window(&window);
        }

        let ctx = Rc::new(RefCell::new(AsyncContext::default()));

        let close_window = CloseHandle({
            let window = window.clone();
            let ctx = ctx.clone();
            Rc::new(move || {
                window.close();
                ctx.borrow_mut().cancel();
            })
        });

        let mut root = root(close_window.clone()).into();

        ctx.borrow_mut().consume(root.get_ctx());
        
        window.set_child(Some(root.get_widget()));
        window.show();

        close_window
    }
}

