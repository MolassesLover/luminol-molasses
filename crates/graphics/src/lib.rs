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

pub mod binding_helpers;
pub use binding_helpers::{BindGroupBuilder, BindGroupLayoutBuilder};

pub mod collision;
pub mod quad;
pub mod sprite;
pub mod tiles;
pub mod vertex;
pub mod viewport;

pub mod event;
pub mod map;
pub mod plane;

pub mod atlas_loader;

pub mod texture_loader;

pub use event::Event;
pub use map::Map;
pub use plane::Plane;

pub use texture_loader::Texture;

pub struct GraphicsState {
    pub texture_loader: texture_loader::Loader,
    pub atlas_loader: atlas_loader::Loader,
    pub render_state: luminol_egui_wgpu::RenderState,

    pub nearest_sampler: wgpu::Sampler,

    pipelines: Pipelines,
    bind_group_layouts: BindGroupLayouts,
}

pub struct BindGroupLayouts {
    sprite: wgpu::BindGroupLayout,
    tiles: wgpu::BindGroupLayout,
    collision: wgpu::BindGroupLayout,
}

pub struct Pipelines {
    sprites: std::collections::HashMap<luminol_data::BlendMode, wgpu::RenderPipeline>,
    tiles: wgpu::RenderPipeline,
    collision: wgpu::RenderPipeline,
}

impl GraphicsState {
    pub fn new(render_state: luminol_egui_wgpu::RenderState) -> Self {
        let bind_group_layouts = BindGroupLayouts {
            sprite: sprite::create_bind_group_layout(&render_state),
            tiles: tiles::create_bind_group_layout(&render_state),
            collision: collision::create_bind_group_layout(&render_state),
        };

        let pipelines = Pipelines {
            sprites: sprite::shader::create_sprite_shaders(&render_state, &bind_group_layouts),
            tiles: tiles::shader::create_render_pipeline(&render_state, &bind_group_layouts),
            collision: collision::shader::create_render_pipeline(
                &render_state,
                &bind_group_layouts,
            ),
        };

        let texture_loader = texture_loader::Loader::new(render_state.clone());
        let atlas_cache = atlas_loader::Loader::default();

        let nearest_sampler = render_state
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("luminol nearest texture sampler"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                ..Default::default()
            });

        Self {
            texture_loader,
            atlas_loader: atlas_cache,
            render_state,

            nearest_sampler,

            pipelines,
            bind_group_layouts,
        }
    }

    pub fn push_constants_supported(&self) -> bool {
        push_constants_supported(&self.render_state)
    }
}

pub fn push_constants_supported(render_state: &luminol_egui_wgpu::RenderState) -> bool {
    let feature_supported = render_state
        .device
        .features()
        .contains(wgpu::Features::PUSH_CONSTANTS);
    let is_dx_12 = render_state.adapter.get_info().backend == wgpu::Backend::Dx12;
    feature_supported && !is_dx_12
}
