use guido::prelude::*;

#[component]
pub fn module_item(
    #[prop(callback)]
    on_click: (),
    #[prop(children)]
    children: (),
) -> impl Widget {
    let on_click = on_click.clone();

    let mut c = container()
        .height(fill())
        .padding([0, 8])
        .corner_radius(16)
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
