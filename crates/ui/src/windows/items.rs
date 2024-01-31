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

use luminol_components::UiExt;

/// Database - Items management window.
#[derive(Default)]
pub struct Window {
    // ? Items ?
    selected_item: usize,
    selected_item_name: Option<String>,

    // ? Icon Graphic Picker ?
    _icon_picker: Option<luminol_modals::graphic_picker::Modal>,

    // ? Menu Sound Effect Picker ?
    _menu_se_picker: Option<luminol_modals::sound_picker::Modal>,

    previous_selected_item: Option<usize>,
}

impl Window {
    pub fn new() -> Self {
        Default::default()
    }
}

impl luminol_core::Window for Window {
    fn name(&self) -> String {
        if let Some(name) = &self.selected_item_name {
            format!("Editing item {:?}", name)
        } else {
            "Item Editor".into()
        }
    }

    fn id(&self) -> egui::Id {
        egui::Id::new("item_editor")
    }

    fn requires_filesystem(&self) -> bool {
        true
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        update_state: &mut luminol_core::UpdateState<'_>,
    ) {
        let change_maximum_text = "Change maximum...";

        let p = update_state
            .project_config
            .as_ref()
            .expect("project not loaded")
            .project
            .persistence_id;
        let mut items = update_state.data.items();
        let animations = update_state.data.animations();
        let common_events = update_state.data.common_events();
        let system = update_state.data.system();
        let states = update_state.data.states();

        self.selected_item = self.selected_item.min(items.data.len().saturating_sub(1));
        self.selected_item_name = items
            .data
            .get(self.selected_item)
            .map(|item| item.name.clone());
        let mut modified = false;

        egui::Window::new(self.name())
            .id(egui::Id::new("item_editor"))
            .default_width(500.)
            .open(open)
            .show(ctx, |ui| {
                let button_height = ui.spacing().interact_size.y.max(
                    ui.text_style_height(&egui::TextStyle::Button)
                        + 2. * ui.spacing().button_padding.y,
                );
                let button_width = ui.spacing().interact_size.x.max(
                    ui.text_width(change_maximum_text, egui::TextStyle::Button)
                        + 2. * ui.spacing().button_padding.x,
                );

                egui::SidePanel::left(egui::Id::new("item_edit_sidepanel")).show_inside(ui, |ui| {
                    ui.with_right_margin(ui.spacing().window_margin.right, |ui| {
                        ui.with_cross_justify(|ui| {
                            ui.label("Items");
                            egui::ScrollArea::both()
                                .id_source(p)
                                .min_scrolled_width(button_width + ui.spacing().item_spacing.x)
                                .max_height(
                                    ui.available_height()
                                        - button_height
                                        - ui.spacing().item_spacing.y,
                                )
                                .show_rows(ui, button_height, items.data.len(), |ui, rows| {
                                    ui.set_width(ui.available_width());

                                    let offset = rows.start;
                                    for (id, item) in items.data[rows].iter_mut().enumerate() {
                                        let id = id + offset;

                                        ui.with_stripe(id % 2 != 0, |ui| {
                                            ui.style_mut().wrap = Some(false);

                                            let response = ui
                                                .selectable_value(
                                                    &mut self.selected_item,
                                                    id,
                                                    format!("{:0>3}: {}", id, item.name),
                                                )
                                                .interact(egui::Sense::click());

                                            if response.clicked() {
                                                response.request_focus();
                                            }

                                            // Reset this item if delete or backspace
                                            // is pressed while this item is focused
                                            if response.has_focus()
                                                && ui.input(|i| {
                                                    i.key_down(egui::Key::Delete)
                                                        || i.key_down(egui::Key::Backspace)
                                                })
                                            {
                                                *item = Default::default();
                                                modified = true;
                                            }
                                        });
                                    }
                                });

                            if ui
                                .add(egui::Button::new(change_maximum_text).wrap(false))
                                .clicked()
                            {
                                luminol_core::basic!(
                                    update_state.toasts,
                                    "`Change maximum...` button trigger"
                                );
                            }
                        });
                    });
                });

                ui.with_left_margin(ui.spacing().window_margin.left, |ui| {
                    ui.with_cross_justify(|ui| {
                        egui::ScrollArea::vertical().id_source(p).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.set_min_width(
                                2. * (ui.spacing().slider_width + ui.spacing().interact_size.x)
                                    + 3. * ui.spacing().item_spacing.x,
                            );

                            let Some(selected_item) = items.data.get_mut(self.selected_item) else {
                                return;
                            };

                            modified |= ui
                                .add(luminol_components::Field::new(
                                    "Name",
                                    egui::TextEdit::singleline(&mut selected_item.name)
                                        .desired_width(f32::INFINITY),
                                ))
                                .changed();

                            modified |= ui
                                .add(luminol_components::Field::new(
                                    "Description",
                                    egui::TextEdit::multiline(&mut selected_item.description)
                                        .desired_width(f32::INFINITY),
                                ))
                                .changed();

                            ui.with_stripe(true, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Scope",
                                            luminol_components::EnumComboBox::new(
                                                (selected_item.id, "scope"),
                                                &mut selected_item.scope,
                                            ),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Occasion",
                                            luminol_components::EnumComboBox::new(
                                                (selected_item.id, "occasion"),
                                                &mut selected_item.occasion,
                                            ),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(false, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "User Animation",
                                            luminol_components::OptionalIdComboBox::new(
                                                (selected_item.id, "animation1_id"),
                                                &mut selected_item.animation1_id,
                                                animations.data.len(),
                                                |id| {
                                                    animations.data.get(id).map_or_else(
                                                        || "".into(),
                                                        |a| format!("{id:0>3}: {}", a.name),
                                                    )
                                                },
                                            ),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Target Animation",
                                            luminol_components::OptionalIdComboBox::new(
                                                (selected_item.id, "animation2_id"),
                                                &mut selected_item.animation2_id,
                                                animations.data.len(),
                                                |id| {
                                                    animations.data.get(id).map_or_else(
                                                        || "".into(),
                                                        |a| format!("{id:0>3}: {}", a.name),
                                                    )
                                                },
                                            ),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(true, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Menu Use SE",
                                            egui::Label::new("TODO"),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Common Event",
                                            luminol_components::OptionalIdComboBox::new(
                                                (selected_item.id, "common_event_id"),
                                                &mut selected_item.common_event_id,
                                                common_events.data.len(),
                                                |id| {
                                                    common_events.data.get(id).map_or_else(
                                                        || "".into(),
                                                        |e| format!("{id:0>3}: {}", e.name),
                                                    )
                                                },
                                            ),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(false, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Price",
                                            egui::DragValue::new(&mut selected_item.price)
                                                .clamp_range(0..=i32::MAX),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Consumable",
                                            egui::Checkbox::without_text(
                                                &mut selected_item.consumable,
                                            ),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(true, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Parameter",
                                            luminol_components::EnumComboBox::new(
                                                "parameter_type",
                                                &mut selected_item.parameter_type,
                                            ),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add_enabled(
                                            !matches!(
                                                selected_item.parameter_type,
                                                luminol_data::rpg::item::ParameterType::None
                                            ),
                                            luminol_components::Field::new(
                                                "Parameter Increment",
                                                egui::DragValue::new(
                                                    &mut selected_item.parameter_points,
                                                )
                                                .clamp_range(0..=i32::MAX),
                                            ),
                                        )
                                        .changed();
                                });
                            });

                            ui.with_stripe(false, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Recover HP Rate",
                                            egui::Slider::new(
                                                &mut selected_item.recover_hp_rate,
                                                0..=100,
                                            )
                                            .suffix("%"),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Recover HP",
                                            egui::DragValue::new(&mut selected_item.recover_hp)
                                                .clamp_range(0..=i32::MAX),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(true, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Recover SP Rate",
                                            egui::Slider::new(
                                                &mut selected_item.recover_sp_rate,
                                                0..=100,
                                            )
                                            .suffix("%"),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Recover SP",
                                            egui::DragValue::new(&mut selected_item.recover_sp)
                                                .clamp_range(0..=i32::MAX),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(false, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "Hit Rate",
                                            egui::Slider::new(&mut selected_item.hit, 0..=100)
                                                .suffix("%"),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "Variance",
                                            egui::Slider::new(&mut selected_item.variance, 0..=100)
                                                .suffix("%"),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(true, |ui| {
                                ui.columns(2, |columns| {
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new(
                                            "PDEF-F",
                                            egui::Slider::new(&mut selected_item.pdef_f, 0..=100)
                                                .suffix("%"),
                                        ))
                                        .changed();

                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "MDEF-F",
                                            egui::Slider::new(&mut selected_item.mdef_f, 0..=100)
                                                .suffix("%"),
                                        ))
                                        .changed();
                                });
                            });

                            ui.with_stripe(false, |ui| {
                                ui.columns(2, |columns| {
                                    let mut selection = luminol_components::IdVecSelection::new(
                                        (selected_item.id, "element_set"),
                                        &mut selected_item.element_set,
                                        system.elements.len(),
                                        |id| {
                                            system.elements.get(id).map_or_else(
                                                || "".into(),
                                                |e| format!("{id:0>3}: {}", e),
                                            )
                                        },
                                    );
                                    if self.previous_selected_item != Some(selected_item.id) {
                                        selection.clear_search();
                                    }
                                    modified |= columns[0]
                                        .add(luminol_components::Field::new("Elements", selection))
                                        .changed();

                                    let mut selection =
                                        luminol_components::IdVecPlusMinusSelection::new(
                                            (selected_item.id, "state_set"),
                                            &mut selected_item.plus_state_set,
                                            &mut selected_item.minus_state_set,
                                            states.data.len(),
                                            |id| {
                                                states.data.get(id).map_or_else(
                                                    || "".into(),
                                                    |s| format!("{id:0>3}: {}", s.name),
                                                )
                                            },
                                        );
                                    if self.previous_selected_item != Some(selected_item.id) {
                                        selection.clear_search();
                                    }
                                    modified |= columns[1]
                                        .add(luminol_components::Field::new(
                                            "State Change",
                                            selection,
                                        ))
                                        .changed();
                                });
                            });
                        });
                    });
                });
            });

        self.previous_selected_item =
            (self.selected_item < items.data.len()).then_some(self.selected_item);

        if modified {
            update_state.modified.set(true);
            items.modified = true;
        }
    }
}
