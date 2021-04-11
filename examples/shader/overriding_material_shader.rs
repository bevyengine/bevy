use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        shader::ShaderStage,
        texture::{Extent3d, TextureDimension, TextureFormat},
    },
};
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}
//This is the handle for our custom pipeline, we have to create a new pipeline for our custom sprite rendering, or they won't get rendered
//This is a pre-made handle so it will be easier to use
pub const CUSTOM_SPRITE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 2785347850338765446);

//This bundle exists to make spawning shader-overriden sprites easier
#[derive(Bundle)]
struct CustomSpriteBundle {
    #[bundle]
    sprite: SpriteBundle,
}
impl Default for CustomSpriteBundle {
    fn default() -> Self {
        Self {
            sprite: SpriteBundle {
                //We make sure the sprite is rendered in our custom pipeline
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    CUSTOM_SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                ..Default::default()
            },
        }
    }
}
impl CustomSpriteBundle {
    fn new(material_handle: Handle<ColorMaterial>, transform: Transform) -> Self {
        let mut default = Self::default();
        default.sprite.material = material_handle;
        default.sprite.transform = transform;
        return default;
    }
}
//We're copying everything from the original fragment shader, but changing stuff in the main() function of the shader.
//We're expecting to see a green triangle on the left side of the screen, and a green triangle on a checkerboard pattern on the right side of the screen.
//If this isn't the result you are getting, something probably changed in the original vertex and fragment shaders in the bevy version you are using.
//For your own shaders, consider writing them in their own files so you could get syntax highlighting for glsl (see e.g. bevy_sprite/src/render/)
const FRAGMENT_SHADER: &str = r#"
#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform ColorMaterial_color {
    vec4 Color;
};

# ifdef COLORMATERIAL_TEXTURE 
layout(set = 1, binding = 1) uniform texture2D ColorMaterial_texture;
layout(set = 1, binding = 2) uniform sampler ColorMaterial_texture_sampler;
# endif

void main() {
    //Color is the color we receive from ColorMaterial's color field, we send it to the fragment shader from the vertex shader
    vec4 color = Color;
# ifdef COLORMATERIAL_TEXTURE
    //Get the color of the current fragment from the texture
    vec4 texture_pixel_color =  texture(
        sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),
        v_Uv);
    //If the color is transparent
    if(texture_pixel_color.a == 0.0){
        //Get the fragment's pixel position in the texture
        vec2 pixel = v_Uv * textureSize(sampler2D(ColorMaterial_texture, ColorMaterial_texture_sampler),0);
        //If the pixel's position fits in the equation y=x+b where b%2==0  (a straight line of pixels from every even row, creates a checkerboard pattern)
        if(mod(int(pixel.y)-int(pixel.x),2.0)==0.0){
            //Color the fragment slightly white
            texture_pixel_color = vec4(0.502,0.502,0.502,1.0);
        }
        else{
            //Color the fragment even whiter
            texture_pixel_color = vec4(0.802,0.802,0.802,1.0);
        }
    }
    //Mutliply the ColorMaterial's color with: 
    //  If the alpha is zero, by our chckerboard pattern
    //  If the alpha is non-zero, by the texture's pixel color
    color *= texture_pixel_color;
# endif
    o_Target = color;
}
"#;
fn setup(
    mut commands: Commands,
    mut render_pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    //If the sprite's original render pipeline already exists
    if let Some(original_sprite_render_pipeline) =
        render_pipelines.get(bevy::sprite::SPRITE_PIPELINE_HANDLE)
    {
        //We can clone the pipeline so we won't have to copy the code for setting it up
        let mut pipeline_clone = original_sprite_render_pipeline.clone();
        //In this example, we only want to override the fragment shader and so:
        pipeline_clone.shader_stages.fragment =
            Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER)));
        //Adding our custom pipeline and making it untracked so it won't get removed automatically when no sprite uses it
        render_pipelines.set_untracked(CUSTOM_SPRITE_PIPELINE_HANDLE, pipeline_clone);

        //Creating our texture
        let texture_size = 20;
        //The vec's size is 4 times our texture's area, since every pixel is built out of 4 u8s (R,G,B,A)
        let mut transparent_texture_vec: Vec<u8> =
            Vec::with_capacity(texture_size * texture_size * 4);
        for y in 0..texture_size {
            for x in 0..texture_size {
                //Make the pixel green, if y is bigger than x
                if y > x {
                    transparent_texture_vec.push(0); //r
                    transparent_texture_vec.push(255); //g
                    transparent_texture_vec.push(0); //b
                    transparent_texture_vec.push(255); //a
                }
                //Else, we don't need to draw this pixel(make it transparent by setting the alpha to zero)
                else {
                    transparent_texture_vec.push(255); //r
                    transparent_texture_vec.push(255); //g
                    transparent_texture_vec.push(255); //b
                    transparent_texture_vec.push(0); //a
                }
            }
        }
        let texture = Texture::new(
            Extent3d::new(texture_size as u32, texture_size as u32, 1),
            TextureDimension::D2,
            transparent_texture_vec,
            //At the moment of writing, only 4 formats are supported so make sure you use one of them and not others
            TextureFormat::Rgba8UnormSrgb,
        );
        //Adding the texture to the Texture assets and getting it's handle
        let texture_handle = textures.add(texture);
        //Creating our shared material
        let material_handle = materials.add(ColorMaterial::texture(texture_handle));
        //Actually spawning our custom sprite bundle, we send it a clone of our material handle since we also want to render the same material in a regular sprite to compare the two
        commands.spawn_bundle(CustomSpriteBundle::new(
            material_handle.to_owned(),
            Transform::from_translation(Vec3::new(texture_size as f32 / 2.0, 0.0, 0.0)),
        ));
        //Spawning a regular sprite bundle to compare the two visually
        commands.spawn_bundle(SpriteBundle {
            material: material_handle,
            transform: Transform::from_translation(Vec3::new(
                texture_size as f32 / 2.0 * -1.0,
                0.0,
                0.0,
            )),
            ..Default::default()
        });
        //Creating a camera so we could see our sprite
        let mut camera = OrthographicCameraBundle::new_2d();
        //The scale is small so we could actually see the pixels(small scale = zooming in)
        camera.transform.scale = Vec3::new(0.05, 0.05, 1.0);
        //Spawning the camera
        commands.spawn_bundle(camera);
    }
}
