use iced::{
    platform_specific::shell::commands::layer_surface::{destroy_layer_surface, get_layer_surface},
    runtime::platform_specific::wayland::layer_surface::{IcedOutput, SctkLayerSurfaceSettings},
    window::Id,
    Task,
};
use log::debug;
use sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

use crate::{app::MenuType, config::Position, menu::Menu, HEIGHT};

type ActiveOutput = (Id, Menu, Option<(String, WlOutput)>);

#[derive(Debug, Default, Clone)]
pub struct Outputs {
    active: Vec<ActiveOutput>,
    inactive: Vec<(String, WlOutput)>,
}

pub enum HasOutput {
    Main,
    Menu(Option<MenuType>),
}

impl Outputs {
    fn create_output_layers<Message: 'static>(
        wl_output: WlOutput,
        position: Position,
    ) -> (Id, Id, Task<Message>) {
        let id = Id::unique();
        let task = get_layer_surface(SctkLayerSurfaceSettings {
            id,
            size: Some((None, Some(HEIGHT))),
            layer: Layer::Bottom,
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
        let menu_task = get_layer_surface(SctkLayerSurfaceSettings {
            id: menu_id,
            size: Some((None, None)),
            layer: Layer::Background,
            pointer_interactivity: true,
            keyboard_interactivity: KeyboardInteractivity::None,
            output: IcedOutput::Output(wl_output.clone()),
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            ..Default::default()
        });

        (id, menu_id, Task::batch(vec![task, menu_task]))
    }

    pub fn has(&self, id: Id) -> Option<HasOutput> {
        self.active.iter().find_map(|(layer_id, menu, _)| {
            if *layer_id == id {
                Some(HasOutput::Main)
            } else if menu.id == id {
                Some(HasOutput::Menu(menu.menu_type))
            } else {
                None
            }
        })
    }

    pub fn add<Message: 'static>(
        &mut self,
        request_outputs: &[String],
        position: Position,
        name: &str,
        wl_output: WlOutput,
    ) -> Task<Message> {
        let target = request_outputs.iter().any(|output| output.as_str() == name);

        if self.active.is_empty() {
            debug!(
                "No outputs, creating a new layer surface. Is a fallback surface {}",
                target
            );

            let (id, menu_id, task) = Self::create_output_layers(wl_output.clone(), position);

            self.active
                .push((id, Menu::new(menu_id), Some((name.to_owned(), wl_output))));

            task
        } else if target {
            debug!("Found target output, creating a new layer surface");

            let (id, menu_id, task) = Self::create_output_layers(wl_output.clone(), position);

            self.active
                .push((id, Menu::new(menu_id), Some((name.to_owned(), wl_output))));

            if let Some(index) = self
                .active
                .iter()
                .position(|(_, _, wl_output)| wl_output.is_none())
            {
                debug!("Found fallback output, removing it");

                let (id, menu, wl_output) = self.active.swap_remove(index);
                let destroy_main_task = destroy_layer_surface(id);
                let destroy_menu_task = destroy_layer_surface(menu.id);

                if let Some(wl_output) = wl_output {
                    self.inactive.push(wl_output);
                }

                Task::batch(vec![task, destroy_main_task, destroy_menu_task])
            } else {
                task
            }
        } else {
            self.inactive.push((name.to_owned(), wl_output));

            Task::none()
        }
    }

    pub fn remove<Message: 'static>(
        &mut self,
        position: Position,
        wl_output: WlOutput,
    ) -> Task<Message> {
        if let Some(to_remove) = self.active.iter().position(|(_, _, output)| {
            output.as_ref().map(|(_, wl_output)| wl_output) == Some(&wl_output)
        }) {
            debug!("Removing layer surface for output");
            let (id, menu, old_wl_output) = self.active.swap_remove(to_remove);

            let destroy_main_task = destroy_layer_surface(id);
            let destroy_menu_task = destroy_layer_surface(menu.id);

            if let Some(wl_output) = old_wl_output {
                self.inactive.push(wl_output);
            }

            if self.active.is_empty() {
                debug!("No outputs left, creating a fallback layer surface");
                let (id, menu_id, task) = Self::create_output_layers(wl_output.clone(), position);

                self.active.push((id, Menu::new(menu_id), None));

                Task::batch(vec![destroy_main_task, destroy_menu_task, task])
            } else {
                Task::batch(vec![destroy_main_task, destroy_menu_task])
            }
        } else {
            Task::none()
        }
    }

    pub fn sync<Message: 'static>(
        &mut self,
        request_outputs: &[String],
        position: Position,
    ) -> Task<Message> {
        debug!(
            "Syncing outputs: {:?}, request_outputs: {:?}",
            self, request_outputs
        );

        let to_remove = self
            .active
            .iter()
            .filter_map(|(_, _, output)| {
                if let Some((name, wl_output)) = output {
                    if !request_outputs.iter().any(|output| output.as_str() == name) {
                        Some(wl_output.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Removing outputs: {:?}", to_remove);

        let to_add = self
            .inactive
            .iter()
            .filter_map(|(name, wl_output)| {
                if request_outputs.iter().any(|output| output.as_str() == name) {
                    Some((name.clone(), wl_output.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        debug!("Adding outputs: {:?}", to_add);

        let mut tasks = Vec::new();
        for wl_output in to_remove {
            tasks.push(self.remove(position, wl_output));
        }

        for (name, wl_output) in to_add {
            tasks.push(self.add(request_outputs, position, &name, wl_output));
        }

        Task::batch(tasks)
    }

    pub fn toggle_menu<Message: 'static>(&mut self, id: Id, menu_type: MenuType) -> Task<Message> {
        if let Some((_, menu, _)) = self
            .active
            .iter_mut()
            .find(|(layer_id, menu, _)| *layer_id == id || menu.id == id)
        {
            let toggle_task = menu.toggle(menu_type);
            let mut tasks = self
                .active
                .iter_mut()
                .filter_map(|(layer_id, menu, _)| {
                    if *layer_id != id && menu.id != id {
                        Some(menu.close())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            tasks.push(toggle_task);
            Task::batch(tasks)
        } else {
            Task::none()
        }
    }

    pub fn close_menu<Message: 'static>(&mut self, id: Id) -> Task<Message> {
        if let Some((_, menu, _)) = self
            .active
            .iter_mut()
            .find(|(layer_id, menu, _)| *layer_id == id || menu.id == id)
        {
            menu.close()
        } else {
            Task::none()
        }
    }

    pub fn close_menu_if<Message: 'static>(
        &mut self,
        id: Id,
        menu_type: MenuType,
    ) -> Task<Message> {
        if let Some((_, menu, _)) = self
            .active
            .iter_mut()
            .find(|(layer_id, menu, _)| *layer_id == id || menu.id == id)
        {
            menu.close_if(menu_type)
        } else {
            Task::none()
        }
    }

    pub fn request_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        if let Some((_, menu, _)) = self
            .active
            .iter()
            .find(|(layer_id, menu, _)| *layer_id == id || menu.id == id)
        {
            menu.request_keyboard()
        } else {
            Task::none()
        }
    }

    pub fn release_keyboard<Message: 'static>(&self, id: Id) -> Task<Message> {
        if let Some((_, menu, _)) = self
            .active
            .iter()
            .find(|(layer_id, menu, _)| *layer_id == id || menu.id == id)
        {
            menu.release_keyboard()
        } else {
            Task::none()
        }
    }
}
