pub mod visibility;
pub mod window;

use bevy_camera::{
    primitives::Frustum, CameraMainTextureUsages, ClearColor, ClearColorConfig, Exposure,
    MainPassResolutionOverride, NormalizedRenderTarget,
};
use bevy_diagnostic::FrameCount;
pub use visibility::*;
pub use window::*;

use crate::{
    camera::{ExtractedCamera, MipBias, NormalizedRenderTargetExt as _, TemporalJitter},
    experimental::occlusion_culling::OcclusionCulling,
    extract_component::ExtractComponentPlugin,
    render_asset::RenderAssets,
    render_phase::ViewRangefinder3d,
    render_resource::{DynamicUniformBuffer, ShaderType, Texture, TextureView},
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    texture::{
        CachedTexture, ColorAttachment, DepthAttachment, GpuImage, ManualTextureViews,
        OutputColorAttachment, TextureCache,
    },
    Render, RenderApp, RenderSystems,
};
use alloc::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_color::LinearRgba;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_image::{BevyDefault as _, ToExtents};
use bevy_math::{mat3, vec2, vec3, Mat3, Mat4, UVec4, Vec2, Vec3, Vec4, Vec4Swizzles};
use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render_macros::ExtractComponent;
use bevy_shader::load_shader_library;
use bevy_transform::components::GlobalTransform;
use core::{
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};
use wgpu::{
    BufferUsages, RenderPassColorAttachment, RenderPassDepthStencilAttachment, StoreOp,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// The matrix that converts from the RGB to the LMS color space.
///
/// To derive this, first we convert from RGB to [CIE 1931 XYZ]:
///
/// ```text
/// ⎡ X ⎤   ⎡ 0.490  0.310  0.200 ⎤ ⎡ R ⎤
/// ⎢ Y ⎥ = ⎢ 0.177  0.812  0.011 ⎥ ⎢ G ⎥
/// ⎣ Z ⎦   ⎣ 0.000  0.010  0.990 ⎦ ⎣ B ⎦
/// ```
///
/// Then we convert to LMS according to the [CAM16 standard matrix]:
///
/// ```text
/// ⎡ L ⎤   ⎡  0.401   0.650  -0.051 ⎤ ⎡ X ⎤
/// ⎢ M ⎥ = ⎢ -0.250   1.204   0.046 ⎥ ⎢ Y ⎥
/// ⎣ S ⎦   ⎣ -0.002   0.049   0.953 ⎦ ⎣ Z ⎦
/// ```
///
/// The resulting matrix is just the concatenation of these two matrices, to do
/// the conversion in one step.
///
/// [CIE 1931 XYZ]: https://en.wikipedia.org/wiki/CIE_1931_color_space
/// [CAM16 standard matrix]: https://en.wikipedia.org/wiki/LMS_color_space
static RGB_TO_LMS: Mat3 = mat3(
    vec3(0.311692, 0.0905138, 0.00764433),
    vec3(0.652085, 0.901341, 0.0486554),
    vec3(0.0362225, 0.00814478, 0.943700),
);

/// The inverse of the [`RGB_TO_LMS`] matrix, converting from the LMS color
/// space back to RGB.
static LMS_TO_RGB: Mat3 = mat3(
    vec3(4.06305, -0.40791, -0.0118812),
    vec3(-2.93241, 1.40437, -0.0486532),
    vec3(-0.130646, 0.00353630, 1.0605344),
);

/// The [CIE 1931] *xy* chromaticity coordinates of the [D65 white point].
///
/// [CIE 1931]: https://en.wikipedia.org/wiki/CIE_1931_color_space
/// [D65 white point]: https://en.wikipedia.org/wiki/Standard_illuminant#D65_values
static D65_XY: Vec2 = vec2(0.31272, 0.32903);

/// The [D65 white point] in [LMS color space].
///
/// [LMS color space]: https://en.wikipedia.org/wiki/LMS_color_space
/// [D65 white point]: https://en.wikipedia.org/wiki/Standard_illuminant#D65_values
static D65_LMS: Vec3 = vec3(0.975538, 1.01648, 1.08475);

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "view.wgsl");

        app
            // NOTE: windows.is_changed() handles cases where a window was resized
            .add_plugins((
                ExtractComponentPlugin::<Hdr>::default(),
                ExtractComponentPlugin::<Msaa>::default(),
                ExtractComponentPlugin::<OcclusionCulling>::default(),
                RenderVisibilityRangePlugin,
            ));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                (
                    // `TextureView`s need to be dropped before reconfiguring window surfaces.
                    clear_view_attachments
                        .in_set(RenderSystems::ManageViews)
                        .before(create_surfaces),
                    prepare_view_attachments
                        .in_set(RenderSystems::ManageViews)
                        .before(prepare_view_targets)
                        .after(prepare_windows),
                    prepare_view_targets
                        .in_set(RenderSystems::ManageViews)
                        .after(prepare_windows)
                        .after(crate::render_asset::prepare_assets::<GpuImage>)
                        .ambiguous_with(crate::camera::sort_cameras), // doesn't use `sorted_camera_index_for_target`
                    prepare_view_uniforms.in_set(RenderSystems::PrepareResources),
                ),
            );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ViewUniforms>()
                .init_resource::<ViewTargetAttachments>();
        }
    }
}

