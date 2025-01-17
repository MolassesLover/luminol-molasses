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

use itertools::Itertools;
use std::sync::Arc;
use std::time::Duration;

pub struct Tilepicker {
    pub selected_tiles_left: i16,
    pub selected_tiles_top: i16,
    pub selected_tiles_right: i16,
    pub selected_tiles_bottom: i16,

    drag_origin: Option<egui::Pos2>,

    resources: Arc<Resources>,
    viewport: Arc<luminol_graphics::viewport::Viewport>,
    ani_time: Option<f64>,
}

struct Resources {
    tiles: luminol_graphics::tiles::Tiles,
    collision: luminol_graphics::collision::Collision,
}

struct Callback {
    resources: Arc<Resources>,
    graphics_state: Arc<luminol_graphics::GraphicsState>,

    coll_enabled: bool,
}

//? SAFETY:
//? wgpu resources are not Send + Sync on wasm, but egui_wgpu::CallbackTrait requires Send + Sync (because egui::Context is Send + Sync)
//? as long as this callback does not leave the thread it was created on on wasm (which it shouldn't be) these are ok.
#[allow(unsafe_code)]
unsafe impl Send for Callback {}
#[allow(unsafe_code)]
unsafe impl Sync for Callback {}

impl luminol_egui_wgpu::CallbackTrait for Callback {
    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        _callback_resources: &'a luminol_egui_wgpu::CallbackResources,
    ) {
        self.resources
            .tiles
            .draw(&self.graphics_state, &[true], None, render_pass);

        if self.coll_enabled {
            self.resources
                .collision
                .draw(&self.graphics_state, render_pass);
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SelectedTile {
    Autotile(i16),
    Tile(i16),
}

impl SelectedTile {
    pub fn from_id(id: i16) -> Self {
        if id < 384 {
            SelectedTile::Autotile(id / 48)
        } else {
            SelectedTile::Tile(id)
        }
    }

    pub fn to_id(&self) -> i16 {
        match *self {
            Self::Autotile(tile) => tile * 48,
            Self::Tile(tile) => tile,
        }
    }
}

impl Default for SelectedTile {
    fn default() -> Self {
        SelectedTile::Autotile(0)
    }
}

impl Tilepicker {
    pub fn new(
        update_state: &luminol_core::UpdateState<'_>,
        map_id: usize, // FIXME
    ) -> anyhow::Result<Tilepicker> {
        let map = update_state
            .data
            .get_or_load_map(map_id, update_state.filesystem);
        let tilesets = update_state.data.tilesets();
        let tileset = &tilesets.data[map.tileset_id];

        let atlas = update_state.graphics.atlas_loader.load_atlas(
            &update_state.graphics,
            update_state.filesystem,
            tileset,
        )?;

        let tilepicker_data = (47..(384 + 47))
            .step_by(48)
            .chain(384..(atlas.tileset_height as i16 / 32 * 8 + 384))
            .collect_vec();
        let tilepicker_data = luminol_data::Table3::new_data(
            8,
            1 + (atlas.tileset_height / 32) as usize,
            1,
            tilepicker_data,
        );

        let viewport = Arc::new(luminol_graphics::viewport::Viewport::new(
            &update_state.graphics,
            256.,
            atlas.tileset_height as f32 + 32.,
        ));

        let tiles = luminol_graphics::tiles::Tiles::new(
            &update_state.graphics,
            viewport.clone(),
            atlas,
            &tilepicker_data,
        );

        let mut passages =
            luminol_data::Table2::new(tilepicker_data.xsize(), tilepicker_data.ysize());
        for x in 0..8 {
            passages[(x, 0)] = {
                let tile_id = tilepicker_data[(x, 0, 0)].try_into().unwrap_or_default();
                if tile_id >= tileset.passages.len() {
                    0
                } else {
                    tileset.passages[tile_id]
                }
            };
        }
        let length =
            (passages.len().saturating_sub(8)).min(tileset.passages.len().saturating_sub(384));
        passages.as_mut_slice()[8..8 + length]
            .copy_from_slice(&tileset.passages.as_slice()[384..384 + length]);
        let collision = luminol_graphics::collision::Collision::new(
            &update_state.graphics,
            viewport.clone(),
            &passages,
        );

        Ok(Self {
            resources: Arc::new(Resources { tiles, collision }),
            viewport,
            ani_time: None,
            selected_tiles_left: 0,
            selected_tiles_top: 0,
            selected_tiles_right: 0,
            selected_tiles_bottom: 0,
            drag_origin: None,
        })
    }

    pub fn get_tile_from_offset(&self, x: i16, y: i16) -> SelectedTile {
        let width = self.selected_tiles_right - self.selected_tiles_left + 1;
        let height = self.selected_tiles_bottom - self.selected_tiles_top + 1;
        let x = self.selected_tiles_left + x.rem_euclid(width);
        let y = self.selected_tiles_top + y.rem_euclid(height);
        match y {
            ..=0 => SelectedTile::Autotile(x),
            _ => SelectedTile::Tile(x + (y - 1) * 8 + 384),
        }
    }

    pub fn ui(
        &mut self,
        update_state: &luminol_core::UpdateState<'_>,
        ui: &mut egui::Ui,
        scroll_rect: egui::Rect,
        coll_enabled: bool,
    ) -> egui::Response {
        let time = ui.ctx().input(|i| i.time);
        let graphics_state = update_state.graphics.clone();

        if let Some(ani_time) = self.ani_time {
            if time - ani_time >= 16. / 60. {
                self.ani_time = Some(time);
                self.resources
                    .tiles
                    .autotiles
                    .inc_ani_index(&graphics_state.render_state);
            }
        } else {
            self.ani_time = Some(time);
        }

        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64(16. / 60.));

        let (canvas_rect, response) = ui.allocate_exact_size(
            egui::vec2(256., self.resources.tiles.atlas.tileset_height as f32 + 32.),
            egui::Sense::click_and_drag(),
        );

        let absolute_scroll_rect = ui
            .ctx()
            .screen_rect()
            .intersect(scroll_rect.translate(canvas_rect.min.to_vec2()));
        let scroll_rect = absolute_scroll_rect.translate(-canvas_rect.min.to_vec2());

        self.viewport.set_proj(
            &graphics_state.render_state,
            glam::Mat4::orthographic_rh(
                scroll_rect.left(),
                scroll_rect.right(),
                scroll_rect.bottom(),
                scroll_rect.top(),
                -1.,
                1.,
            ),
        );
        // FIXME: move this into graphics
        ui.painter()
            .add(luminol_egui_wgpu::Callback::new_paint_callback(
                absolute_scroll_rect,
                Callback {
                    resources: self.resources.clone(),
                    graphics_state: graphics_state.clone(),
                    coll_enabled,
                },
            ));

        let rect = egui::Rect::from_x_y_ranges(
            (self.selected_tiles_left * 32) as f32..=((self.selected_tiles_right + 1) * 32) as f32,
            (self.selected_tiles_top * 32) as f32..=((self.selected_tiles_bottom + 1) * 32) as f32,
        )
        .translate(canvas_rect.min.to_vec2());
        ui.painter()
            .rect_stroke(rect, 5.0, egui::Stroke::new(1.0, egui::Color32::WHITE));

        let Some(pos) = response.interact_pointer_pos() else {
            return response;
        };
        let pos = ((pos - canvas_rect.min) / 32.).to_pos2();

        if response.dragged_by(egui::PointerButton::Primary) {
            let drag_origin = if let Some(drag_origin) = self.drag_origin {
                drag_origin
            } else {
                self.drag_origin = Some(pos);
                pos
            };
            let rect = egui::Rect::from_two_pos(drag_origin, pos);
            let bottom = self.resources.tiles.atlas.tileset_height as i16 / 32;
            self.selected_tiles_left = (rect.left() as i16).clamp(0, 7);
            self.selected_tiles_right = (rect.right() as i16).clamp(0, 7);
            self.selected_tiles_top = (rect.top() as i16).clamp(0, bottom);
            self.selected_tiles_bottom = (rect.bottom() as i16).clamp(0, bottom);
        } else {
            self.drag_origin = None;
        }

        response
    }
}
