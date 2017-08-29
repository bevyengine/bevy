
#![allow(bad_style)]
#![allow(dead_code)]

use std::os::raw::*;

pub type tbool = c_int;
pub const TFALSE: tbool = 0;
pub const TTRUE: tbool = 1;

#[repr(C)]
pub struct SMikkTSpaceInterface {
    /// Returns the number of faces (triangles/quads) on the mesh to be processed.
    pub m_getNumFaces: extern "C" fn(pContext: *const SMikkTSpaceContext) -> c_int,

    /// Returns the number of vertices on face number iFace
    /// iFace is a number in the range {0, 1, ..., getNumFaces()-1}
    pub m_getNumVerticesOfFace: extern "C" fn(
        pContext: *const SMikkTSpaceContext,
        iFace: c_int,
    ) -> c_int,

    /// Returns the position of the referenced face of vertex number
    /// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
    pub m_getPosition: extern "C" fn(
        pContext: *const SMikkTSpaceContext,
        fvPosOut: *mut c_float,
        iFace: c_int,
        iVert: c_int,
    ),

    /// Returns the normal of the referenced face of vertex number
    /// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
    pub m_getNormal: extern "C" fn(
        pContext: *const SMikkTSpaceContext,
        fvNormOut: *mut c_float,
        iFace: c_int,
        iVert: c_int,
    ),

    /// Returns the texcoord of the referenced face of vertex number
    /// iVert, in the range {0,1,2} for triangles, and {0,1,2,3} for quads.
    pub m_getTexCoord: extern "C" fn(
        pContext: *const SMikkTSpaceContext,
        fvTexcOut: *mut c_float,
        iFace: c_int,
        iVert: c_int,
    ),

    /// either (or both) of the two setTSpace callbacks can be set.
    /// The call-back m_setTSpaceBasic() is sufficient for basic normal mapping.

    /// This function is used to return the tangent and fSign to the application.
    /// fvTangent is a unit length vector.
    /// For normal maps it is sufficient to use the following simplified version of the bitangent which is generated at pixel/vertex level.
    /// bitangent = fSign * cross(vN, tangent);
    /// Note that the results are returned unindexed. It is possible to generate a new index list
    /// But averaging/overwriting tangent spaces by using an already existing index list WILL produce INCRORRECT results.
    /// DO NOT! use an already existing index list.
    pub m_setTSpaceBasic: extern "C" fn(
        pContext: *mut SMikkTSpaceContext,
        fvTangent: *const c_float,
        fSign: *const c_float,
        iFace: c_int,
        iVert: c_int,
    ),

    /// This function is used to return tangent space results to the application.
    /// fvTangent and fvBiTangent are unit length vectors and fMagS and fMagT are their
    /// true magnitudes which can be used for relief mapping effects.
    /// fvBiTangent is the "real" bitangent and thus may not be perpendicular to fvTangent.
    /// However, both are perpendicular to the vertex normal.
    /// For normal maps it is sufficient to use the following simplified version of the bitangent which is generated at pixel/vertex level.
    /// fSign = bIsOrientationPreserving ? 1.0f : (-1.0f);
    /// bitangent = fSign * cross(vN, tangent);
    /// Note that the results are returned unindexed. It is possible to generate a new index list
    /// But averaging/overwriting tangent spaces by using an already existing index list WILL produce INCRORRECT results.
    /// DO NOT! use an already existing index list.
    pub m_setTSpace: extern "C" fn(
        pContext: *mut SMikkTSpaceContext,
        fvTangent: *const c_float,
        fvBiTangent: *const c_float,
        fMagS: *const c_float,
        fMagT: *const c_float,
        bIsOrientationPreserving: tbool,
        iFace: c_int,
        iVert: c_int,
    ),
}

/// these are both thread safe!
/// Default (recommended) fAngularThreshold is 180 degrees (which means threshold disabled)
extern "system" {
    pub fn genTangSpaceDefault(pContext: *const SMikkTSpaceContext) -> tbool;
    #[allow(dead_code)]
    pub fn genTangSpace(pContext: *const SMikkTSpaceContext, fAngularThreshold: c_float) -> tbool;
}

/// To avoid visual errors (distortions/unwanted hard edges in lighting), when using sampled normal maps, the
/// normal map sampler must use the exact inverse of the pixel shader transformation.
/// The most efficient transformation we can possibly do in the pixel shader is
/// achieved by using, directly, the "unnormalized" interpolated tangent, bitangent and vertex normal: vT, vB and vN.
/// pixel shader (fast transform out)
/// vNout = normalize( vNt.x * vT + vNt.y * vB + vNt.z * vN );
/// where vNt is the tangent space normal. The normal map sampler must likewise use the
/// interpolated and "unnormalized" tangent, bitangent and vertex normal to be compliant with the pixel shader.
/// sampler does (exact inverse of pixel shader):
/// float3 row0 = cross(vB, vN);
/// float3 row1 = cross(vN, vT);
/// float3 row2 = cross(vT, vB);
/// float fSign = dot(vT, row0)<0 ? -1 : 1;
/// vNt = normalize( fSign * float3(dot(vNout,row0), dot(vNout,row1), dot(vNout,row2)) );
/// where vNout is the sampled normal in some chosen 3D space.
///
/// Should you choose to reconstruct the bitangent in the pixel shader instead
/// of the vertex shader, as explained earlier, then be sure to do this in the normal map sampler also.
/// Finally, beware of quad triangulations. If the normal map sampler doesn't use the same triangulation of
/// quads as your renderer then problems will occur since the interpolated tangent spaces will differ
/// eventhough the vertex level tangent spaces match. This can be solved either by triangulating before
/// sampling/exporting or by using the order-independent choice of diagonal for splitting quads suggested earlier.
/// However, this must be used both by the sampler and your tools/rendering pipeline.

#[repr(C)]
pub struct SMikkTSpaceContext {
    /// initialized with callback functions
    pub m_pInterface: *const SMikkTSpaceInterface,
    /// pointer to client side mesh data etc. (passed as the first parameter with every interface call)
    pub m_pUserData: *mut c_void,
}
