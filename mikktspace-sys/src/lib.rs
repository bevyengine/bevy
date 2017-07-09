
mod ffi;

use std::os::raw::*;
use std::mem;

const INTERFACE: ffi::SMikkTSpaceInterface = ffi::SMikkTSpaceInterface {
    m_getNumFaces: faces,
    m_getNumVerticesOfFace: vertices,
    m_getPosition: position,
    m_getNormal: normal,
    m_getTexCoord: tex_coord,
    m_setTSpaceBasic: set_tspace_basic,
    m_setTSpace: set_tspace,
};

pub struct Context {
    faces: i32,
}

/// Returns the number of faces (triangles/quads) on the mesh to be processed.
#[no_mangle]
extern "C" fn faces(pContext: *const ffi::SMikkTSpaceContext) -> c_int {
    unsafe {
        let m: *const Context = mem::transmute(pContext);
        (*m).faces as c_int
    }
}

/// Returns the number of vertices on face number iFace
/// iFace is a number in the range {0, 1, ..., getNumFaces()-1}
#[no_mangle]
extern "C" fn vertices(
    pContext: *const ffi::SMikkTSpaceContext,
    iFace: c_int,
) -> c_int {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
        unimplemented!()
    }
}

/// Returns the position of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
#[no_mangle]
extern "C" fn position(
    pContext: *const ffi::SMikkTSpaceContext,
    fvPosOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
    }
}

/// Returns the normal of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
#[no_mangle]
extern "C" fn normal(
    pContext: *const ffi::SMikkTSpaceContext,
    fvPosOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
    }
}

/// Returns the texcoord of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
#[no_mangle]
extern "C" fn tex_coord(
    pContext: *const ffi::SMikkTSpaceContext,
    fvTexcOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
    }
}

/// Returns the tangent and its sign to the application.
#[no_mangle]
extern "C" fn set_tspace_basic(
    pContext: *const ffi::SMikkTSpaceContext,
    fvTangent: *const c_float,
    fSign: *const c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
    }
}

/// Returns tangent space results to the application.
#[no_mangle]
extern "C" fn set_tspace(
    pContext: *const ffi::SMikkTSpaceContext,
    fvTangent: *const c_float,
    fvBiTangent: *const c_float,
    fMagS: *const c_float,
    fMagT: *const c_float,
    bIsOrientationPreserving: ffi::tbool,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let _: *const Context = mem::transmute(pContext);
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            faces: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
