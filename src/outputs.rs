use iced::{
    platform_specific::shell::commands::layer_surface::{destroy_layer_surface, get_layer_surface},
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    window::Id,
    Task,
};
use log::debug;
use sctk::{
    output::OutputInfo,
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer},
};
use wayland_client::protocol::wl_output::WlOutput;

use crate::{config::Position, menu::Menu, HEIGHT};

#[derive(Debug, Default, Clone)]
pub struct Outputs(Vec<(Id, Menu, Option<WlOutput>)>);

impl Outputs {
    pub fn has(&self, id: Id) -> bool {
        self.0.iter().any(|(layer_id, _)| *layer_id == id)
    }

    pub fn add<Message: 'static>(
        &mut self,
        request_outputs: &[String],
        position: Position,
        output_info: Option<OutputInfo>,
        wl_output: WlOutput,
    ) -> Task<Message> {
        debug!("request_outputs: {:?}", request_outputs);
        let target = request_outputs.iter().any(|output| {
            Some(output.as_str()) == output_info.as_ref().and_then(|info| info.name.as_deref())
        });
        debug!("target: {:?}", target);
        let id = Id::unique();

        if self.0.is_empty() {
            let cmd = get_layer_surface(SctkLayerSurfaceSettings {
                id,
                size: Some((None, Some(HEIGHT))),
                layer: Layer::Top,
                pointer_interactivity: true,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: HEIGHT as i32,
                output: IcedOutput::Output(wl_output.clone()),
                anchor: match position {
                    Position::Top => Anchor::TOP,
                    Position::Bottom => Anchor::BOTTOM,
                } | Anchor::LEFT
                    | Anchor::RIGHT,
                ..Default::default()
            });

            let menu_id = Id::unique();
            let menu = get_layer_surface(SctkLayerSurfaceSettings {
                id: menu_id,
                size: Some((None, None)),
                layer: Layer::Background,
                pointer_interactivity: true,
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                ..Default::default()
            });

            self.0.push((id, target.then_some(wl_output)));

            cmd
        } else if target {
            let create_cmd = get_layer_surface(SctkLayerSurfaceSettings {
                id,
                size: Some((None, Some(HEIGHT))),
                pointer_interactivity: true,
                keyboard_interactivity: KeyboardInteractivity::None,
                exclusive_zone: HEIGHT as i32,
                output: IcedOutput::Output(wl_output.clone()),
                anchor: match position {
                    Position::Top => Anchor::TOP,
                    Position::Bottom => Anchor::BOTTOM,
                } | Anchor::LEFT
                    | Anchor::RIGHT,
                ..Default::default()
            });

            self.0.push((id, Some(wl_output)));

            if let Some(index) = self.0.iter().position(|(_, wl_output)| wl_output.is_none()) {
                let (id, _) = self.0.swap_remove(index);
                let destroy_cmd = destroy_layer_surface(id);

                Task::batch(vec![create_cmd, destroy_cmd])
            } else {
                create_cmd
            }
        } else {
            Task::none()
        }
    }

    pub fn remove<Message: 'static>(
        &mut self,
        position: Position,
        wl_output: WlOutput,
    ) -> Task<Message> {
        if let Some(to_remove) = self
            .0
            .iter()
            .position(|(_, output)| output.as_ref() == Some(&wl_output))
        {
            let (id, _) = self.0.swap_remove(to_remove);

            let destroy_cmd = destroy_layer_surface(id);

            if self.0.is_empty() {
                let id = Id::unique();
                let create_cmd = get_layer_surface(SctkLayerSurfaceSettings {
                    id,
                    size: Some((None, Some(HEIGHT))),
                    pointer_interactivity: true,
                    keyboard_interactivity: KeyboardInteractivity::None,
                    exclusive_zone: HEIGHT as i32,
                    output: IcedOutput::Active,
                    anchor: match position {
                        Position::Top => Anchor::TOP,
                        Position::Bottom => Anchor::BOTTOM,
                    } | Anchor::LEFT
                        | Anchor::RIGHT,
                    ..Default::default()
                });

                self.0.push((id, None));

                Task::batch(vec![destroy_cmd, create_cmd])
            } else {
                destroy_cmd
            }
        } else {
            Task::none()
        }
    }
}
