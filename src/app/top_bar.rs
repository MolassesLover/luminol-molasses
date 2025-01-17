// Copyright (C) 2023 Lily Lyons
//
// This file is part of Luminol.
//
// Luminol is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Luminol is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Luminol.  If not, see <http://www.gnu.org/licenses/>.
//
//     Additional permission under GNU GPL version 3 section 7
//
// If you modify this Program, or any covered work, by linking or combining
// it with Steamworks API by Valve Corporation, containing parts covered by
// terms of the Steamworks API by Valve Corporation, the licensors of this
// Program grant you additional permission to convey the resulting work.

use strum::IntoEnumIterator;

/// The top bar for managing the project.
#[derive(Default)]
pub struct TopBar {
    #[cfg(not(target_arch = "wasm32"))]
    fullscreen: bool,
}

impl TopBar {
    /// Display the top bar.
    #[allow(unused_variables)]
    pub fn ui(&mut self, ui: &mut egui::Ui, update_state: &mut luminol_core::UpdateState<'_>) {
        egui::widgets::global_dark_light_mode_switch(ui);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let old_fullscreen = self.fullscreen;
            ui.checkbox(&mut self.fullscreen, "Fullscreen");
            if self.fullscreen != old_fullscreen {
                update_state
                    .ctx
                    .send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
        }

        let mut open_project = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O))
            && update_state.filesystem.project_loaded();
        let mut save_project = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
            && update_state.filesystem.project_loaded();
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            update_state
                .edit_windows
                .add_window(luminol_ui::windows::new_project::Window::default());
        }

        ui.separator();

        ui.menu_button("File", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            ui.label(if let Some(path) = update_state.filesystem.project_path() {
                format!("Current project:\n{}", path)
            } else {
                "No project open".to_string()
            });

            ui.add_enabled_ui(
                update_state
                    .project_manager
                    .load_filesystem_promise
                    .is_none(),
                |ui| {
                    if ui.button("New Project").clicked() {
                        update_state
                            .edit_windows
                            .add_window(luminol_ui::windows::new_project::Window::default());
                    }

                    open_project |= ui.button("Open Project").clicked();
                },
            );

            ui.separator();

            ui.add_enabled_ui(update_state.filesystem.project_loaded(), |ui| {
                if ui.button("Project Config").clicked() {
                    update_state
                        .edit_windows
                        .add_window(luminol_ui::windows::config_window::Window {});
                }

                if ui.button("Close Project").clicked() {
                    update_state.project_manager.close_project();
                }

                save_project |= ui.button("Save Project").clicked();
            });

            ui.separator();

            ui.add_enabled_ui(update_state.filesystem.project_loaded(), |ui| {
                if ui.button("Command Maker").clicked() {
                    // update_state.windows.add_window(
                    //     luminol_ui::windows::command_gen::CommandGeneratorWindow::default(),
                    // );
                }
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.separator();

                if ui.button("Quit").clicked() {
                    update_state
                        .ctx
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });

        ui.separator();

        ui.menu_button("Edit", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            if ui.button("Preferences").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::global_config_window::Window::default())
            }

            if ui.button("Appearance").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::appearance::Window::default())
            }
        });

        ui.separator();

        ui.menu_button("Data", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            ui.add_enabled_ui(update_state.filesystem.project_loaded(), |ui| {
                if ui.button("Maps").clicked() {
                    update_state
                        .edit_windows
                        .add_window(luminol_ui::windows::map_picker::Window::default());
                }

                if ui.button("Items").clicked() {
                    update_state
                        .edit_windows
                        .add_window(luminol_ui::windows::items::Window::new(update_state.data));
                }

                if ui.button("Common Events").clicked() {
                    update_state
                        .edit_windows
                        .add_window(luminol_ui::windows::common_event_edit::Window::default());
                }

                if ui.button("Scripts").clicked() {
                    update_state
                        .edit_windows
                        .add_window(luminol_ui::windows::script_edit::Window::default());
                }

                if ui.button("Sound Test").clicked() {
                    update_state.edit_windows.add_window(
                        luminol_ui::windows::sound_test::Window::new(update_state.filesystem),
                    );
                }
            });
        });

        ui.separator();

        ui.menu_button("Tools", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            if ui.button("RGSSAD Archive Manager").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::archive_manager::Window::default());
            }
        });

        ui.separator();

        ui.menu_button("Help", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            ui.button("Contents").clicked();

            if ui.button("About...").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::about::Window::default());
            };
        });

        ui.menu_button("Debug", |ui| {
            // Hide this menu if the unsaved changes modal or a file/folder picker is open
            if update_state.project_manager.is_modal_open()
                || update_state.project_manager.is_picker_open()
            {
                ui.close_menu();
            }

            if ui.button("Egui Inspection").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::misc::EguiInspection::default());
            }

            if ui.button("Egui Memory").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::misc::EguiMemory::default());
            }

            #[cfg(debug_assertions)]
            {
                let mut debug_on_hover = ui.ctx().debug_on_hover();
                ui.toggle_value(&mut debug_on_hover, "Debug on hover");
                ui.ctx().set_debug_on_hover(debug_on_hover);
            }

            ui.separator();

            if ui.button("Filesystem Debug").clicked() {
                update_state
                    .edit_windows
                    .add_window(luminol_ui::windows::misc::FilesystemDebug::default());
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.separator();

            ui.add_enabled_ui(update_state.filesystem.project_loaded(), |ui| {
                if ui.button("Playtest").clicked() {
                    let mut cmd = luminol_term::CommandBuilder::new("steamshim");
                    cmd.cwd(
                        update_state
                            .filesystem
                            .project_path()
                            .expect("project not loaded"),
                    );

                    let result = luminol_ui::windows::console::Window::new(cmd).or_else(|_| {
                        let mut cmd = luminol_term::CommandBuilder::new("game");
                        cmd.cwd(
                            update_state
                                .filesystem
                                .project_path()
                                .expect("project not loaded"),
                        );

                        luminol_ui::windows::console::Window::new(cmd)
                    });

                    match result {
                        Ok(w) => update_state.edit_windows.add_window(w),
                        Err(e) => update_state.toasts.error(format!(
                            "error starting game (tried steamshim.exe and then game.exe): {e}"
                        )),
                    }
                }

                if ui.button("Terminal").clicked() {
                    #[cfg(windows)]
                    let shell = "powershell";
                    #[cfg(unix)]
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
                    let mut cmd = luminol_term::CommandBuilder::new(shell);
                    cmd.cwd(
                        update_state
                            .filesystem
                            .project_path()
                            .expect("project not loaded"),
                    );

                    match luminol_ui::windows::console::Window::new(cmd) {
                        Ok(w) => update_state.edit_windows.add_window(w),
                        Err(e) => update_state
                            .toasts
                            .error(format!("error starting shell: {e}")),
                    }
                }
            });
        }

        ui.separator();

        ui.label("Brush:");

        for brush in luminol_core::Pencil::iter() {
            ui.selectable_value(&mut update_state.toolbar.pencil, brush, brush.to_string());
        }

        if open_project {
            update_state.project_manager.open_project_picker();
        }

        if save_project {
            if let Some(config) = update_state.project_config {
                match update_state.data.save(update_state.filesystem, config) {
                    Ok(_) => {
                        update_state.modified.set(false);
                        update_state.toasts.info("Saved project successfully!")
                    }
                    Err(e) => update_state.toasts.error(e.to_string()),
                }
            }
        }

        if update_state
            .project_manager
            .load_filesystem_promise
            .is_some()
        {
            ui.spinner();
        }
    }
}