/// Component for configuring the number of samples for [Multi-Sample Anti-Aliasing](https://en.wikipedia.org/wiki/Multisample_anti-aliasing)
/// for a [`Camera`](bevy_camera::Camera).
///
/// Defaults to 4 samples. A higher number of samples results in smoother edges.
///
/// Some advanced rendering features may require that MSAA is disabled.
///
/// Note that the web currently only supports 1 or 4 samples.
#[derive(
    Component,
    Default,
    Clone,
    Copy,
    ExtractComponent,
    Reflect,
    PartialEq,
    PartialOrd,
    Eq,
    Hash,
    Debug,
)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
pub enum Msaa {
    Off = 1,
    Sample2 = 2,
    #[default]
    Sample4 = 4,
    Sample8 = 8,
}

impl Msaa {
    #[inline]
    pub fn samples(&self) -> u32 {
        *self as u32
    }

    pub fn from_samples(samples: u32) -> Self {
        match samples {
            1 => Msaa::Off,
            2 => Msaa::Sample2,
            4 => Msaa::Sample4,
            8 => Msaa::Sample8,
            _ => panic!("Unsupported MSAA sample count: {samples}"),
        }
    }
}

/// If this component is added to a camera, the camera will use an intermediate "high dynamic range" render texture.
/// This allows rendering with a wider range of lighting values. However, this does *not* affect
/// whether the camera will render with hdr display output (which bevy does not support currently)
/// and only affects the intermediate render texture.
#[derive(
    Component, Default, Copy, Clone, ExtractComponent, Reflect, PartialEq, Eq, Hash, Debug,
)]
#[reflect(Component, Default, PartialEq, Hash, Debug)]
pub struct Hdr;

/// An identifier for a view that is stable across frames.
///
/// We can't use [`Entity`] for this because render world entities aren't
/// stable, and we can't use just [`MainEntity`] because some main world views
/// extract to multiple render world views. For example, a directional light
/// extracts to one render world view per cascade, and a point light extracts to
/// one render world view per cubemap face. So we pair the main entity with an
/// *auxiliary entity* and a *subview index*, which *together* uniquely identify
/// a view in the render world in a way that's stable from frame to frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RetainedViewEntity {
    /// The main entity that this view corresponds to.
    pub main_entity: MainEntity,

    /// Another entity associated with the view entity.
    ///
    /// This is currently used for shadow cascades. If there are multiple
    /// cameras, each camera needs to have its own set of shadow cascades. Thus
    /// the light and subview index aren't themselves enough to uniquely
    /// identify a shadow cascade: we need the camera that the cascade is
    /// associated with as well. This entity stores that camera.
    ///
    /// If not present, this will be `MainEntity(Entity::PLACEHOLDER)`.
    pub auxiliary_entity: MainEntity,

    /// The index of the view corresponding to the entity.
    ///
    /// For example, for point lights that cast shadows, this is the index of
    /// the cubemap face (0 through 5 inclusive). For directional lights, this
    /// is the index of the cascade.
    pub subview_index: u32,
}

impl RetainedViewEntity {
    /// Creates a new [`RetainedViewEntity`] from the given main world entity,
    /// auxiliary main world entity, and subview index.
    ///
    /// See [`RetainedViewEntity::subview_index`] for an explanation of what
    /// `auxiliary_entity` and `subview_index` are.
    pub fn new(
        main_entity: MainEntity,
        auxiliary_entity: Option<MainEntity>,
        subview_index: u32,
    ) -> Self {
        Self {
            main_entity,
            auxiliary_entity: auxiliary_entity.unwrap_or(Entity::PLACEHOLDER.into()),
            subview_index,
        }
    }
}

