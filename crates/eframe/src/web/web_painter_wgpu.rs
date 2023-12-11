use std::sync::Arc;

use wasm_bindgen::JsValue;

use luminol_egui_wgpu::{renderer::ScreenDescriptor, RenderState, SurfaceErrorAction};

use crate::WebOptions;

use super::web_painter::WebPainter;

struct EguiWebWindow(u32);

#[allow(unsafe_code)]
unsafe impl raw_window_handle::HasRawWindowHandle for EguiWebWindow {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut window_handle = raw_window_handle::WebWindowHandle::empty();
        window_handle.id = self.0;
        raw_window_handle::RawWindowHandle::Web(window_handle)
    }
}

#[allow(unsafe_code)]
unsafe impl raw_window_handle::HasRawDisplayHandle for EguiWebWindow {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        raw_window_handle::RawDisplayHandle::Web(raw_window_handle::WebDisplayHandle::empty())
    }
}

pub(crate) struct WebPainterWgpu {
    canvas: web_sys::OffscreenCanvas,
    surface: wgpu::Surface,
    pub(super) surface_configuration: wgpu::SurfaceConfiguration,
    render_state: Option<RenderState>,
    on_surface_error: Arc<dyn Fn(wgpu::SurfaceError) -> SurfaceErrorAction>,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,

    /// Width of the canvas in points. `surface_configuration.width` is the width in pixels.
    pub(super) width: u32,
    /// Height of the canvas in points. `surface_configuration.height` is the height in pixels.
    pub(super) height: u32,
    /// Length of a pixel divided by length of a point.
    pub(super) pixel_ratio: f32,
    pub(super) needs_resize: bool,
}

impl WebPainterWgpu {
    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    pub fn generate_depth_texture_view(
        &self,
        render_state: &RenderState,
        width_in_pixels: u32,
        height_in_pixels: u32,
    ) -> Option<wgpu::TextureView> {
        let device = &render_state.device;
        self.depth_format.map(|depth_format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("egui_depth_texture"),
                    size: wgpu::Extent3d {
                        width: width_in_pixels,
                        height: height_in_pixels,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: depth_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[depth_format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        })
    }

    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub async fn new(
        canvas: web_sys::OffscreenCanvas,
        options: &WebOptions,
    ) -> Result<Self, String> {
        log::debug!("Creating wgpu painter");

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: options.wgpu_options.supported_backends,
            ..Default::default()
        });

        let surface = instance
            .create_surface_from_offscreen_canvas(canvas.clone())
            .map_err(|err| format!("failed to create wgpu surface: {err}"))?;

        let depth_format = luminol_egui_wgpu::depth_format_from_bits(options.depth_buffer, 0);
        let render_state =
            RenderState::create(&options.wgpu_options, &instance, &surface, depth_format, 1)
                .await
                .map_err(|err| err.to_string())?;

        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: render_state.target_format,
            width: 0,
            height: 0,
            present_mode: options.wgpu_options.present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![render_state.target_format],
        };

        log::debug!("wgpu painter initialized.");

        Ok(Self {
            canvas,
            render_state: Some(render_state),
            surface,
            surface_configuration,
            depth_format,
            depth_texture_view: None,
            on_surface_error: options.wgpu_options.on_surface_error.clone(),

            width: 0,
            height: 0,
            pixel_ratio: 1.,
            needs_resize: false,
        })
    }
}

impl WebPainter for WebPainterWgpu {
    fn max_texture_side(&self) -> usize {
        self.render_state.as_ref().map_or(0, |state| {
            state.device.limits().max_texture_dimension_2d as _
        })
    }

    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        let size_in_pixels = [
            self.surface_configuration.width,
            self.surface_configuration.height,
        ];

        let Some(render_state) = &self.render_state else {
            return Err(JsValue::from_str(
                "Can't paint, wgpu renderer was already disposed",
            ));
        };

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_webpainter_paint_and_update_textures"),
                });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels,
            pixels_per_point,
        };

        let user_cmd_bufs = {
            let mut renderer = render_state.renderer.write();
            for (id, image_delta) in &textures_delta.set {
                renderer.update_texture(
                    &render_state.device,
                    &render_state.queue,
                    *id,
                    image_delta,
                );
            }

            renderer.update_buffers(
                &render_state.device,
                &render_state.queue,
                &mut encoder,
                clipped_primitives,
                &screen_descriptor,
            )
        };

        // Resize surface if needed
        let is_zero_sized_surface = size_in_pixels[0] == 0 || size_in_pixels[1] == 0;
        let frame = if is_zero_sized_surface {
            None
        } else {
            if self.needs_resize {
                self.needs_resize = false;
                self.surface
                    .configure(&render_state.device, &self.surface_configuration);
                self.depth_texture_view = self.generate_depth_texture_view(
                    render_state,
                    size_in_pixels[0],
                    size_in_pixels[1],
                );
            }

            let frame = match self.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(err) => match (*self.on_surface_error)(err) {
                    SurfaceErrorAction::RecreateSurface => {
                        self.surface
                            .configure(&render_state.device, &self.surface_configuration);
                        return Ok(());
                    }
                    SurfaceErrorAction::SkipFrame => {
                        return Ok(());
                    }
                },
            };

            {
                let renderer = render_state.renderer.read();
                let frame_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_color[0] as f64,
                                g: clear_color[1] as f64,
                                b: clear_color[2] as f64,
                                a: clear_color[3] as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: self.depth_texture_view.as_ref().map(|view| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                // It is very unlikely that the depth buffer is needed after egui finished rendering
                                // so no need to store it. (this can improve performance on tiling GPUs like mobile chips or Apple Silicon)
                                store: wgpu::StoreOp::Discard,
                            }),
                            stencil_ops: None,
                        }
                    }),
                    label: Some("egui_render"),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                renderer.render(&mut render_pass, clipped_primitives, &screen_descriptor);
            }

            Some(frame)
        };

        {
            let mut renderer = render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
            }
        }

        // Submit the commands: both the main buffer and user-defined ones.
        render_state
            .queue
            .submit(user_cmd_bufs.into_iter().chain([encoder.finish()]));

        if let Some(frame) = frame {
            frame.present();
        }

        Ok(())
    }

    fn destroy(&mut self) {
        self.render_state = None;
    }
}
