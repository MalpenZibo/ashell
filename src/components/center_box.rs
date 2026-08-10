use guido::layout::{Constraints, Layout, Size};
use guido::prelude::*;
use guido::tree::{Tree, WidgetId};

// -- Layout -----------------------------------------------------------------

struct CenterBoxLayout {
    child_sizes: Vec<Size>,
}

impl CenterBoxLayout {
    fn new() -> Self {
        Self {
            child_sizes: Vec::with_capacity(3),
        }
    }
}

impl Layout for CenterBoxLayout {
    fn layout(
        &mut self,
        tree: &mut Tree,
        children: &[WidgetId],
        constraints: Constraints,
        origin: (f32, f32),
    ) -> Size {
        let available_width = constraints.max_width;
        let available_height = constraints.max_height;

        // Only use first 3 children
        let children = &children[..children.len().min(3)];

        // Measure the sides first with loose constraints; the center then
        // gets only the space they leave, so a long window title clips
        // instead of running underneath the side sections
        const SIDE_GAP: f32 = 8.0;
        let loose = Constraints::loose(Size::new(available_width, available_height));
        let measure = |tree: &mut Tree, id: WidgetId, c: Constraints| {
            tree.with_widget_mut(id, |widget, id, tree| widget.layout(tree, id, c))
                .unwrap_or_else(Size::zero)
        };

        let left_size = children
            .first()
            .map_or_else(Size::zero, |&id| measure(tree, id, loose));
        let right_size = children
            .get(2)
            .map_or_else(Size::zero, |&id| measure(tree, id, loose));

        let center_max =
            (available_width - left_size.width - right_size.width - 2.0 * SIDE_GAP).max(0.0);
        let center_constraints = Constraints::loose(Size::new(center_max, available_height));
        let center_size = children
            .get(1)
            .map_or_else(Size::zero, |&id| measure(tree, id, center_constraints));

        self.child_sizes.clear();
        self.child_sizes
            .extend([left_size, center_size, right_size]);

        let left_width = left_size.width;
        let center_width = center_size.width;
        let right_width = right_size.width;

        // Position left at origin
        if !children.is_empty() {
            let left_h = self.child_sizes[0].height;
            let y = origin.1 + (available_height - left_h) / 2.0;
            tree.set_origin(children[0], origin.0, y);
        }

        // Position right at far end
        if children.len() > 2 {
            let right_h = self.child_sizes[2].height;
            let y = origin.1 + (available_height - right_h) / 2.0;
            let x = origin.0 + available_width - right_width;
            tree.set_origin(children[2], x, y);
        }

        // Position center
        if children.len() > 1 {
            let center_h = self.child_sizes[1].height;
            let y = origin.1 + (available_height - center_h) / 2.0;

            let half_available = available_width / 2.0;
            let half_center = center_width / 2.0;

            let x = if half_available - left_width < half_center
                || half_available - right_width < half_center
            {
                // Not enough room for true centering — center in gap between left and right
                let gap_start = left_width;
                let gap_end = available_width - right_width;
                let gap = gap_end - gap_start;
                origin.0 + gap_start + (gap - center_width).max(0.0) / 2.0
            } else {
                // True center
                origin.0 + half_available - half_center
            };

            tree.set_origin(children[1], x, y);
        }

        // Return full available size
        constraints.constrain(Size::new(available_width, available_height))
    }
}

// -- Component --------------------------------------------------------------

#[component]
pub fn center_box(
    #[prop(slot)] left: (),
    #[prop(slot)] center: (),
    #[prop(slot)] right: (),
) -> impl Widget {
    container()
        .width(fill())
        .height(fill())
        .layout(CenterBoxLayout::new())
        .padding([0, 4])
        .child(left.unwrap_or_else(|| Box::new(container())))
        .child(center.unwrap_or_else(|| Box::new(container())))
        .child(right.unwrap_or_else(|| Box::new(container())))
}