/// Describes a camera in the render world.
///
/// Each entity in the main world can potentially extract to multiple subviews,
/// each of which has a [`RetainedViewEntity::subview_index`]. For instance, 3D
/// cameras extract to both a 3D camera subview with index 0 and a special UI
/// subview with index 1. Likewise, point lights with shadows extract to 6
/// subviews, one for each side of the shadow cubemap.
#[derive(Component)]
pub struct ExtractedView {
    /// The entity in the main world corresponding to this render world view.
    pub retained_view_entity: RetainedViewEntity,
    /// Typically a column-major right-handed projection matrix, one of either:
    ///
    /// Perspective (infinite reverse z)
    /// ```text
    /// f = 1 / tan(fov_y_radians / 2)
    ///
    /// ⎡ f / aspect  0   0     0 ⎤
    /// ⎢          0  f   0     0 ⎥
    /// ⎢          0  0   0  near ⎥
    /// ⎣          0  0  -1     0 ⎦
    /// ```
    ///
    /// Orthographic
    /// ```text
    /// w = right - left
    /// h = top - bottom
    /// d = far - near
    /// cw = -right - left
    /// ch = -top - bottom
    ///
    /// ⎡ 2 / w      0      0   cw / w ⎤
    /// ⎢     0  2 / h      0   ch / h ⎥
    /// ⎢     0      0  1 / d  far / d ⎥
    /// ⎣     0      0      0        1 ⎦
    /// ```
    ///
    /// `clip_from_view[3][3] == 1.0` is the standard way to check if a projection is orthographic
    ///
    /// Glam matrices are column major, so for example getting the near plane of a perspective projection is `clip_from_view[3][2]`
    ///
    /// Custom projections are also possible however.
    pub clip_from_view: Mat4,
    pub world_from_view: GlobalTransform,
    // The view-projection matrix. When provided it is used instead of deriving it from
    // `projection` and `transform` fields, which can be helpful in cases where numerical
    // stability matters and there is a more direct way to derive the view-projection matrix.
    pub clip_from_world: Option<Mat4>,
    pub hdr: bool,
    // uvec4(origin.x, origin.y, width, height)
    pub viewport: UVec4,
    pub color_grading: ColorGrading,
}

impl ExtractedView {
    /// Creates a 3D rangefinder for a view
    pub fn rangefinder3d(&self) -> ViewRangefinder3d {
        ViewRangefinder3d::from_world_from_view(&self.world_from_view.to_matrix())
    }
}

/// Configures filmic color grading parameters to adjust the image appearance.
///
/// Color grading is applied just before tonemapping for a given
/// [`Camera`](bevy_camera::Camera) entity, with the sole exception of the
/// `post_saturation` value in [`ColorGradingGlobal`], which is applied after
/// tonemapping.
#[derive(Component, Reflect, Debug, Default, Clone)]
#[reflect(Component, Default, Debug, Clone)]
pub struct ColorGrading {
    /// Filmic color grading values applied to the image as a whole (as opposed
    /// to individual sections, like shadows and highlights).
    pub global: ColorGradingGlobal,

    /// Color grading values that are applied to the darker parts of the image.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub shadows: ColorGradingSection,

    /// Color grading values that are applied to the parts of the image with
    /// intermediate brightness.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub midtones: ColorGradingSection,

    /// Color grading values that are applied to the lighter parts of the image.
    ///
    /// The cutoff points can be customized with the
    /// [`ColorGradingGlobal::midtones_range`] field.
    pub highlights: ColorGradingSection,
}

/// Filmic color grading values applied to the image as a whole (as opposed to
/// individual sections, like shadows and highlights).
#[derive(Clone, Debug, Reflect)]
#[reflect(Default, Clone)]
pub struct ColorGradingGlobal {
    /// Exposure value (EV) offset, measured in stops.
    pub exposure: f32,

    /// An adjustment made to the [CIE 1931] chromaticity *x* value.
    ///
    /// Positive values make the colors redder. Negative values make the colors
    /// bluer. This has no effect on luminance (brightness).
    ///
    /// [CIE 1931]: https://en.wikipedia.org/wiki/CIE_1931_color_space#CIE_xy_chromaticity_diagram_and_the_CIE_xyY_color_space
    pub temperature: f32,

    /// An adjustment made to the [CIE 1931] chromaticity *y* value.
    ///
    /// Positive values make the colors more magenta. Negative values make the
    /// colors greener. This has no effect on luminance (brightness).
    ///
    /// [CIE 1931]: https://en.wikipedia.org/wiki/CIE_1931_color_space#CIE_xy_chromaticity_diagram_and_the_CIE_xyY_color_space
    pub tint: f32,

    /// An adjustment to the [hue], in radians.
    ///
    /// Adjusting this value changes the perceived colors in the image: red to
    /// yellow to green to blue, etc. It has no effect on the saturation or
    /// brightness of the colors.
    ///
    /// [hue]: https://en.wikipedia.org/wiki/HSL_and_HSV#Formal_derivation
    pub hue: f32,

    /// Saturation adjustment applied after tonemapping.
    /// Values below 1.0 desaturate, with a value of 0.0 resulting in a grayscale image
    /// with luminance defined by ITU-R BT.709
    /// Values above 1.0 increase saturation.
    pub post_saturation: f32,

