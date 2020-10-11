mod webgl2_render_context;
//mod webgl2_render_graph_executor;
mod utils;
mod webgl2_render_resource_context;

pub use webgl2_render_context::*;
//pub use webgl2_render_graph_executor::*;
pub use webgl2_render_resource_context::*;

pub use js_sys;
pub use wasm_bindgen::JsCast;
pub use web_sys::{
    WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlShader, WebGlTexture,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

pub type Gl = WebGl2RenderingContext;

pub use utils::*;
