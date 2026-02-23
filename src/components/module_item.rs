use guido::prelude::*;

#[component]
pub struct ModuleItem {
    #[prop(callback)]
    on_click: (),
    #[prop(children)]
    children: (),
}

impl ModuleItem {
    fn render(&self) -> impl Widget + use<> {
        let on_click = self.on_click.clone();
        let children = self.take_children();

        let mut c = container()
            .height(fill())
            .padding([0.0, 10.0])
            .corner_radius(16.0)
            .layout(Flex::row().cross_alignment(CrossAlignment::Center));

        if let Some(click_fn) = on_click {
            let hovered = create_signal(false);
            c = c
                .on_click(move || click_fn())
                .on_hover(move |h| hovered.set(h))
                .background(move || {
                    if hovered.get() {
                        Color::rgba(1.0, 1.0, 1.0, 0.1)
                    } else {
                        Color::TRANSPARENT
                    }
                });
        }

        c.children_source(children)
    }
}