    /// The luminance (brightness) ranges that are considered part of the
    /// "midtones" of the image.
    ///
    /// This affects which [`ColorGradingSection`]s apply to which colors. Note
    /// that the sections smoothly blend into one another, to avoid abrupt
    /// transitions.
    ///
    /// The default value is 0.2 to 0.7.
    pub midtones_range: Range<f32>,
}

/// The [`ColorGrading`] structure, packed into the most efficient form for the
/// GPU.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ColorGradingUniform {
    pub balance: Mat3,
    pub saturation: Vec3,
    pub contrast: Vec3,
    pub gamma: Vec3,
    pub gain: Vec3,
    pub lift: Vec3,
    pub midtone_range: Vec2,
    pub exposure: f32,
    pub hue: f32,
    pub post_saturation: f32,
}

/// A section of color grading values that can be selectively applied to
/// shadows, midtones, and highlights.
#[derive(Reflect, Debug, Copy, Clone, PartialEq)]
#[reflect(Clone, PartialEq)]
pub struct ColorGradingSection {
    /// Values below 1.0 desaturate, with a value of 0.0 resulting in a grayscale image
    /// with luminance defined by ITU-R BT.709.
    /// Values above 1.0 increase saturation.
    pub saturation: f32,

    /// Adjusts the range of colors.
    ///
    /// A value of 1.0 applies no changes. Values below 1.0 move the colors more
    /// toward a neutral gray. Values above 1.0 spread the colors out away from
    /// the neutral gray.
    pub contrast: f32,

    /// A nonlinear luminance adjustment, mainly affecting the high end of the
    /// range.
    ///
    /// This is the *n* exponent in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub gamma: f32,

    /// A linear luminance adjustment, mainly affecting the middle part of the
    /// range.
    ///
    /// This is the *s* factor in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub gain: f32,

    /// A fixed luminance adjustment, mainly affecting the lower part of the
    /// range.
    ///
    /// This is the *o* term in the standard [ASC CDL] formula for color
    /// correction:
    ///
    /// ```text
    /// out = (i × s + o)ⁿ
    /// ```
    ///
    /// [ASC CDL]: https://en.wikipedia.org/wiki/ASC_CDL#Combined_Function
    pub lift: f32,
}

impl Default for ColorGradingGlobal {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            temperature: 0.0,
            tint: 0.0,
            hue: 0.0,
            post_saturation: 1.0,
            midtones_range: 0.2..0.7,
        }
    }
}

impl Default for ColorGradingSection {
    fn default() -> Self {
        Self {
            saturation: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            gain: 1.0,
            lift: 0.0,
        }
    }
}

impl ColorGrading {
    /// Creates a new [`ColorGrading`] instance in which shadows, midtones, and
    /// highlights all have the same set of color grading values.
    pub fn with_identical_sections(
        global: ColorGradingGlobal,
        section: ColorGradingSection,
    ) -> ColorGrading {
        ColorGrading {
            global,
            highlights: section,
            midtones: section,
            shadows: section,
        }
    }

    /// Returns an iterator that visits the shadows, midtones, and highlights
    /// sections, in that order.
    pub fn all_sections(&self) -> impl Iterator<Item = &ColorGradingSection> {
        [&self.shadows, &self.midtones, &self.highlights].into_iter()
    }

    /// Applies the given mutating function to the shadows, midtones, and
    /// highlights sections, in that order.
    ///
    /// Returns an array composed of the results of such evaluation, in that
    /// order.
    pub fn all_sections_mut(&mut self) -> impl Iterator<Item = &mut ColorGradingSection> {
        [&mut self.shadows, &mut self.midtones, &mut self.highlights].into_iter()
    }
}

#[derive(Clone, ShaderType)]
pub struct ViewUniform {
    pub clip_from_world: Mat4,
    pub unjittered_clip_from_world: Mat4,
    pub world_from_clip: Mat4,
    pub world_from_view: Mat4,
    pub view_from_world: Mat4,
    /// Typically a column-major right-handed projection matrix, one of either:
    ///
    /// Perspective (infinite reverse z)
    /// ```text
    /// f = 1 / tan(fov_y_radians / 2)
    ///
    /// ⎡ f / aspect  0   0     0 ⎤
    /// ⎢          0  f   0     0 ⎥
    /// ⎢          0  0   0  near ⎥
    /// ⎣          0  0  -1     0 ⎦
    /// ```
    ///
    /// Orthographic
    /// ```text
    /// w = right - left
    /// h = top - bottom
    /// d = far - near
    /// cw = -right - left
    /// ch = -top - bottom
    ///
    /// ⎡ 2 / w      0      0   cw / w ⎤
    /// ⎢     0  2 / h      0   ch / h ⎥
    /// ⎢     0      0  1 / d  far / d ⎥
    /// ⎣     0      0      0        1 ⎦
    /// ```
    ///
    /// `clip_from_view[3][3] == 1.0` is the standard way to check if a projection is orthographic
    ///
    /// Glam matrices are column major, so for example getting the near plane of a perspective projection is `clip_from_view[3][2]`
    ///
    /// Custom projections are also possible however.
    pub clip_from_view: Mat4,
    pub view_from_clip: Mat4,
    pub world_position: Vec3,
    pub exposure: f32,
    // viewport(x_origin, y_origin, width, height)
    pub viewport: Vec4,
    pub main_pass_viewport: Vec4,
    /// 6 world-space half spaces (normal: vec3, distance: f32) ordered left, right, top, bottom, near, far.
    /// The normal vectors point towards the interior of the frustum.
    /// A half space contains `p` if `normal.dot(p) + distance > 0.`
    pub frustum: [Vec4; 6],
    pub color_grading: ColorGradingUniform,
    pub mip_bias: f32,
    pub frame_count: u32,
}

