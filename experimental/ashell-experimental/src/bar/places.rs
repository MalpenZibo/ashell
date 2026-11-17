use guido::prelude::*;
use guido::tree::{LayoutCtx, WidgetId};
use guido::widget_prelude::{Constraints, Layout};

/// The three places of a wide node, as ashell has them: the ends at the ends,
/// the centre in the middle of the room. An end too long for its half pushes
/// the centre only as far as it has to, so the centre moves smoothly while a
/// group in an end grows; when the centre fits nowhere, it is in the middle
/// of the gap between the ends.
///
/// An end that fills reaches the centre.
pub(super) struct PlacesLayout {
    pub spacing: f32,
    /// `End` keeps the three on the bottom edge, anything else on the top.
    pub cross: CrossAlignment,
}

impl Layout for PlacesLayout {
    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        children: &[WidgetId],
        constraints: Constraints,
        origin: (f32, f32),
    ) -> Size {
        let &[start, center, end] = children else {
            return Size::zero();
        };
        let max_height = constraints.max_height;
        let lay_out = |ctx: &mut LayoutCtx, child, width: f32| {
            let room = Constraints::loose(Size::new(width.max(0.0), max_height));
            ctx.layout_child(child, room).unwrap_or_default()
        };

        // The ends' content, which a filling end is with no room to fill.
        let start_width = lay_out(ctx, start, f32::INFINITY).width;
        let end_width = lay_out(ctx, end, f32::INFINITY).width;

        let width = constraints.max_width;
        let (x, center_size) = if width.is_finite() {
            let center_size = lay_out(ctx, center, width - 2.0 * self.spacing);

            let low = start_width + self.spacing;
            let high = width - end_width - self.spacing - center_size.width;
            let middle = (width - center_size.width) / 2.0;
            let x = if low <= high {
                middle.clamp(low, high)
            } else {
                (low + high) / 2.0
            };

            (x, center_size)
        } else {
            // Measured for its content: the three one after the other.
            let center_size = lay_out(ctx, center, f32::INFINITY);

            (start_width + self.spacing, center_size)
        };
        let width = if width.is_finite() {
            width
        } else {
            x + center_size.width + self.spacing + end_width
        };

        let start_size = lay_out(ctx, start, x - self.spacing);
        let end_size = lay_out(ctx, end, width - x - center_size.width - self.spacing);

        let height = start_size
            .height
            .max(center_size.height)
            .max(end_size.height);
        let y = |child_height: f32| match self.cross {
            CrossAlignment::End => origin.1 + height - child_height,
            _ => origin.1,
        };

        let tree = ctx.tree();
        tree.set_origin(start, origin.0, y(start_size.height));
        tree.set_origin(center, origin.0 + x, y(center_size.height));
        tree.set_origin(end, origin.0 + width - end_size.width, y(end_size.height));

        constraints.constrain(Size::new(width, height))
    }
}
