use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        shader::{ShaderStage},
        texture::{Extent3d,TextureDimension,TextureFormat},
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
#[derive(Bundle, Default)]
struct CustomSpriteBundle {
    #[bundle]
    sprite: SpriteBundle,
}
impl CustomSpriteBundle {
    fn new(material_handle: Handle<ColorMaterial>) -> Self {
        Self {
            sprite: SpriteBundle {
                //We set the material for the sprite
                material: material_handle,
                //We make sure the sprite is rendered in our custom pipeline
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    CUSTOM_SPRITE_PIPELINE_HANDLE.typed(),
                )]),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
//We're copying everything from the original fragment shader, but changing stuff in the main() function of the shader
//If the shader isn't working, probably something changed in the original vertex and fragment shaders in the version you are using
//In your own shader, you could write it in a 'your-shader-name'.frag file and do like what is done in bevy_sprite/src/render/mod.rs (it's prettier, and you can get syntax highlighting for glsl, but i don't want to clutter the examples folder) 
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
    //  If the alpha is non-zero, by the the texture's pixel color
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
    if let Some(original_sprite_render_graph) =
        render_pipelines.get(bevy::sprite::SPRITE_PIPELINE_HANDLE)
    {
        //We can clone the pipeline so we won't have to copy the code for setting it up
        let mut pipeline_clone = original_sprite_render_graph.clone();
        //In this example, we only want to override the fragment shader and so:
        pipeline_clone.shader_stages.fragment =
            Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER)));
        //Adding our custom pipeline and making it untracked so it won't get removed automatically when no sprite uses it
        render_pipelines.set_untracked(CUSTOM_SPRITE_PIPELINE_HANDLE, pipeline_clone);

        //Spawning our custom sprite
        let texture_size = 1000;
        //The vec's size is 4 times our texture's area, since every pixel is built out of 4 u8s (R,G,B,A)
        let mut transparent_texture_vec: Vec<u8> =
            Vec::with_capacity(texture_size * texture_size * 4);
        for _y in 0..texture_size {
            for _x in 0..texture_size {
                //The first three don't really matter in this case, since we set the alpha to zero in the 4th row, and our shader will thus ignore the rgb values we give here
                //If you want to play with it, you can change the values here to see what will happen
                transparent_texture_vec.push(255); //r
                transparent_texture_vec.push(255); //g
                transparent_texture_vec.push(255); //b
                transparent_texture_vec.push(0); //a
            }
        }
        let texture = Texture::new(
            Extent3d::new(texture_size as u32, texture_size as u32, 1),
            TextureDimension::D2,
            transparent_texture_vec,
            //At the moment of writing, only 4 formats are supported so make sure you use one of them and not others
            TextureFormat::Rgba8UnormSrgb,
        );
        let texture_handle = textures.add(texture);
        let material_handle = materials.add(ColorMaterial::texture(texture_handle));
        //Actually spawning our custom sprite bundle
        commands.spawn_bundle(CustomSpriteBundle::new(material_handle));
        //Creating a camera so we could see our sprite
        let mut camera = OrthographicCameraBundle::new_2d();
        //The scale is small so we could actually see the pixels(small scale = zooming in)
        camera.transform.scale = Vec3::new(0.1,0.1,1.0); 
        //Spawning the camera
        commands.spawn_bundle(camera);
    }
}