#[derive(Resource)]
pub struct ViewUniforms {
    pub uniforms: DynamicUniformBuffer<ViewUniform>,
}

impl FromWorld for ViewUniforms {
    fn from_world(world: &mut World) -> Self {
        let mut uniforms = DynamicUniformBuffer::default();
        uniforms.set_label(Some("view_uniforms_buffer"));

        let render_device = world.resource::<RenderDevice>();
        if render_device.limits().max_storage_buffers_per_shader_stage > 0 {
            uniforms.add_usages(BufferUsages::STORAGE);
        }

        Self { uniforms }
    }
}

#[derive(Component)]
pub struct ViewUniformOffset {
    pub offset: u32,
}

#[derive(Component)]
pub struct ViewTarget {
    main_textures: MainTargetTextures,
    main_texture_format: TextureFormat,
    /// 0 represents `main_textures.a`, 1 represents `main_textures.b`
    /// This is shared across view targets with the same render target
    main_texture: Arc<AtomicUsize>,
    out_texture: OutputColorAttachment,
}

/// Contains [`OutputColorAttachment`] used for each target present on any view in the current
/// frame, after being prepared by [`prepare_view_attachments`]. Users that want to override
/// the default output color attachment for a specific target can do so by adding a
/// [`OutputColorAttachment`] to this resource before [`prepare_view_targets`] is called.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ViewTargetAttachments(HashMap<NormalizedRenderTarget, OutputColorAttachment>);

pub struct PostProcessWrite<'a> {
    pub source: &'a TextureView,
    pub source_texture: &'a Texture,
    pub destination: &'a TextureView,
    pub destination_texture: &'a Texture,
}

impl From<ColorGrading> for ColorGradingUniform {
    fn from(component: ColorGrading) -> Self {
        // Compute the balance matrix that will be used to apply the white
        // balance adjustment to an RGB color. Our general approach will be to
        // convert both the color and the developer-supplied white point to the
        // LMS color space, apply the conversion, and then convert back.
        //
        // First, we start with the CIE 1931 *xy* values of the standard D65
        // illuminant:
        // <https://en.wikipedia.org/wiki/Standard_illuminant#D65_values>
        //
        // We then adjust them based on the developer's requested white balance.
        let white_point_xy = D65_XY + vec2(-component.global.temperature, component.global.tint);

        // Convert the white point from CIE 1931 *xy* to LMS. First, we convert to XYZ:
        //
        //                  Y          Y
        //     Y = 1    X = ─ x    Z = ─ (1 - x - y)
        //                  y          y
        //
        // Then we convert from XYZ to LMS color space, using the CAM16 matrix
        // from <https://en.wikipedia.org/wiki/LMS_color_space#Later_CIECAMs>:
        //
        //     ⎡ L ⎤   ⎡  0.401   0.650  -0.051 ⎤ ⎡ X ⎤
        //     ⎢ M ⎥ = ⎢ -0.250   1.204   0.046 ⎥ ⎢ Y ⎥
        //     ⎣ S ⎦   ⎣ -0.002   0.049   0.953 ⎦ ⎣ Z ⎦
        //
        // The following formula is just a simplification of the above.

        let white_point_lms = vec3(0.701634, 1.15856, -0.904175)
            + (vec3(-0.051461, 0.045854, 0.953127)
                + vec3(0.452749, -0.296122, -0.955206) * white_point_xy.x)
                / white_point_xy.y;

        // Now that we're in LMS space, perform the white point scaling.
        let white_point_adjustment = Mat3::from_diagonal(D65_LMS / white_point_lms);

        // Finally, combine the RGB → LMS → corrected LMS → corrected RGB
        // pipeline into a single 3×3 matrix.
        let balance = LMS_TO_RGB * white_point_adjustment * RGB_TO_LMS;

        Self {
            balance,
            saturation: vec3(
                component.shadows.saturation,
                component.midtones.saturation,
                component.highlights.saturation,
            ),
            contrast: vec3(
                component.shadows.contrast,
                component.midtones.contrast,
                component.highlights.contrast,
            ),
            gamma: vec3(
                component.shadows.gamma,
                component.midtones.gamma,
                component.highlights.gamma,
            ),
            gain: vec3(
                component.shadows.gain,
                component.midtones.gain,
                component.highlights.gain,
            ),
            lift: vec3(
                component.shadows.lift,
                component.midtones.lift,
                component.highlights.lift,
            ),
            midtone_range: vec2(
                component.global.midtones_range.start,
                component.global.midtones_range.end,
            ),
            exposure: component.global.exposure,
            hue: component.global.hue,
            post_saturation: component.global.post_saturation,
        }
    }
}

