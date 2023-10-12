use gdk4::Display;
use gtk4::{prelude::*, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4::{Application, ApplicationWindow, Widget};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub struct LayerOption {
    pub r#type: Layer,
    pub exclusive_zone: bool,
    pub top_anchor: bool,
    pub bottom_anchor: bool,
    pub left_anchor: bool,
    pub right_anchor: bool,
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

    pub fn run<F: Fn() -> Widget + 'static>(self, root: F) {
        let build_ui = move |app: &Application| {
            let window = ApplicationWindow::new(app);
            window.set_title(self.title.as_deref());
            window.set_default_size(self.width.unwrap_or(-1), self.height.unwrap_or(-1));

            if let Some(layer_option) = &self.layer_option {
                window.init_layer_shell();
                window.set_layer(layer_option.r#type);

                if layer_option.exclusive_zone {
                    window.auto_exclusive_zone_enable();
                }

                let anchors = [
                    (Edge::Left, layer_option.left_anchor),
                    (Edge::Right, layer_option.right_anchor),
                    (Edge::Top, layer_option.top_anchor),
                    (Edge::Bottom, layer_option.bottom_anchor),
                ];

                for (anchor, state) in anchors {
                    window.set_anchor(anchor, state);
                }
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

            window.set_child(Some(&root()));

            // Present window
            window.present();
        };

        // Connect to "activate" signal of `app`
        self.gtk_application.connect_activate(build_ui);

        // Run the application
        self.gtk_application.run();
    }
}
