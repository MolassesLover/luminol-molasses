// Copyright (C) 2022 Lily Lyons
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

use egui::{CollapsingResponse, Color32, RichText};

use crate::data::{
    command_tree::Node,
    commands::{Command, CommandKind::*},
};

const CONTROL_FLOW: Color32 = Color32::BLUE;
const ERROR: Color32 = Color32::RED;
const NORMAL: Color32 = Color32::WHITE;
const COMMENT: Color32 = Color32::GREEN;

/// An event command viewer.

pub struct CommandView<'co> {
    commands: &'co mut Node,
}

impl<'co> CommandView<'co> {
    /// Create a new command viewer.
    pub fn new(commands: &'co mut Node) -> Self {
        Self { commands }
    }

    /// Show the viewer.
    pub fn ui(self, ui: &mut egui::Ui) {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_height(500.)
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                ui.visuals_mut().override_text_color = Some(NORMAL);
                ui.visuals_mut().button_frame = false;

                // ui.horizontal(|ui| {
                //     ui.vertical(|ui| {
                //         for (ele, cmd) in self.commands.iter().enumerate() {
                //             ui.selectable_label(false, format!("{:0>3}>", ele))
                //                 .clicked();
                //
                //             #[allow(clippy::single_match)]
                //             match cmd.kind {
                //                 Break => {
                //                     ui.selectable_label(false, "").clicked();
                //                 }
                //                 _ => (),
                //             }
                //         }
                //     });
                //
                //     ui.vertical(|ui| {
                //         let mut iter = self.commands.iter_mut().enumerate();
                //         let mut selected_index = ui
                //             .memory()
                //             .data
                //             .get_temp(egui::Id::new("command_view_selected_index"));
                //         let mut selected_index = *selected_index.get_or_insert(0);
                //
                //         while let Some((ele, cmd)) = iter.next() {
                //             Self::render_command(ui, ele, cmd, &mut iter, &mut selected_index);
                //
                //             if cmd.kind.is_break() {
                //                 continue;
                //             }
                //         }
                //
                //         if ui.input().key_pressed(egui::Key::ArrowUp) {
                //             selected_index =
                //                 (selected_index + self.commands.len() - 1) % self.commands.len()
                //         }
                //         if ui.input().key_pressed(egui::Key::ArrowDown) {
                //             selected_index =
                //                 (selected_index + self.commands.len() + 1) % self.commands.len()
                //         }
                //
                //         ui.memory().data.insert_temp(
                //             egui::Id::new("command_view_selected_index"),
                //             selected_index,
                //         );
                //     });
                // });
            });
    }

    fn render_command<'iter>(
        ui: &mut egui::Ui,
        ele: usize,
        cmd: &mut Command,
        iter: &mut impl Iterator<Item = (usize, &'iter mut Command)>,
        selected_index: &mut usize,
    ) {
        let Command { indent, kind } = cmd;

        let response = match kind {
            Break => {
                ui.vertical(|ui| {
                    ui.selectable_value(selected_index, ele, "@>");

                    ui.colored_label(CONTROL_FLOW, "Branch End");
                })
                .response
            }
            Text { text } => {
                ui.selectable_value(selected_index, ele, format!("Show Text: {}", text))
            }
            TextExt { text } => ui.label(format!("          :  {}", text)),
            Conditional { .. } => {
                //
                Self::collapsible(
                    "Conditional Branch".to_string(),
                    ui,
                    ele,
                    indent,
                    iter,
                    selected_index,
                )
                .header_response
            }
            Else => {
                Self::collapsible("Else".to_string(), ui, ele, indent, iter, selected_index)
                    .header_response
            }
            Loop => {
                //
                Self::collapsible("Loop".to_string(), ui, ele, indent, iter, selected_index)
                    .header_response
            }
            Comment { text } => {
                //
                ui.selectable_value(
                    selected_index,
                    ele,
                    RichText::new(format!("Comment: {}", text)).color(COMMENT),
                )
            }
            CommentExt { text } => {
                ui.selectable_value(
                    selected_index,
                    ele,
                    //                               "Comment: {}"
                    RichText::new(format!("       : {}", text)).color(COMMENT),
                )
            }
            Invalid { code } => {
                //
                ui.selectable_value(
                    selected_index,
                    ele,
                    RichText::new(format!("Invalid Command {} 🔥", code)).color(ERROR),
                )
                .on_hover_text(
                    RichText::new("This happens when Luminol does not recognize a command ID.")
                        .color(ERROR),
                )
            }
            _ => {
                //
                ui.selectable_value(selected_index, ele, format!("{:?} ???", kind))
                    .on_hover_text(
                        RichText::new(
                            "Luminol recognizes this command, but there is no code to render it.",
                        )
                        .color(ERROR),
                    )
            }
        };

        if *selected_index == ele
            && (ui.input().key_pressed(egui::Key::ArrowUp)
                || ui.input().key_pressed(egui::Key::ArrowDown))
        {
            response.scroll_to_me(None);
        }
    }

    fn collapsible<'iter>(
        text: String,
        ui: &mut egui::Ui,
        ele: usize,
        indent: &mut usize,
        iter: &mut impl Iterator<Item = (usize, &'iter mut Command)>,
        selected_index: &mut usize,
    ) -> CollapsingResponse<()> {
        ui.vertical(|ui| {
            let header = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                egui::Id::new(format!("{}_collapsible_command", ele)),
                true,
            );
            let openness = header.openness(ui.ctx());

            let ret_response = header
                .show_header(ui, |ui| {
                    ui.selectable_value(
                        selected_index,
                        ele,
                        RichText::new(text).color(CONTROL_FLOW),
                    )
                })
                .body(|ui| {
                    while let Some((ele, cmd)) = iter.next() {
                        let is_break = cmd.kind.is_break() || cmd.kind.is_else();

                        Self::render_command(ui, ele, cmd, iter, selected_index);

                        if is_break {
                            break;
                        }
                    }
                });

            let response = if let Some(ret_response_2) = ret_response.2 {
                egui::collapsing_header::CollapsingResponse {
                    header_response: ret_response.0,
                    body_response: Some(ret_response_2.response),
                    body_returned: Some(()),
                    openness,
                }
            } else {
                CollapsingResponse {
                    header_response: ret_response.0,
                    body_response: None,
                    body_returned: None,
                    openness,
                }
            };

            if response.fully_closed() {
                Self::break_until(iter, indent)
            }
            response
        })
        .inner
    }

    fn break_until<'iter>(
        iter: &mut impl Iterator<Item = (usize, &'iter mut Command)>,
        _indent: &mut usize,
    ) {
        while let Some((_, cmd)) = iter.next() {
            if cmd.kind.is_break() {
                break;
            }

            match cmd.kind {
                Loop | Else | Conditional { .. } => Self::break_until(iter, _indent),
                _ => (),
            }
        }
    }
}