/// Add this component to a camera to disable *indirect mode*.
///
/// Indirect mode, automatically enabled on supported hardware, allows Bevy to
/// offload transform and cull operations to the GPU, reducing CPU overhead.
/// Doing this, however, reduces the amount of control that your app has over
/// instancing decisions. In certain circumstances, you may want to disable
/// indirect drawing so that your app can manually instance meshes as it sees
/// fit. See the `custom_shader_instancing` example.
///
/// The vast majority of applications will not need to use this component, as it
/// generally reduces rendering performance.
///
/// Note: This component should only be added when initially spawning a camera. Adding
/// or removing after spawn can result in unspecified behavior.
#[derive(Component, Default)]
pub struct NoIndirectDrawing;

impl ViewTarget {
    pub const TEXTURE_FORMAT_HDR: TextureFormat = TextureFormat::Rgba16Float;

    /// Retrieve this target's main texture's color attachment.
    pub fn get_color_attachment(&self) -> RenderPassColorAttachment<'_> {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            self.main_textures.a.get_attachment()
        } else {
            self.main_textures.b.get_attachment()
        }
    }

    /// Retrieve this target's "unsampled" main texture's color attachment.
    pub fn get_unsampled_color_attachment(&self) -> RenderPassColorAttachment<'_> {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            self.main_textures.a.get_unsampled_attachment()
        } else {
            self.main_textures.b.get_unsampled_attachment()
        }
    }

    /// The "main" unsampled texture.
    pub fn main_texture(&self) -> &Texture {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a.texture.texture
        } else {
            &self.main_textures.b.texture.texture
        }
    }

    /// The _other_ "main" unsampled texture.
    /// In most cases you should use [`Self::main_texture`] instead and never this.
    /// The textures will naturally be swapped when [`Self::post_process_write`] is called.
    ///
    /// A use case for this is to be able to prepare a bind group for all main textures
    /// ahead of time.
    pub fn main_texture_other(&self) -> &Texture {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.b.texture.texture
        } else {
            &self.main_textures.a.texture.texture
        }
    }

    /// The "main" unsampled texture.
    pub fn main_texture_view(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.a.texture.default_view
        } else {
            &self.main_textures.b.texture.default_view
        }
    }

    /// The _other_ "main" unsampled texture view.
    /// In most cases you should use [`Self::main_texture_view`] instead and never this.
    /// The textures will naturally be swapped when [`Self::post_process_write`] is called.
    ///
    /// A use case for this is to be able to prepare a bind group for all main textures
    /// ahead of time.
    pub fn main_texture_other_view(&self) -> &TextureView {
        if self.main_texture.load(Ordering::SeqCst) == 0 {
            &self.main_textures.b.texture.default_view
        } else {
            &self.main_textures.a.texture.default_view
        }
    }

    /// The "main" sampled texture.
    pub fn sampled_main_texture(&self) -> Option<&Texture> {
        self.main_textures
            .a
            .resolve_target
            .as_ref()
            .map(|sampled| &sampled.texture)
    }

    /// The "main" sampled texture view.
    pub fn sampled_main_texture_view(&self) -> Option<&TextureView> {
        self.main_textures
            .a
            .resolve_target
            .as_ref()
            .map(|sampled| &sampled.default_view)
    }

    #[inline]
    pub fn main_texture_format(&self) -> TextureFormat {
        self.main_texture_format
    }

    /// Returns `true` if and only if the main texture is [`Self::TEXTURE_FORMAT_HDR`]
    #[inline]
    pub fn is_hdr(&self) -> bool {
        self.main_texture_format == ViewTarget::TEXTURE_FORMAT_HDR
    }

    /// The final texture this view will render to.
    #[inline]
    pub fn out_texture(&self) -> &TextureView {
        &self.out_texture.view
    }

    pub fn out_texture_color_attachment(
        &self,
        clear_color: Option<LinearRgba>,
    ) -> RenderPassColorAttachment<'_> {
        self.out_texture.get_attachment(clear_color)
    }

    /// The format of the final texture this view will render to
    #[inline]
    pub fn out_texture_format(&self) -> TextureFormat {
        self.out_texture.format
    }

    /// This will start a new "post process write", which assumes that the caller
    /// will write the [`PostProcessWrite`]'s `source` to the `destination`.
    ///
    /// `source` is the "current" main texture. This will internally flip this
    /// [`ViewTarget`]'s main texture to the `destination` texture, so the caller
    /// _must_ ensure `source` is copied to `destination`, with or without modifications.
    /// Failing to do so will cause the current main texture information to be lost.
    pub fn post_process_write(&self) -> PostProcessWrite<'_> {
        let old_is_a_main_texture = self.main_texture.fetch_xor(1, Ordering::SeqCst);
        // if the old main texture is a, then the post processing must write from a to b
        if old_is_a_main_texture == 0 {
            self.main_textures.b.mark_as_cleared();
            PostProcessWrite {
                source: &self.main_textures.a.texture.default_view,
                source_texture: &self.main_textures.a.texture.texture,
                destination: &self.main_textures.b.texture.default_view,
                destination_texture: &self.main_textures.b.texture.texture,
            }
        } else {
            self.main_textures.a.mark_as_cleared();
            PostProcessWrite {
                source: &self.main_textures.b.texture.default_view,
                source_texture: &self.main_textures.b.texture.texture,
                destination: &self.main_textures.a.texture.default_view,
                destination_texture: &self.main_textures.a.texture.texture,
            }
        }
    }
}

