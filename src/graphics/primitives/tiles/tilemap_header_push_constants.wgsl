struct PushConstants {
    viewport: Viewport,
    autotiles: Autotiles,
    opacity: f32,
}

var<push_constant> push_constants: PushConstants;