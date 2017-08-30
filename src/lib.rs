#![allow(bad_style)]

mod ffi;

use std::os::raw::*;
use std::mem;
use std::ptr;

/// Rust FFI for the MikkTSpace implementation.
const INTERFACE: ffi::SMikkTSpaceInterface = ffi::SMikkTSpaceInterface {
    m_getNumFaces: faces,
    m_getNumVerticesOfFace: vertices,
    m_getPosition: position,
    m_getNormal: normal,
    m_getTexCoord: tex_coord,
    m_setTSpaceBasic: set_tspace_basic,
    m_setTSpace: set_tspace,
};

/// Rust front-end API for tangent generation.
struct Closures<'a> {
    /// Returns the number of vertices per face.
    pub vertices_per_face: &'a Fn() -> usize,
    
    /// Returns the number of faces.
    pub face_count: &'a Fn() -> usize,
    
    /// Returns the positions of the indexed face.
    pub position: &'a Fn(usize, usize) -> &'a [f32; 3],

    /// Returns the normals of the indexed face.
    pub normal: &'a Fn(usize, usize) -> &'a [f32; 3],

    /// Returns the texture co-ordinates of the indexed face.
    pub tex_coord: &'a Fn(usize, usize) -> &'a [f32; 2],

    /// Sets the generated tangent for the indexed face.
    pub set_tangent: &'a mut FnMut(usize, usize, [f32; 4]),
}

/// Returns the number of faces (triangles/quads) on the mesh to be processed.
extern "C" fn faces(pContext: *const ffi::SMikkTSpaceContext) -> c_int {
    unsafe {
        let x = (*pContext).m_pUserData as *const Closures;
        ((*x).face_count)() as c_int
    }
}

/// Returns the number of vertices on face number iFace
/// iFace is a number in the range {0, 1, ..., getNumFaces()-1}
extern "C" fn vertices(
    pContext: *const ffi::SMikkTSpaceContext,
    _iFace: c_int,
) -> c_int {
    unsafe {
        let x = (*pContext).m_pUserData as *const Closures;
        ((*x).vertices_per_face)() as c_int
    }
}

/// Returns the position of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
extern "C" fn position(
    pContext: *const ffi::SMikkTSpaceContext,
    fvPosOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let x = (*pContext).m_pUserData as *const Closures;
        let slice = ((*x).position)(iFace as usize, iVert as usize);
        let src = slice.as_ptr() as *const c_float;
        ptr::copy_nonoverlapping::<c_float>(src, fvPosOut, 3);
    }
}

/// Returns the normal of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
extern "C" fn normal(
    pContext: *const ffi::SMikkTSpaceContext,
    fvNormOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let x = (*pContext).m_pUserData as *const Closures;
        let slice = ((*x).normal)(iFace as usize, iVert as usize);
        let src = slice.as_ptr() as *const c_float;
        ptr::copy_nonoverlapping::<c_float>(src, fvNormOut, 3);
    }
}

/// Returns the texcoord of the referenced face of vertex number
/// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
extern "C" fn tex_coord(
    pContext: *const ffi::SMikkTSpaceContext,
    fvTexcOut: *mut c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let x = (*pContext).m_pUserData as *const Closures;
        let slice = ((*x).tex_coord)(iFace as usize, iVert as usize);
        let src = slice.as_ptr() as *const c_float;
        ptr::copy_nonoverlapping::<c_float>(src, fvTexcOut, 2);
    }
}

/// Returns the tangent and its sign to the application.
extern "C" fn set_tspace_basic(
    pContext: *mut ffi::SMikkTSpaceContext,
    fvTangent: *const c_float,
    fSign: c_float,
    iFace: c_int,
    iVert: c_int,
) {
    unsafe {
        let x = (*pContext).m_pUserData as *mut Closures;
        let mut tangent: [f32; 4] = mem::uninitialized();
        let dst: *mut c_float = tangent.as_mut_ptr();
        ptr::copy_nonoverlapping::<c_float>(fvTangent, dst, 3);
        tangent[3] = fSign;
        ((*x).set_tangent)(iFace as usize, iVert as usize, tangent);
    }
}

/// Returns tangent space results to the application.
extern "C" fn set_tspace(
    pContext: *mut ffi::SMikkTSpaceContext,
    fvTangent: *const c_float,
    _fvBiTangent: *const c_float,
    _fMagS: c_float,
    _fMagT: c_float,
    bIsOrientationPreserving: ffi::tbool,
    iFace: c_int,
    iVert: c_int,
) {
    let fSign = if bIsOrientationPreserving != 0 { 1.0 } else { -1.0 };
    set_tspace_basic(pContext, fvTangent, fSign, iFace, iVert);
}

impl<'a> Closures<'a> {
    /// Generates tangents.
    pub fn generate(mut self) -> bool {
        let ctx = ffi::SMikkTSpaceContext {
            m_pInterface: &INTERFACE,
            m_pUserData: &mut self as *mut Closures as *mut c_void,
        };
        unsafe {
            ffi::genTangSpaceDefault(&ctx) == ffi::TTRUE
        }
    }
}

/// Generates tangents.
pub fn generate<'a>(
    vertices_per_face: &'a Fn() -> usize,
    face_count: &'a Fn() -> usize,
    position: &'a Fn(usize, usize) -> &'a [f32; 3],
    normal: &'a Fn(usize, usize) -> &'a [f32; 3],
    tex_coord: &'a Fn(usize, usize) -> &'a [f32; 2],
    set_tangent: &'a mut FnMut(usize, usize, [f32; 4]),
) -> bool {
    let closures = Closures {
        vertices_per_face,
        face_count,
        position,
        normal,
        tex_coord,
        set_tangent,
    };
    closures.generate()
}