#[derive(Component)]
pub struct ViewDepthTexture {
    pub texture: Texture,
    attachment: DepthAttachment,
}

impl ViewDepthTexture {
    pub fn new(texture: CachedTexture, clear_value: Option<f32>) -> Self {
        Self {
            texture: texture.texture,
            attachment: DepthAttachment::new(texture.default_view, clear_value),
        }
    }

    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment<'_> {
        self.attachment.get_attachment(store)
    }

    pub fn view(&self) -> &TextureView {
        &self.attachment.view
    }
}

pub fn prepare_view_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut view_uniforms: ResMut<ViewUniforms>,
    views: Query<(
        Entity,
        Option<&ExtractedCamera>,
        &ExtractedView,
        Option<&Frustum>,
        Option<&TemporalJitter>,
        Option<&MipBias>,
        Option<&MainPassResolutionOverride>,
    )>,
    frame_count: Res<FrameCount>,
) {
    let view_iter = views.iter();
    let view_count = view_iter.len();
    let Some(mut writer) =
        view_uniforms
            .uniforms
            .get_writer(view_count, &render_device, &render_queue)
    else {
        return;
    };
    for (
        entity,
        extracted_camera,
        extracted_view,
        frustum,
        temporal_jitter,
        mip_bias,
        resolution_override,
    ) in &views
    {
        let viewport = extracted_view.viewport.as_vec4();
        let mut main_pass_viewport = viewport;
        if let Some(resolution_override) = resolution_override {
            main_pass_viewport.z = resolution_override.0.x as f32;
            main_pass_viewport.w = resolution_override.0.y as f32;
        }

        let unjittered_projection = extracted_view.clip_from_view;
        let mut clip_from_view = unjittered_projection;

        if let Some(temporal_jitter) = temporal_jitter {
            temporal_jitter.jitter_projection(&mut clip_from_view, main_pass_viewport.zw());
        }

        let view_from_clip = clip_from_view.inverse();
        let world_from_view = extracted_view.world_from_view.to_matrix();
        let view_from_world = world_from_view.inverse();

        let clip_from_world = if temporal_jitter.is_some() {
            clip_from_view * view_from_world
        } else {
            extracted_view
                .clip_from_world
                .unwrap_or_else(|| clip_from_view * view_from_world)
        };

        // Map Frustum type to shader array<vec4<f32>, 6>
        let frustum = frustum
            .map(|frustum| frustum.half_spaces.map(|h| h.normal_d()))
            .unwrap_or([Vec4::ZERO; 6]);

        let view_uniforms = ViewUniformOffset {
            offset: writer.write(&ViewUniform {
                clip_from_world,
                unjittered_clip_from_world: unjittered_projection * view_from_world,
                world_from_clip: world_from_view * view_from_clip,
                world_from_view,
                view_from_world,
                clip_from_view,
                view_from_clip,
                world_position: extracted_view.world_from_view.translation(),
                exposure: extracted_camera
                    .map(|c| c.exposure)
                    .unwrap_or_else(|| Exposure::default().exposure()),
                viewport,
                main_pass_viewport,
                frustum,
                color_grading: extracted_view.color_grading.clone().into(),
                mip_bias: mip_bias.unwrap_or(&MipBias(0.0)).0,
                frame_count: frame_count.0,
            }),
        };

        commands.entity(entity).insert(view_uniforms);
    }
}

#[derive(Clone)]
struct MainTargetTextures {
    a: ColorAttachment,
    b: ColorAttachment,
    /// 0 represents `main_textures.a`, 1 represents `main_textures.b`
    /// This is shared across view targets with the same render target
    main_texture: Arc<AtomicUsize>,
}

/// Prepares the view target [`OutputColorAttachment`] for each view in the current frame.
pub fn prepare_view_attachments(
    windows: Res<ExtractedWindows>,
    images: Res<RenderAssets<GpuImage>>,
    manual_texture_views: Res<ManualTextureViews>,
    cameras: Query<&ExtractedCamera>,
    mut view_target_attachments: ResMut<ViewTargetAttachments>,
) {
    for camera in cameras.iter() {
        let Some(target) = &camera.target else {
            continue;
        };

        match view_target_attachments.entry(target.clone()) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                let Some(attachment) = target
                    .get_texture_view(&windows, &images, &manual_texture_views)
                    .cloned()
                    .zip(target.get_texture_format(&windows, &images, &manual_texture_views))
                    .map(|(view, format)| {
                        OutputColorAttachment::new(view.clone(), format.add_srgb_suffix())
                    })
                else {
                    continue;
                };
                entry.insert(attachment);
            }
        };
    }
}

/// Clears the view target [`OutputColorAttachment`]s.
pub fn clear_view_attachments(mut view_target_attachments: ResMut<ViewTargetAttachments>) {
    view_target_attachments.clear();
}

pub fn prepare_view_targets(
    mut commands: Commands,
    clear_color_global: Res<ClearColor>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    cameras: Query<(
        Entity,
        &ExtractedCamera,
        &ExtractedView,
        &CameraMainTextureUsages,
        &Msaa,
    )>,
    view_target_attachments: Res<ViewTargetAttachments>,
) {
    let mut textures = <HashMap<_, _>>::default();
    for (entity, camera, view, texture_usage, msaa) in cameras.iter() {
        let (Some(target_size), Some(target)) = (camera.physical_target_size, &camera.target)
        else {
            continue;
        };

        let Some(out_attachment) = view_target_attachments.get(target) else {
            continue;
        };

        let main_texture_format = if view.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let clear_color = match camera.clear_color {
            ClearColorConfig::Custom(color) => Some(color),
            ClearColorConfig::None => None,
            _ => Some(clear_color_global.0),
        };

        let (a, b, sampled, main_texture) = textures
            .entry((camera.target.clone(), texture_usage.0, view.hdr, msaa))
            .or_insert_with(|| {
                let descriptor = TextureDescriptor {
                    label: None,
                    size: target_size.to_extents(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: main_texture_format,
                    usage: texture_usage.0,
                    view_formats: match main_texture_format {
                        TextureFormat::Bgra8Unorm => &[TextureFormat::Bgra8UnormSrgb],
                        TextureFormat::Rgba8Unorm => &[TextureFormat::Rgba8UnormSrgb],
                        _ => &[],
                    },
                };
                let a = texture_cache.get(
                    &render_device,
                    TextureDescriptor {
                        label: Some("main_texture_a"),
                        ..descriptor
                    },
                );
                let b = texture_cache.get(
                    &render_device,
                    TextureDescriptor {
                        label: Some("main_texture_b"),
                        ..descriptor
                    },
                );
                let sampled = if msaa.samples() > 1 {
                    let sampled = texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            label: Some("main_texture_sampled"),
                            size: target_size.to_extents(),
                            mip_level_count: 1,
                            sample_count: msaa.samples(),
                            dimension: TextureDimension::D2,
                            format: main_texture_format,
                            usage: TextureUsages::RENDER_ATTACHMENT,
                            view_formats: descriptor.view_formats,
                        },
                    );
                    Some(sampled)
                } else {
                    None
                };
                let main_texture = Arc::new(AtomicUsize::new(0));
                (a, b, sampled, main_texture)
            });

        let converted_clear_color = clear_color.map(Into::into);

        let main_textures = MainTargetTextures {
            a: ColorAttachment::new(a.clone(), sampled.clone(), converted_clear_color),
            b: ColorAttachment::new(b.clone(), sampled.clone(), converted_clear_color),
            main_texture: main_texture.clone(),
        };

        commands.entity(entity).insert(ViewTarget {
            main_texture: main_textures.main_texture.clone(),
            main_textures,
            main_texture_format,
            out_texture: out_attachment.clone(),
        });
    }
}
