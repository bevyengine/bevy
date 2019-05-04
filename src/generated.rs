#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_mut,
    unused_assignments,
    unused_variables
)]

use std::mem::size_of;

use {
    libc::{c_int, c_ulong},
    nalgebra::Vector3,
};

use crate::{face_vert_to_index, get_normal, get_position, get_tex_coord, Geometry};

extern "C" {
    #[no_mangle]
    fn memcpy(_: *mut libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn free(__ptr: *mut libc::c_void);
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct STSpace {
    pub vOs: Vector3<f32>,
    pub fMagS: libc::c_float,
    pub vOt: Vector3<f32>,
    pub fMagT: libc::c_float,
    pub iCounter: libc::c_int,
    pub bOrient: bool,
}
// To avoid visual errors (distortions/unwanted hard edges in lighting), when using sampled normal maps, the
// normal map sampler must use the exact inverse of the pixel shader transformation.
// The most efficient transformation we can possibly do in the pixel shader is
// achieved by using, directly, the "unnormalized" interpolated tangent, bitangent and vertex normal: vT, vB and vN.
// pixel shader (fast transform out)
// vNout = normalize( vNt.x * vT + vNt.y * vB + vNt.z * vN );
// where vNt is the tangent space normal. The normal map sampler must likewise use the
// interpolated and "unnormalized" tangent, bitangent and vertex normal to be compliant with the pixel shader.
// sampler does (exact inverse of pixel shader):
// float3 row0 = cross(vB, vN);
// float3 row1 = cross(vN, vT);
// float3 row2 = cross(vT, vB);
// float fSign = dot(vT, row0)<0 ? -1 : 1;
// vNt = normalize( fSign * float3(dot(vNout,row0), dot(vNout,row1), dot(vNout,row2)) );
// where vNout is the sampled normal in some chosen 3D space.
//
// Should you choose to reconstruct the bitangent in the pixel shader instead
// of the vertex shader, as explained earlier, then be sure to do this in the normal map sampler also.
// Finally, beware of quad triangulations. If the normal map sampler doesn't use the same triangulation of
// quads as your renderer then problems will occur since the interpolated tangent spaces will differ
// eventhough the vertex level tangent spaces match. This can be solved either by triangulating before
// sampling/exporting or by using the order-independent choice of diagonal for splitting quads suggested earlier.
// However, this must be used both by the sampler and your tools/rendering pipeline.
// internal structure

#[derive(Copy, Clone)]
#[repr(C)]
pub struct STriInfo {
    pub FaceNeighbors: [libc::c_int; 3],
    pub AssignedGroup: [*mut SGroup; 3],
    pub vOs: Vector3<f32>,
    pub vOt: Vector3<f32>,
    pub fMagS: libc::c_float,
    pub fMagT: libc::c_float,
    pub iOrgFaceNumber: libc::c_int,
    pub iFlag: libc::c_int,
    pub iTSpacesOffs: libc::c_int,
    pub vert_num: [libc::c_uchar; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SGroup {
    pub iNrFaces: libc::c_int,
    pub pFaceIndices: *mut libc::c_int,
    pub iVertexRepresentitive: libc::c_int,
    pub bOrientPreservering: bool,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SSubGroup {
    pub iNrFaces: libc::c_int,
    pub pTriMembers: *mut libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union SEdge {
    pub unnamed: unnamed,
    pub array: [libc::c_int; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct unnamed {
    pub i0: libc::c_int,
    pub i1: libc::c_int,
    pub f: libc::c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct STmpVert {
    pub vert: [libc::c_float; 3],
    pub index: libc::c_int,
}

pub unsafe fn genTangSpace<I: Geometry>(
    geometry: &mut I,
    fAngularThreshold: libc::c_float,
) -> bool {
    // count nr_triangles
    let mut piTriListIn: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut piGroupTrianglesBuffer: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut pTriInfos: *mut STriInfo = 0 as *mut STriInfo;
    let mut pGroups: *mut SGroup = 0 as *mut SGroup;
    let mut psTspace: *mut STSpace = 0 as *mut STSpace;
    let mut iNrTrianglesIn = 0;
    let mut f = 0;
    let mut t = 0;
    let mut i = 0;
    let mut iNrTSPaces = 0;
    let mut iTotTris = 0;
    let mut iDegenTriangles = 0;
    let mut iNrMaxGroups = 0;
    let mut iNrActiveGroups: libc::c_int = 0i32;
    let mut index: libc::c_int = 0i32;
    let iNrFaces = geometry.get_num_faces();
    let mut bRes: bool = false;
    let fThresCos: libc::c_float = ((fAngularThreshold * 3.14159265358979323846f64 as libc::c_float
        / 180.0f32) as libc::c_double)
        .cos() as libc::c_float;
    f = 0;
    while f < iNrFaces {
        let verts = geometry.get_num_vertices_of_face(f);
        if verts == 3 {
            iNrTrianglesIn += 1
        } else if verts == 4 {
            iNrTrianglesIn += 2
        }
        f += 1
    }
    if iNrTrianglesIn <= 0 {
        return false;
    }
    piTriListIn = malloc((size_of::<libc::c_int>() * 3 * iNrTrianglesIn) as c_ulong) as *mut c_int;
    pTriInfos = malloc((size_of::<STriInfo>() * iNrTrianglesIn) as c_ulong) as *mut STriInfo;
    if piTriListIn.is_null() || pTriInfos.is_null() {
        if !piTriListIn.is_null() {
            free(piTriListIn as *mut libc::c_void);
        }
        if !pTriInfos.is_null() {
            free(pTriInfos as *mut libc::c_void);
        }
        return false;
    }
    iNrTSPaces =
        GenerateInitialVerticesIndexList(pTriInfos, piTriListIn, geometry, iNrTrianglesIn as c_int);
    GenerateSharedVerticesIndexList(piTriListIn, geometry, iNrTrianglesIn);
    iTotTris = iNrTrianglesIn;
    iDegenTriangles = 0;
    t = 0;
    while t < iTotTris as usize {
        let i0 = *piTriListIn.offset((t * 3 + 0) as isize);
        let i1 = *piTriListIn.offset((t * 3 + 1) as isize);
        let i2 = *piTriListIn.offset((t * 3 + 2) as isize);
        let p0 = get_position(geometry, i0 as usize);
        let p1 = get_position(geometry, i1 as usize);
        let p2 = get_position(geometry, i2 as usize);
        if p0 == p1 || p0 == p2 || p1 == p2 {
            (*pTriInfos.offset(t as isize)).iFlag |= 1i32;
            iDegenTriangles += 1
        }
        t += 1
    }
    iNrTrianglesIn = iTotTris - iDegenTriangles;
    DegenPrologue(
        pTriInfos,
        piTriListIn,
        iNrTrianglesIn as c_int,
        iTotTris as c_int,
    );
    InitTriInfo(
        pTriInfos,
        piTriListIn as *const libc::c_int,
        geometry,
        iNrTrianglesIn,
    );
    iNrMaxGroups = iNrTrianglesIn * 3;
    pGroups = malloc((size_of::<SGroup>() * iNrMaxGroups) as c_ulong) as *mut SGroup;
    piGroupTrianglesBuffer =
        malloc((size_of::<c_int>() * iNrTrianglesIn * 3) as c_ulong) as *mut c_int;
    if pGroups.is_null() || piGroupTrianglesBuffer.is_null() {
        if !pGroups.is_null() {
            free(pGroups as *mut libc::c_void);
        }
        if !piGroupTrianglesBuffer.is_null() {
            free(piGroupTrianglesBuffer as *mut libc::c_void);
        }
        free(piTriListIn as *mut libc::c_void);
        free(pTriInfos as *mut libc::c_void);
        return false;
    }
    iNrActiveGroups = Build4RuleGroups(
        pTriInfos,
        pGroups,
        piGroupTrianglesBuffer,
        piTriListIn as *const libc::c_int,
        iNrTrianglesIn as c_int,
    );
    psTspace = malloc((size_of::<STSpace>() * iNrTSPaces) as c_ulong) as *mut STSpace;
    if psTspace.is_null() {
        free(piTriListIn as *mut libc::c_void);
        free(pTriInfos as *mut libc::c_void);
        free(pGroups as *mut libc::c_void);
        free(piGroupTrianglesBuffer as *mut libc::c_void);
        return false;
    }
    memset(
        psTspace as *mut libc::c_void,
        0,
        (size_of::<STSpace>() * iNrTSPaces) as c_ulong,
    );
    t = 0;
    while t < iNrTSPaces {
        (*psTspace.offset(t as isize)).vOs.x = 1.0f32;
        (*psTspace.offset(t as isize)).vOs.y = 0.0f32;
        (*psTspace.offset(t as isize)).vOs.z = 0.0f32;
        (*psTspace.offset(t as isize)).fMagS = 1.0f32;
        (*psTspace.offset(t as isize)).vOt.x = 0.0f32;
        (*psTspace.offset(t as isize)).vOt.y = 1.0f32;
        (*psTspace.offset(t as isize)).vOt.z = 0.0f32;
        (*psTspace.offset(t as isize)).fMagT = 1.0f32;
        t += 1
    }
    bRes = GenerateTSpaces(
        psTspace,
        pTriInfos as *const STriInfo,
        pGroups as *const SGroup,
        iNrActiveGroups,
        piTriListIn as *const libc::c_int,
        fThresCos,
        geometry,
    );
    free(pGroups as *mut libc::c_void);
    free(piGroupTrianglesBuffer as *mut libc::c_void);
    if !bRes {
        free(pTriInfos as *mut libc::c_void);
        free(piTriListIn as *mut libc::c_void);
        free(psTspace as *mut libc::c_void);
        return false;
    }
    DegenEpilogue(
        psTspace,
        pTriInfos,
        piTriListIn,
        geometry,
        iNrTrianglesIn as c_int,
        iTotTris as c_int,
    );
    free(pTriInfos as *mut libc::c_void);
    free(piTriListIn as *mut libc::c_void);
    index = 0i32;
    f = 0;
    while f < iNrFaces {
        let verts_0 = geometry.get_num_vertices_of_face(f);
        if !(verts_0 != 3 && verts_0 != 4) {
            i = 0;
            while i < verts_0 {
                let mut pTSpace: *const STSpace =
                    &mut *psTspace.offset(index as isize) as *mut STSpace;
                let mut tang = Vector3::new((*pTSpace).vOs.x, (*pTSpace).vOs.y, (*pTSpace).vOs.z);
                let mut bitang = Vector3::new((*pTSpace).vOt.x, (*pTSpace).vOt.y, (*pTSpace).vOt.z);
                geometry.set_tangent(
                    tang,
                    bitang,
                    (*pTSpace).fMagS,
                    (*pTSpace).fMagT,
                    (*pTSpace).bOrient,
                    f,
                    i,
                );
                index += 1;
                i += 1
            }
        }
        f += 1
    }
    free(psTspace as *mut libc::c_void);
    return true;
}
unsafe fn DegenEpilogue<I: Geometry>(
    mut psTspace: *mut STSpace,
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *mut libc::c_int,
    geometry: &mut I,
    iNrTrianglesIn: libc::c_int,
    iTotTris: libc::c_int,
) {
    let mut t: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    t = iNrTrianglesIn;
    while t < iTotTris {
        let bSkip: bool = if (*pTriInfos.offset(t as isize)).iFlag & 2i32 != 0i32 {
            true
        } else {
            false
        };
        if !bSkip {
            i = 0i32;
            while i < 3i32 {
                let index1: libc::c_int = *piTriListIn.offset((t * 3i32 + i) as isize);
                let mut bNotFound: bool = true;
                let mut j: libc::c_int = 0i32;
                while bNotFound && j < 3i32 * iNrTrianglesIn {
                    let index2: libc::c_int = *piTriListIn.offset(j as isize);
                    if index1 == index2 {
                        bNotFound = false
                    } else {
                        j += 1
                    }
                }
                if !bNotFound {
                    let iTri: libc::c_int = j / 3i32;
                    let iVert: libc::c_int = j % 3i32;
                    let iSrcVert: libc::c_int =
                        (*pTriInfos.offset(iTri as isize)).vert_num[iVert as usize] as libc::c_int;
                    let iSrcOffs: libc::c_int = (*pTriInfos.offset(iTri as isize)).iTSpacesOffs;
                    let iDstVert: libc::c_int =
                        (*pTriInfos.offset(t as isize)).vert_num[i as usize] as libc::c_int;
                    let iDstOffs: libc::c_int = (*pTriInfos.offset(t as isize)).iTSpacesOffs;
                    *psTspace.offset((iDstOffs + iDstVert) as isize) =
                        *psTspace.offset((iSrcOffs + iSrcVert) as isize)
                }
                i += 1
            }
        }
        t += 1
    }
    t = 0i32;
    while t < iNrTrianglesIn {
        if (*pTriInfos.offset(t as isize)).iFlag & 2i32 != 0i32 {
            let mut vDstP = Vector3::new(0.0, 0.0, 0.0);
            let mut iOrgF: libc::c_int = -1i32;
            let mut i_0: libc::c_int = 0i32;
            let mut bNotFound_0: bool = false;
            let mut pV: *mut libc::c_uchar = (*pTriInfos.offset(t as isize)).vert_num.as_mut_ptr();
            let mut iFlag: libc::c_int = 1i32 << *pV.offset(0isize) as libc::c_int
                | 1i32 << *pV.offset(1isize) as libc::c_int
                | 1i32 << *pV.offset(2isize) as libc::c_int;
            let mut iMissingIndex: libc::c_int = 0i32;
            if iFlag & 2i32 == 0i32 {
                iMissingIndex = 1i32
            } else if iFlag & 4i32 == 0i32 {
                iMissingIndex = 2i32
            } else if iFlag & 8i32 == 0i32 {
                iMissingIndex = 3i32
            }
            iOrgF = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
            vDstP = get_position(
                geometry,
                face_vert_to_index(iOrgF as usize, iMissingIndex as usize),
            );
            bNotFound_0 = true;
            i_0 = 0i32;
            while bNotFound_0 && i_0 < 3i32 {
                let iVert_0: libc::c_int = *pV.offset(i_0 as isize) as libc::c_int;
                let vSrcP = get_position(
                    geometry,
                    face_vert_to_index(iOrgF as usize, iVert_0 as usize),
                );
                if vSrcP == vDstP {
                    let iOffs: libc::c_int = (*pTriInfos.offset(t as isize)).iTSpacesOffs;
                    *psTspace.offset((iOffs + iMissingIndex) as isize) =
                        *psTspace.offset((iOffs + iVert_0) as isize);
                    bNotFound_0 = false
                } else {
                    i_0 += 1
                }
            }
        }
        t += 1
    }
}

unsafe fn GenerateTSpaces<I: Geometry>(
    mut psTspace: *mut STSpace,
    mut pTriInfos: *const STriInfo,
    mut pGroups: *const SGroup,
    iNrActiveGroups: libc::c_int,
    mut piTriListIn: *const libc::c_int,
    fThresCos: libc::c_float,
    geometry: &mut I,
) -> bool {
    let mut pSubGroupTspace: *mut STSpace = 0 as *mut STSpace;
    let mut pUniSubGroups: *mut SSubGroup = 0 as *mut SSubGroup;
    let mut pTmpMembers: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut iMaxNrFaces = 0;
    let mut iUniqueTspaces: libc::c_int = 0i32;
    let mut g: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    g = 0i32;
    while g < iNrActiveGroups {
        if iMaxNrFaces < (*pGroups.offset(g as isize)).iNrFaces {
            iMaxNrFaces = (*pGroups.offset(g as isize)).iNrFaces
        }
        g += 1
    }
    if iMaxNrFaces == 0i32 {
        return true;
    }
    pSubGroupTspace =
        malloc((size_of::<STSpace>() * iMaxNrFaces as usize) as c_ulong) as *mut STSpace;
    pUniSubGroups =
        malloc((size_of::<SSubGroup>() * iMaxNrFaces as usize) as c_ulong) as *mut SSubGroup;
    pTmpMembers =
        malloc((size_of::<libc::c_int>() * iMaxNrFaces as usize) as c_ulong) as *mut c_int;
    if pSubGroupTspace.is_null() || pUniSubGroups.is_null() || pTmpMembers.is_null() {
        if !pSubGroupTspace.is_null() {
            free(pSubGroupTspace as *mut libc::c_void);
        }
        if !pUniSubGroups.is_null() {
            free(pUniSubGroups as *mut libc::c_void);
        }
        if !pTmpMembers.is_null() {
            free(pTmpMembers as *mut libc::c_void);
        }
        return false;
    }
    iUniqueTspaces = 0i32;
    g = 0i32;
    while g < iNrActiveGroups {
        let mut pGroup: *const SGroup = &*pGroups.offset(g as isize) as *const SGroup;
        let mut iUniqueSubGroups: libc::c_int = 0i32;
        let mut s: libc::c_int = 0i32;
        i = 0i32;
        while i < (*pGroup).iNrFaces {
            let f: libc::c_int = *(*pGroup).pFaceIndices.offset(i as isize);
            let mut index: libc::c_int = -1i32;
            let mut iVertIndex: libc::c_int = -1i32;
            let mut iOF_1: libc::c_int = -1i32;
            let mut iMembers: usize = 0;
            let mut j: libc::c_int = 0i32;
            let mut l: libc::c_int = 0i32;
            let mut tmp_group: SSubGroup = SSubGroup {
                iNrFaces: 0,
                pTriMembers: 0 as *mut libc::c_int,
            };
            let mut bFound: bool = false;
            let mut n = Vector3::new(0.0, 0.0, 0.0);
            let mut vOs = Vector3::new(0.0, 0.0, 0.0);
            let mut vOt = Vector3::new(0.0, 0.0, 0.0);
            if (*pTriInfos.offset(f as isize)).AssignedGroup[0usize] == pGroup as *mut SGroup {
                index = 0i32
            } else if (*pTriInfos.offset(f as isize)).AssignedGroup[1usize] == pGroup as *mut SGroup
            {
                index = 1i32
            } else if (*pTriInfos.offset(f as isize)).AssignedGroup[2usize] == pGroup as *mut SGroup
            {
                index = 2i32
            }
            iVertIndex = *piTriListIn.offset((f * 3i32 + index) as isize);
            n = get_normal(geometry, iVertIndex as usize);
            vOs = (*pTriInfos.offset(f as isize)).vOs
                - (n.dot(&(*pTriInfos.offset(f as isize)).vOs) * n);
            vOt = (*pTriInfos.offset(f as isize)).vOt
                - (n.dot(&(*pTriInfos.offset(f as isize)).vOt) * n);
            if VNotZero(vOs) {
                vOs = Normalize(vOs)
            }
            if VNotZero(vOt) {
                vOt = Normalize(vOt)
            }
            iOF_1 = (*pTriInfos.offset(f as isize)).iOrgFaceNumber;
            iMembers = 0;
            j = 0i32;
            while j < (*pGroup).iNrFaces {
                let t: libc::c_int = *(*pGroup).pFaceIndices.offset(j as isize);
                let iOF_2: libc::c_int = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
                let mut vOs2 = (*pTriInfos.offset(t as isize)).vOs
                    - (n.dot(&(*pTriInfos.offset(t as isize)).vOs) * n);
                let mut vOt2 = (*pTriInfos.offset(t as isize)).vOt
                    - (n.dot(&(*pTriInfos.offset(t as isize)).vOt) * n);
                if VNotZero(vOs2) {
                    vOs2 = Normalize(vOs2)
                }
                if VNotZero(vOt2) {
                    vOt2 = Normalize(vOt2)
                }
                let bAny: bool = if ((*pTriInfos.offset(f as isize)).iFlag
                    | (*pTriInfos.offset(t as isize)).iFlag)
                    & 4i32
                    != 0i32
                {
                    true
                } else {
                    false
                };
                let bSameOrgFace: bool = iOF_1 == iOF_2;
                let fCosS: libc::c_float = vOs.dot(&vOs2);
                let fCosT: libc::c_float = vOt.dot(&vOt2);
                if bAny || bSameOrgFace || fCosS > fThresCos && fCosT > fThresCos {
                    let fresh0 = iMembers;
                    iMembers = iMembers + 1;
                    *pTmpMembers.offset(fresh0 as isize) = t
                }
                j += 1
            }
            tmp_group.iNrFaces = iMembers as c_int;
            tmp_group.pTriMembers = pTmpMembers;
            if iMembers > 1 {
                let mut uSeed: libc::c_uint = 39871946i32 as libc::c_uint;
                QuickSort(pTmpMembers, 0i32, (iMembers - 1) as c_int, uSeed);
            }
            bFound = false;
            l = 0i32;
            while l < iUniqueSubGroups && !bFound {
                bFound = CompareSubGroups(&mut tmp_group, &mut *pUniSubGroups.offset(l as isize));
                if !bFound {
                    l += 1
                }
            }
            if !bFound {
                let mut pIndices = malloc((size_of::<c_int>() * iMembers) as c_ulong) as *mut c_int;
                if pIndices.is_null() {
                    let mut s_0: libc::c_int = 0i32;
                    s_0 = 0i32;
                    while s_0 < iUniqueSubGroups {
                        free(
                            (*pUniSubGroups.offset(s_0 as isize)).pTriMembers as *mut libc::c_void,
                        );
                        s_0 += 1
                    }
                    free(pUniSubGroups as *mut libc::c_void);
                    free(pTmpMembers as *mut libc::c_void);
                    free(pSubGroupTspace as *mut libc::c_void);
                    return false;
                }
                (*pUniSubGroups.offset(iUniqueSubGroups as isize)).iNrFaces = iMembers as c_int;
                let ref mut fresh1 = (*pUniSubGroups.offset(iUniqueSubGroups as isize)).pTriMembers;
                *fresh1 = pIndices;
                memcpy(
                    pIndices as *mut libc::c_void,
                    tmp_group.pTriMembers as *const libc::c_void,
                    (iMembers as libc::c_ulong)
                        .wrapping_mul(::std::mem::size_of::<libc::c_int>() as libc::c_ulong),
                );
                *pSubGroupTspace.offset(iUniqueSubGroups as isize) = EvalTspace(
                    tmp_group.pTriMembers,
                    iMembers as c_int,
                    piTriListIn,
                    pTriInfos,
                    geometry,
                    (*pGroup).iVertexRepresentitive,
                );
                iUniqueSubGroups += 1
            }
            let iOffs: libc::c_int = (*pTriInfos.offset(f as isize)).iTSpacesOffs;
            let iVert: libc::c_int =
                (*pTriInfos.offset(f as isize)).vert_num[index as usize] as libc::c_int;
            let mut pTS_out: *mut STSpace =
                &mut *psTspace.offset((iOffs + iVert) as isize) as *mut STSpace;
            if (*pTS_out).iCounter == 1i32 {
                *pTS_out = AvgTSpace(pTS_out, &mut *pSubGroupTspace.offset(l as isize));
                (*pTS_out).iCounter = 2i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering
            } else {
                *pTS_out = *pSubGroupTspace.offset(l as isize);
                (*pTS_out).iCounter = 1i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering
            }
            i += 1
        }
        s = 0i32;
        while s < iUniqueSubGroups {
            free((*pUniSubGroups.offset(s as isize)).pTriMembers as *mut libc::c_void);
            s += 1
        }
        iUniqueTspaces += iUniqueSubGroups;
        g += 1
    }
    free(pUniSubGroups as *mut libc::c_void);
    free(pTmpMembers as *mut libc::c_void);
    free(pSubGroupTspace as *mut libc::c_void);
    return true;
}
unsafe fn AvgTSpace(mut pTS0: *const STSpace, mut pTS1: *const STSpace) -> STSpace {
    let mut ts_res: STSpace = STSpace {
        vOs: Vector3::new(0.0, 0.0, 0.0),
        fMagS: 0.,
        vOt: Vector3::new(0.0, 0.0, 0.0),
        fMagT: 0.,
        iCounter: 0,
        bOrient: false,
    };
    if (*pTS0).fMagS == (*pTS1).fMagS
        && (*pTS0).fMagT == (*pTS1).fMagT
        && (*pTS0).vOs == (*pTS1).vOs
        && (*pTS0).vOt == (*pTS1).vOt
    {
        ts_res.fMagS = (*pTS0).fMagS;
        ts_res.fMagT = (*pTS0).fMagT;
        ts_res.vOs = (*pTS0).vOs;
        ts_res.vOt = (*pTS0).vOt
    } else {
        ts_res.fMagS = 0.5f32 * ((*pTS0).fMagS + (*pTS1).fMagS);
        ts_res.fMagT = 0.5f32 * ((*pTS0).fMagT + (*pTS1).fMagT);
        ts_res.vOs = (*pTS0).vOs + (*pTS1).vOs;
        ts_res.vOt = (*pTS0).vOt + (*pTS1).vOt;
        if VNotZero(ts_res.vOs) {
            ts_res.vOs = Normalize(ts_res.vOs)
        }
        if VNotZero(ts_res.vOt) {
            ts_res.vOt = Normalize(ts_res.vOt)
        }
    }
    return ts_res;
}

unsafe fn Normalize(v: Vector3<f32>) -> Vector3<f32> {
    return (1.0 / v.magnitude()) * v;
}

unsafe fn VNotZero(v: Vector3<f32>) -> bool {
    NotZero(v.x) || NotZero(v.y) || NotZero(v.z)
}

unsafe fn NotZero(fX: libc::c_float) -> bool {
    fX.abs() > 1.17549435e-38f32
}

unsafe fn EvalTspace<I: Geometry>(
    mut face_indices: *mut libc::c_int,
    iFaces: libc::c_int,
    mut piTriListIn: *const libc::c_int,
    mut pTriInfos: *const STriInfo,
    geometry: &mut I,
    iVertexRepresentitive: libc::c_int,
) -> STSpace {
    let mut res: STSpace = STSpace {
        vOs: Vector3::new(0.0, 0.0, 0.0),
        fMagS: 0.,
        vOt: Vector3::new(0.0, 0.0, 0.0),
        fMagT: 0.,
        iCounter: 0,
        bOrient: false,
    };
    let mut fAngleSum: libc::c_float = 0i32 as libc::c_float;
    let mut face: libc::c_int = 0i32;
    res.vOs.x = 0.0f32;
    res.vOs.y = 0.0f32;
    res.vOs.z = 0.0f32;
    res.vOt.x = 0.0f32;
    res.vOt.y = 0.0f32;
    res.vOt.z = 0.0f32;
    res.fMagS = 0i32 as libc::c_float;
    res.fMagT = 0i32 as libc::c_float;
    face = 0i32;
    while face < iFaces {
        let f: libc::c_int = *face_indices.offset(face as isize);
        if (*pTriInfos.offset(f as isize)).iFlag & 4i32 == 0i32 {
            let mut n = Vector3::new(0.0, 0.0, 0.0);
            let mut vOs = Vector3::new(0.0, 0.0, 0.0);
            let mut vOt = Vector3::new(0.0, 0.0, 0.0);
            let mut p0 = Vector3::new(0.0, 0.0, 0.0);
            let mut p1 = Vector3::new(0.0, 0.0, 0.0);
            let mut p2 = Vector3::new(0.0, 0.0, 0.0);
            let mut v1 = Vector3::new(0.0, 0.0, 0.0);
            let mut v2 = Vector3::new(0.0, 0.0, 0.0);
            let mut fCos: libc::c_float = 0.;
            let mut fAngle: libc::c_float = 0.;
            let mut fMagS: libc::c_float = 0.;
            let mut fMagT: libc::c_float = 0.;
            let mut i: libc::c_int = -1i32;
            let mut index: libc::c_int = -1i32;
            let mut i0: libc::c_int = -1i32;
            let mut i1: libc::c_int = -1i32;
            let mut i2: libc::c_int = -1i32;
            if *piTriListIn.offset((3i32 * f + 0i32) as isize) == iVertexRepresentitive {
                i = 0i32
            } else if *piTriListIn.offset((3i32 * f + 1i32) as isize) == iVertexRepresentitive {
                i = 1i32
            } else if *piTriListIn.offset((3i32 * f + 2i32) as isize) == iVertexRepresentitive {
                i = 2i32
            }
            index = *piTriListIn.offset((3i32 * f + i) as isize);
            n = get_normal(geometry, index as usize);
            vOs = (*pTriInfos.offset(f as isize)).vOs
                - (n.dot(&(*pTriInfos.offset(f as isize)).vOs) * n);
            vOt = (*pTriInfos.offset(f as isize)).vOt
                - (n.dot(&(*pTriInfos.offset(f as isize)).vOt) * n);
            if VNotZero(vOs) {
                vOs = Normalize(vOs)
            }
            if VNotZero(vOt) {
                vOt = Normalize(vOt)
            }
            i2 = *piTriListIn.offset((3i32 * f + if i < 2i32 { i + 1i32 } else { 0i32 }) as isize);
            i1 = *piTriListIn.offset((3i32 * f + i) as isize);
            i0 = *piTriListIn.offset((3i32 * f + if i > 0i32 { i - 1i32 } else { 2i32 }) as isize);
            p0 = get_position(geometry, i0 as usize);
            p1 = get_position(geometry, i1 as usize);
            p2 = get_position(geometry, i2 as usize);
            v1 = p0 - p1;
            v2 = p2 - p1;
            v1 = v1 - (n.dot(&v1) * n);
            if VNotZero(v1) {
                v1 = Normalize(v1)
            }
            v2 = v2 - (n.dot(&v2) * n);
            if VNotZero(v2) {
                v2 = Normalize(v2)
            }
            fCos = v1.dot(&v2);
            fCos = if fCos > 1i32 as libc::c_float {
                1i32 as libc::c_float
            } else if fCos < -1i32 as libc::c_float {
                -1i32 as libc::c_float
            } else {
                fCos
            };
            fAngle = (fCos as libc::c_double).acos() as libc::c_float;
            fMagS = (*pTriInfos.offset(f as isize)).fMagS;
            fMagT = (*pTriInfos.offset(f as isize)).fMagT;
            res.vOs = res.vOs + (fAngle * vOs);
            res.vOt = res.vOt + (fAngle * vOt);
            res.fMagS += fAngle * fMagS;
            res.fMagT += fAngle * fMagT;
            fAngleSum += fAngle
        }
        face += 1
    }
    if VNotZero(res.vOs) {
        res.vOs = Normalize(res.vOs)
    }
    if VNotZero(res.vOt) {
        res.vOt = Normalize(res.vOt)
    }
    if fAngleSum > 0i32 as libc::c_float {
        res.fMagS /= fAngleSum;
        res.fMagT /= fAngleSum
    }
    return res;
}

unsafe fn CompareSubGroups(mut pg1: *const SSubGroup, mut pg2: *const SSubGroup) -> bool {
    let mut bStillSame: bool = true;
    let mut i: libc::c_int = 0i32;
    if (*pg1).iNrFaces != (*pg2).iNrFaces {
        return false;
    }
    while i < (*pg1).iNrFaces && bStillSame {
        bStillSame =
            if *(*pg1).pTriMembers.offset(i as isize) == *(*pg2).pTriMembers.offset(i as isize) {
                true
            } else {
                false
            };
        if bStillSame {
            i += 1
        }
    }
    return bStillSame;
}
unsafe fn QuickSort(
    mut pSortBuffer: *mut libc::c_int,
    mut iLeft: libc::c_int,
    mut iRight: libc::c_int,
    mut uSeed: libc::c_uint,
) {
    let mut iL: libc::c_int = 0;
    let mut iR: libc::c_int = 0;
    let mut n: libc::c_int = 0;
    let mut index: libc::c_int = 0;
    let mut iMid: libc::c_int = 0;
    let mut iTmp: libc::c_int = 0;

    // Random
    let mut t: libc::c_uint = uSeed & 31i32 as libc::c_uint;
    t = uSeed.rotate_left(t) | uSeed.rotate_right((32i32 as libc::c_uint).wrapping_sub(t));
    uSeed = uSeed.wrapping_add(t).wrapping_add(3i32 as libc::c_uint);
    // Random end

    iL = iLeft;
    iR = iRight;
    n = iR - iL + 1i32;
    index = uSeed.wrapping_rem(n as libc::c_uint) as libc::c_int;
    iMid = *pSortBuffer.offset((index + iL) as isize);
    loop {
        while *pSortBuffer.offset(iL as isize) < iMid {
            iL += 1
        }
        while *pSortBuffer.offset(iR as isize) > iMid {
            iR -= 1
        }
        if iL <= iR {
            iTmp = *pSortBuffer.offset(iL as isize);
            *pSortBuffer.offset(iL as isize) = *pSortBuffer.offset(iR as isize);
            *pSortBuffer.offset(iR as isize) = iTmp;
            iL += 1;
            iR -= 1
        }
        if !(iL <= iR) {
            break;
        }
    }
    if iLeft < iR {
        QuickSort(pSortBuffer, iLeft, iR, uSeed);
    }
    if iL < iRight {
        QuickSort(pSortBuffer, iL, iRight, uSeed);
    };
}
unsafe fn Build4RuleGroups(
    mut pTriInfos: *mut STriInfo,
    mut pGroups: *mut SGroup,
    mut piGroupTrianglesBuffer: *mut libc::c_int,
    mut piTriListIn: *const libc::c_int,
    iNrTrianglesIn: libc::c_int,
) -> libc::c_int {
    let iNrMaxGroups: libc::c_int = iNrTrianglesIn * 3i32;
    let mut iNrActiveGroups: libc::c_int = 0i32;
    let mut iOffset: libc::c_int = 0i32;
    let mut f: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    f = 0i32;
    while f < iNrTrianglesIn {
        i = 0i32;
        while i < 3i32 {
            if (*pTriInfos.offset(f as isize)).iFlag & 4i32 == 0i32
                && (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize].is_null()
            {
                let mut bOrPre: bool = false;
                let mut neigh_indexL: libc::c_int = 0;
                let mut neigh_indexR: libc::c_int = 0;
                let vert_index: libc::c_int = *piTriListIn.offset((f * 3i32 + i) as isize);
                let ref mut fresh2 = (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize];
                *fresh2 = &mut *pGroups.offset(iNrActiveGroups as isize) as *mut SGroup;
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize])
                    .iVertexRepresentitive = vert_index;
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).bOrientPreservering =
                    (*pTriInfos.offset(f as isize)).iFlag & 8i32 != 0i32;
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).iNrFaces = 0i32;
                let ref mut fresh3 =
                    (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).pFaceIndices;
                *fresh3 = &mut *piGroupTrianglesBuffer.offset(iOffset as isize) as *mut libc::c_int;
                iNrActiveGroups += 1;
                AddTriToGroup((*pTriInfos.offset(f as isize)).AssignedGroup[i as usize], f);
                bOrPre = if (*pTriInfos.offset(f as isize)).iFlag & 8i32 != 0i32 {
                    true
                } else {
                    false
                };
                neigh_indexL = (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize];
                neigh_indexR = (*pTriInfos.offset(f as isize)).FaceNeighbors
                    [(if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
                if neigh_indexL >= 0i32 {
                    let bAnswer: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexL,
                        (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize],
                    );
                    let bOrPre2: bool =
                        if (*pTriInfos.offset(neigh_indexL as isize)).iFlag & 8i32 != 0i32 {
                            true
                        } else {
                            false
                        };
                    let bDiff: bool = if bOrPre != bOrPre2 { true } else { false };
                }
                if neigh_indexR >= 0i32 {
                    let bAnswer_0: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexR,
                        (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize],
                    );
                    let bOrPre2_0: bool =
                        if (*pTriInfos.offset(neigh_indexR as isize)).iFlag & 8i32 != 0i32 {
                            true
                        } else {
                            false
                        };
                    let bDiff_0: bool = if bOrPre != bOrPre2_0 { true } else { false };
                }
                iOffset += (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).iNrFaces
            }
            i += 1
        }
        f += 1
    }
    return iNrActiveGroups;
}
// ///////////////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////////////////////////////
unsafe fn AssignRecur(
    mut piTriListIn: *const libc::c_int,
    mut psTriInfos: *mut STriInfo,
    iMyTriIndex: libc::c_int,
    mut pGroup: *mut SGroup,
) -> bool {
    let mut pMyTriInfo: *mut STriInfo =
        &mut *psTriInfos.offset(iMyTriIndex as isize) as *mut STriInfo;
    // track down vertex
    let iVertRep: libc::c_int = (*pGroup).iVertexRepresentitive;
    let mut pVerts: *const libc::c_int =
        &*piTriListIn.offset((3i32 * iMyTriIndex + 0i32) as isize) as *const libc::c_int;
    let mut i: libc::c_int = -1i32;
    if *pVerts.offset(0isize) == iVertRep {
        i = 0i32
    } else if *pVerts.offset(1isize) == iVertRep {
        i = 1i32
    } else if *pVerts.offset(2isize) == iVertRep {
        i = 2i32
    }
    if (*pMyTriInfo).AssignedGroup[i as usize] == pGroup {
        return true;
    } else {
        if !(*pMyTriInfo).AssignedGroup[i as usize].is_null() {
            return false;
        }
    }
    if (*pMyTriInfo).iFlag & 4i32 != 0i32 {
        if (*pMyTriInfo).AssignedGroup[0usize].is_null()
            && (*pMyTriInfo).AssignedGroup[1usize].is_null()
            && (*pMyTriInfo).AssignedGroup[2usize].is_null()
        {
            (*pMyTriInfo).iFlag &= !8i32;
            (*pMyTriInfo).iFlag |= if (*pGroup).bOrientPreservering {
                8i32
            } else {
                0i32
            }
        }
    }
    let bOrient: bool = if (*pMyTriInfo).iFlag & 8i32 != 0i32 {
        true
    } else {
        false
    };
    if bOrient != (*pGroup).bOrientPreservering {
        return false;
    }
    AddTriToGroup(pGroup, iMyTriIndex);
    (*pMyTriInfo).AssignedGroup[i as usize] = pGroup;
    let neigh_indexL: libc::c_int = (*pMyTriInfo).FaceNeighbors[i as usize];
    let neigh_indexR: libc::c_int =
        (*pMyTriInfo).FaceNeighbors[(if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
    if neigh_indexL >= 0i32 {
        AssignRecur(piTriListIn, psTriInfos, neigh_indexL, pGroup);
    }
    if neigh_indexR >= 0i32 {
        AssignRecur(piTriListIn, psTriInfos, neigh_indexR, pGroup);
    }
    return true;
}
unsafe fn AddTriToGroup(mut pGroup: *mut SGroup, iTriIndex: libc::c_int) {
    *(*pGroup).pFaceIndices.offset((*pGroup).iNrFaces as isize) = iTriIndex;
    (*pGroup).iNrFaces += 1;
}
unsafe fn InitTriInfo<I: Geometry>(
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *const libc::c_int,
    geometry: &mut I,
    iNrTrianglesIn: usize,
) {
    let mut f = 0;
    let mut i = 0;
    let mut t = 0;
    f = 0;
    while f < iNrTrianglesIn {
        i = 0i32;
        while i < 3i32 {
            (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize] = -1i32;
            let ref mut fresh4 = (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize];
            *fresh4 = 0 as *mut SGroup;
            (*pTriInfos.offset(f as isize)).vOs.x = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOs.y = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOs.z = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.x = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.y = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.z = 0.0f32;
            (*pTriInfos.offset(f as isize)).fMagS = 0i32 as libc::c_float;
            (*pTriInfos.offset(f as isize)).fMagT = 0i32 as libc::c_float;
            (*pTriInfos.offset(f as isize)).iFlag |= 4i32;
            i += 1
        }
        f += 1
    }
    f = 0;
    while f < iNrTrianglesIn {
        let v1 = get_position(geometry, *piTriListIn.offset((f * 3 + 0) as isize) as usize);
        let v2 = get_position(geometry, *piTriListIn.offset((f * 3 + 1) as isize) as usize);
        let v3 = get_position(geometry, *piTriListIn.offset((f * 3 + 2) as isize) as usize);
        let t1 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 0) as isize) as usize);
        let t2 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 1) as isize) as usize);
        let t3 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 2) as isize) as usize);
        let t21x: libc::c_float = t2.x - t1.x;
        let t21y: libc::c_float = t2.y - t1.y;
        let t31x: libc::c_float = t3.x - t1.x;
        let t31y: libc::c_float = t3.y - t1.y;
        let d1 = v2 - v1;
        let d2 = v3 - v1;
        let fSignedAreaSTx2: libc::c_float = t21x * t31y - t21y * t31x;
        let mut vOs = (t31y * d1) - (t21y * d2);
        let mut vOt = (-t31x * d1) + (t21x * d2);
        (*pTriInfos.offset(f as isize)).iFlag |= if fSignedAreaSTx2 > 0i32 as libc::c_float {
            8i32
        } else {
            0i32
        };
        if NotZero(fSignedAreaSTx2) {
            let fAbsArea: libc::c_float = fSignedAreaSTx2.abs();
            let fLenOs: libc::c_float = vOs.magnitude();
            let fLenOt: libc::c_float = vOt.magnitude();
            let fS: libc::c_float = if (*pTriInfos.offset(f as isize)).iFlag & 8i32 == 0i32 {
                -1.0f32
            } else {
                1.0f32
            };
            if NotZero(fLenOs) {
                (*pTriInfos.offset(f as isize)).vOs = (fS / fLenOs) * vOs
            }
            if NotZero(fLenOt) {
                (*pTriInfos.offset(f as isize)).vOt = (fS / fLenOt) * vOt
            }
            (*pTriInfos.offset(f as isize)).fMagS = fLenOs / fAbsArea;
            (*pTriInfos.offset(f as isize)).fMagT = fLenOt / fAbsArea;
            if NotZero((*pTriInfos.offset(f as isize)).fMagS)
                && NotZero((*pTriInfos.offset(f as isize)).fMagT)
            {
                (*pTriInfos.offset(f as isize)).iFlag &= !4i32
            }
        }
        f += 1
    }
    while t < iNrTrianglesIn - 1 {
        let iFO_a: libc::c_int = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
        let iFO_b: libc::c_int = (*pTriInfos.offset((t + 1) as isize)).iOrgFaceNumber;
        if iFO_a == iFO_b {
            let bIsDeg_a: bool = if (*pTriInfos.offset(t as isize)).iFlag & 1i32 != 0i32 {
                true
            } else {
                false
            };
            let bIsDeg_b: bool = if (*pTriInfos.offset((t + 1) as isize)).iFlag & 1i32 != 0i32 {
                true
            } else {
                false
            };
            if !(bIsDeg_a || bIsDeg_b) {
                let bOrientA: bool = if (*pTriInfos.offset(t as isize)).iFlag & 8i32 != 0i32 {
                    true
                } else {
                    false
                };
                let bOrientB: bool = if (*pTriInfos.offset((t + 1) as isize)).iFlag & 8i32 != 0i32 {
                    true
                } else {
                    false
                };
                if bOrientA != bOrientB {
                    let mut bChooseOrientFirstTri: bool = false;
                    if (*pTriInfos.offset((t + 1) as isize)).iFlag & 4i32 != 0i32 {
                        bChooseOrientFirstTri = true
                    } else if CalcTexArea(geometry, &*piTriListIn.offset((t * 3 + 0) as isize))
                        >= CalcTexArea(geometry, &*piTriListIn.offset(((t + 1) * 3 + 0) as isize))
                    {
                        bChooseOrientFirstTri = true
                    }
                    let t0 = if bChooseOrientFirstTri { t } else { t + 1 };
                    let t1_0 = if bChooseOrientFirstTri { t + 1 } else { t };
                    (*pTriInfos.offset(t1_0 as isize)).iFlag &= !8i32;
                    (*pTriInfos.offset(t1_0 as isize)).iFlag |=
                        (*pTriInfos.offset(t0 as isize)).iFlag & 8i32
                }
            }
            t += 2
        } else {
            t += 1
        }
    }
    let mut pEdges: *mut SEdge =
        malloc((size_of::<SEdge>() * iNrTrianglesIn * 3) as c_ulong) as *mut SEdge;
    if pEdges.is_null() {
        BuildNeighborsSlow(pTriInfos, piTriListIn, iNrTrianglesIn as c_int);
    } else {
        BuildNeighborsFast(pTriInfos, pEdges, piTriListIn, iNrTrianglesIn as c_int);
        free(pEdges as *mut libc::c_void);
    };
}
unsafe fn BuildNeighborsFast(
    mut pTriInfos: *mut STriInfo,
    mut pEdges: *mut SEdge,
    mut piTriListIn: *const libc::c_int,
    iNrTrianglesIn: libc::c_int,
) {
    // build array of edges
    // could replace with a random seed?
    let mut uSeed: libc::c_uint = 39871946i32 as libc::c_uint;
    let mut iEntries: libc::c_int = 0i32;
    let mut iCurStartIndex: libc::c_int = -1i32;
    let mut f: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    f = 0i32;
    while f < iNrTrianglesIn {
        i = 0i32;
        while i < 3i32 {
            let i0: libc::c_int = *piTriListIn.offset((f * 3i32 + i) as isize);
            let i1: libc::c_int =
                *piTriListIn.offset((f * 3i32 + if i < 2i32 { i + 1i32 } else { 0i32 }) as isize);
            (*pEdges.offset((f * 3i32 + i) as isize)).unnamed.i0 = if i0 < i1 { i0 } else { i1 };
            (*pEdges.offset((f * 3i32 + i) as isize)).unnamed.i1 = if !(i0 < i1) { i0 } else { i1 };
            (*pEdges.offset((f * 3i32 + i) as isize)).unnamed.f = f;
            i += 1
        }
        f += 1
    }
    QuickSortEdges(pEdges, 0i32, iNrTrianglesIn * 3i32 - 1i32, 0i32, uSeed);
    iEntries = iNrTrianglesIn * 3i32;
    iCurStartIndex = 0i32;
    i = 1i32;
    while i < iEntries {
        if (*pEdges.offset(iCurStartIndex as isize)).unnamed.i0
            != (*pEdges.offset(i as isize)).unnamed.i0
        {
            let iL: libc::c_int = iCurStartIndex;
            let iR: libc::c_int = i - 1i32;
            iCurStartIndex = i;
            QuickSortEdges(pEdges, iL, iR, 1i32, uSeed);
        }
        i += 1
    }
    iCurStartIndex = 0i32;
    i = 1i32;
    while i < iEntries {
        if (*pEdges.offset(iCurStartIndex as isize)).unnamed.i0
            != (*pEdges.offset(i as isize)).unnamed.i0
            || (*pEdges.offset(iCurStartIndex as isize)).unnamed.i1
                != (*pEdges.offset(i as isize)).unnamed.i1
        {
            let iL_0: libc::c_int = iCurStartIndex;
            let iR_0: libc::c_int = i - 1i32;
            iCurStartIndex = i;
            QuickSortEdges(pEdges, iL_0, iR_0, 2i32, uSeed);
        }
        i += 1
    }
    i = 0i32;
    while i < iEntries {
        let i0_0: libc::c_int = (*pEdges.offset(i as isize)).unnamed.i0;
        let i1_0: libc::c_int = (*pEdges.offset(i as isize)).unnamed.i1;
        let f_0: libc::c_int = (*pEdges.offset(i as isize)).unnamed.f;
        let mut bUnassigned_A: bool = false;
        let mut i0_A: libc::c_int = 0;
        let mut i1_A: libc::c_int = 0;
        let mut edgenum_A: libc::c_int = 0;
        let mut edgenum_B: libc::c_int = 0i32;
        GetEdge(
            &mut i0_A,
            &mut i1_A,
            &mut edgenum_A,
            &*piTriListIn.offset((f_0 * 3i32) as isize),
            i0_0,
            i1_0,
        );
        bUnassigned_A =
            if (*pTriInfos.offset(f_0 as isize)).FaceNeighbors[edgenum_A as usize] == -1i32 {
                true
            } else {
                false
            };
        if bUnassigned_A {
            let mut j: libc::c_int = i + 1i32;
            let mut t: libc::c_int = 0;
            let mut bNotFound: bool = true;
            while j < iEntries
                && i0_0 == (*pEdges.offset(j as isize)).unnamed.i0
                && i1_0 == (*pEdges.offset(j as isize)).unnamed.i1
                && bNotFound
            {
                let mut bUnassigned_B: bool = false;
                let mut i0_B: libc::c_int = 0;
                let mut i1_B: libc::c_int = 0;
                t = (*pEdges.offset(j as isize)).unnamed.f;
                GetEdge(
                    &mut i1_B,
                    &mut i0_B,
                    &mut edgenum_B,
                    &*piTriListIn.offset((t * 3i32) as isize),
                    (*pEdges.offset(j as isize)).unnamed.i0,
                    (*pEdges.offset(j as isize)).unnamed.i1,
                );
                bUnassigned_B =
                    if (*pTriInfos.offset(t as isize)).FaceNeighbors[edgenum_B as usize] == -1i32 {
                        true
                    } else {
                        false
                    };
                if i0_A == i0_B && i1_A == i1_B && bUnassigned_B {
                    bNotFound = false
                } else {
                    j += 1
                }
            }
            if !bNotFound {
                let mut t_0: libc::c_int = (*pEdges.offset(j as isize)).unnamed.f;
                (*pTriInfos.offset(f_0 as isize)).FaceNeighbors[edgenum_A as usize] = t_0;
                (*pTriInfos.offset(t_0 as isize)).FaceNeighbors[edgenum_B as usize] = f_0
            }
        }
        i += 1
    }
}
unsafe fn GetEdge(
    mut i0_out: *mut libc::c_int,
    mut i1_out: *mut libc::c_int,
    mut edgenum_out: *mut libc::c_int,
    mut indices: *const libc::c_int,
    i0_in: libc::c_int,
    i1_in: libc::c_int,
) {
    *edgenum_out = -1i32;
    if *indices.offset(0isize) == i0_in || *indices.offset(0isize) == i1_in {
        if *indices.offset(1isize) == i0_in || *indices.offset(1isize) == i1_in {
            *edgenum_out.offset(0isize) = 0i32;
            *i0_out.offset(0isize) = *indices.offset(0isize);
            *i1_out.offset(0isize) = *indices.offset(1isize)
        } else {
            *edgenum_out.offset(0isize) = 2i32;
            *i0_out.offset(0isize) = *indices.offset(2isize);
            *i1_out.offset(0isize) = *indices.offset(0isize)
        }
    } else {
        *edgenum_out.offset(0isize) = 1i32;
        *i0_out.offset(0isize) = *indices.offset(1isize);
        *i1_out.offset(0isize) = *indices.offset(2isize)
    };
}
// ///////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////////////////////
unsafe fn QuickSortEdges(
    mut pSortBuffer: *mut SEdge,
    mut iLeft: libc::c_int,
    mut iRight: libc::c_int,
    channel: libc::c_int,
    mut uSeed: libc::c_uint,
) {
    let mut t: libc::c_uint = 0;
    let mut iL: libc::c_int = 0;
    let mut iR: libc::c_int = 0;
    let mut n: libc::c_int = 0;
    let mut index: libc::c_int = 0;
    let mut iMid: libc::c_int = 0;
    // early out
    let mut sTmp: SEdge = SEdge {
        unnamed: unnamed { i0: 0, i1: 0, f: 0 },
    };
    let iElems: libc::c_int = iRight - iLeft + 1i32;
    if iElems < 2i32 {
        return;
    } else {
        if iElems == 2i32 {
            if (*pSortBuffer.offset(iLeft as isize)).array[channel as usize]
                > (*pSortBuffer.offset(iRight as isize)).array[channel as usize]
            {
                sTmp = *pSortBuffer.offset(iLeft as isize);
                *pSortBuffer.offset(iLeft as isize) = *pSortBuffer.offset(iRight as isize);
                *pSortBuffer.offset(iRight as isize) = sTmp
            }
            return;
        }
    }

    // Random
    t = uSeed & 31i32 as libc::c_uint;
    t = uSeed.rotate_left(t) | uSeed.rotate_right((32i32 as libc::c_uint).wrapping_sub(t));
    uSeed = uSeed.wrapping_add(t).wrapping_add(3i32 as libc::c_uint);
    // Random end

    iL = iLeft;
    iR = iRight;
    n = iR - iL + 1i32;
    index = uSeed.wrapping_rem(n as libc::c_uint) as libc::c_int;
    iMid = (*pSortBuffer.offset((index + iL) as isize)).array[channel as usize];
    loop {
        while (*pSortBuffer.offset(iL as isize)).array[channel as usize] < iMid {
            iL += 1
        }
        while (*pSortBuffer.offset(iR as isize)).array[channel as usize] > iMid {
            iR -= 1
        }
        if iL <= iR {
            sTmp = *pSortBuffer.offset(iL as isize);
            *pSortBuffer.offset(iL as isize) = *pSortBuffer.offset(iR as isize);
            *pSortBuffer.offset(iR as isize) = sTmp;
            iL += 1;
            iR -= 1
        }
        if !(iL <= iR) {
            break;
        }
    }
    if iLeft < iR {
        QuickSortEdges(pSortBuffer, iLeft, iR, channel, uSeed);
    }
    if iL < iRight {
        QuickSortEdges(pSortBuffer, iL, iRight, channel, uSeed);
    };
}
unsafe fn BuildNeighborsSlow(
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *const libc::c_int,
    iNrTrianglesIn: libc::c_int,
) {
    let mut f: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    f = 0i32;
    while f < iNrTrianglesIn {
        i = 0i32;
        while i < 3i32 {
            if (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize] == -1i32 {
                let i0_A: libc::c_int = *piTriListIn.offset((f * 3i32 + i) as isize);
                let i1_A: libc::c_int = *piTriListIn
                    .offset((f * 3i32 + if i < 2i32 { i + 1i32 } else { 0i32 }) as isize);
                let mut bFound: bool = false;
                let mut t: libc::c_int = 0i32;
                let mut j: libc::c_int = 0i32;
                while !bFound && t < iNrTrianglesIn {
                    if t != f {
                        j = 0i32;
                        while !bFound && j < 3i32 {
                            let i1_B: libc::c_int = *piTriListIn.offset((t * 3i32 + j) as isize);
                            let i0_B: libc::c_int = *piTriListIn.offset(
                                (t * 3i32 + if j < 2i32 { j + 1i32 } else { 0i32 }) as isize,
                            );
                            if i0_A == i0_B && i1_A == i1_B {
                                bFound = true
                            } else {
                                j += 1
                            }
                        }
                    }
                    if !bFound {
                        t += 1
                    }
                }
                if bFound {
                    (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize] = t;
                    (*pTriInfos.offset(t as isize)).FaceNeighbors[j as usize] = f
                }
            }
            i += 1
        }
        f += 1
    }
}
// returns the texture area times 2
unsafe fn CalcTexArea<I: Geometry>(
    geometry: &mut I,
    mut indices: *const libc::c_int,
) -> libc::c_float {
    let t1 = get_tex_coord(geometry, *indices.offset(0isize) as usize);
    let t2 = get_tex_coord(geometry, *indices.offset(1isize) as usize);
    let t3 = get_tex_coord(geometry, *indices.offset(2isize) as usize);
    let t21x: libc::c_float = t2.x - t1.x;
    let t21y: libc::c_float = t2.y - t1.y;
    let t31x: libc::c_float = t3.x - t1.x;
    let t31y: libc::c_float = t3.y - t1.y;
    let fSignedAreaSTx2: libc::c_float = t21x * t31y - t21y * t31x;
    return if fSignedAreaSTx2 < 0i32 as libc::c_float {
        -fSignedAreaSTx2
    } else {
        fSignedAreaSTx2
    };
}

// degen triangles
unsafe fn DegenPrologue(
    mut pTriInfos: *mut STriInfo,
    mut piTriList_out: *mut libc::c_int,
    iNrTrianglesIn: libc::c_int,
    iTotTris: libc::c_int,
) {
    let mut iNextGoodTriangleSearchIndex: libc::c_int = -1i32;
    let mut bStillFindingGoodOnes: bool = false;
    // locate quads with only one good triangle
    let mut t: libc::c_int = 0i32;
    while t < iTotTris - 1i32 {
        let iFO_a: libc::c_int = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
        let iFO_b: libc::c_int = (*pTriInfos.offset((t + 1i32) as isize)).iOrgFaceNumber;
        if iFO_a == iFO_b {
            let bIsDeg_a: bool = if (*pTriInfos.offset(t as isize)).iFlag & 1i32 != 0i32 {
                true
            } else {
                false
            };
            let bIsDeg_b: bool = if (*pTriInfos.offset((t + 1i32) as isize)).iFlag & 1i32 != 0i32 {
                true
            } else {
                false
            };
            if bIsDeg_a ^ bIsDeg_b != false {
                (*pTriInfos.offset(t as isize)).iFlag |= 2i32;
                (*pTriInfos.offset((t + 1i32) as isize)).iFlag |= 2i32
            }
            t += 2i32
        } else {
            t += 1
        }
    }
    iNextGoodTriangleSearchIndex = 1i32;
    t = 0i32;
    bStillFindingGoodOnes = true;
    while t < iNrTrianglesIn && bStillFindingGoodOnes {
        let bIsGood: bool = if (*pTriInfos.offset(t as isize)).iFlag & 1i32 == 0i32 {
            true
        } else {
            false
        };
        if bIsGood {
            if iNextGoodTriangleSearchIndex < t + 2i32 {
                iNextGoodTriangleSearchIndex = t + 2i32
            }
        } else {
            let mut t0: libc::c_int = 0;
            let mut t1: libc::c_int = 0;
            let mut bJustADegenerate: bool = true;
            while bJustADegenerate && iNextGoodTriangleSearchIndex < iTotTris {
                let bIsGood_0: bool =
                    if (*pTriInfos.offset(iNextGoodTriangleSearchIndex as isize)).iFlag & 1i32
                        == 0i32
                    {
                        true
                    } else {
                        false
                    };
                if bIsGood_0 {
                    bJustADegenerate = false
                } else {
                    iNextGoodTriangleSearchIndex += 1
                }
            }
            t0 = t;
            t1 = iNextGoodTriangleSearchIndex;
            iNextGoodTriangleSearchIndex += 1;
            if !bJustADegenerate {
                let mut i: libc::c_int = 0i32;
                i = 0i32;
                while i < 3i32 {
                    let index: libc::c_int = *piTriList_out.offset((t0 * 3i32 + i) as isize);
                    *piTriList_out.offset((t0 * 3i32 + i) as isize) =
                        *piTriList_out.offset((t1 * 3i32 + i) as isize);
                    *piTriList_out.offset((t1 * 3i32 + i) as isize) = index;
                    i += 1
                }
                let tri_info: STriInfo = *pTriInfos.offset(t0 as isize);
                *pTriInfos.offset(t0 as isize) = *pTriInfos.offset(t1 as isize);
                *pTriInfos.offset(t1 as isize) = tri_info
            } else {
                bStillFindingGoodOnes = false
            }
        }
        if bStillFindingGoodOnes {
            t += 1
        }
    }
}
unsafe fn GenerateSharedVerticesIndexList<I: Geometry>(
    mut piTriList_in_and_out: *mut libc::c_int,
    geometry: &mut I,
    iNrTrianglesIn: usize,
) {
    // Generate bounding box
    let mut piHashTable: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut piHashCount: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut piHashOffsets: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut piHashCount2: *mut libc::c_int = 0 as *mut libc::c_int;
    let mut pTmpVert: *mut STmpVert = 0 as *mut STmpVert;
    let mut i = 0;
    let mut iChannel: libc::c_int = 0i32;
    let mut k = 0;
    let mut e: libc::c_int = 0i32;
    let mut iMaxCount: libc::c_int = 0i32;
    let mut vMin = get_position(geometry, 0);
    let mut vMax = vMin;
    let mut vDim = Vector3::new(0.0, 0.0, 0.0);
    let mut fMin: libc::c_float = 0.;
    let mut fMax: libc::c_float = 0.;
    i = 1;
    while i < iNrTrianglesIn * 3 {
        let index: libc::c_int = *piTriList_in_and_out.offset(i as isize);
        let vP = get_position(geometry, index as usize);
        if vMin.x > vP.x {
            vMin.x = vP.x
        } else if vMax.x < vP.x {
            vMax.x = vP.x
        }
        if vMin.y > vP.y {
            vMin.y = vP.y
        } else if vMax.y < vP.y {
            vMax.y = vP.y
        }
        if vMin.z > vP.z {
            vMin.z = vP.z
        } else if vMax.z < vP.z {
            vMax.z = vP.z
        }
        i += 1
    }
    vDim = vMax - vMin;
    iChannel = 0i32;
    fMin = vMin.x;
    fMax = vMax.x;
    if vDim.y > vDim.x && vDim.y > vDim.z {
        iChannel = 1i32;
        fMin = vMin.y;
        fMax = vMax.y
    } else if vDim.z > vDim.x {
        iChannel = 2i32;
        fMin = vMin.z;
        fMax = vMax.z
    }
    piHashTable = malloc((size_of::<libc::c_int>() * iNrTrianglesIn * 3) as c_ulong) as *mut c_int;
    piHashCount = malloc((size_of::<libc::c_int>() * g_iCells) as c_ulong) as *mut c_int;
    piHashOffsets = malloc((size_of::<libc::c_int>() * g_iCells) as c_ulong) as *mut c_int;
    piHashCount2 = malloc((size_of::<libc::c_int>() * g_iCells) as c_ulong) as *mut c_int;
    if piHashTable.is_null()
        || piHashCount.is_null()
        || piHashOffsets.is_null()
        || piHashCount2.is_null()
    {
        if !piHashTable.is_null() {
            free(piHashTable as *mut libc::c_void);
        }
        if !piHashCount.is_null() {
            free(piHashCount as *mut libc::c_void);
        }
        if !piHashOffsets.is_null() {
            free(piHashOffsets as *mut libc::c_void);
        }
        if !piHashCount2.is_null() {
            free(piHashCount2 as *mut libc::c_void);
        }
        GenerateSharedVerticesIndexListSlow(
            piTriList_in_and_out,
            geometry,
            iNrTrianglesIn as c_int,
        );
        return;
    }
    memset(
        piHashCount as *mut libc::c_void,
        0,
        (size_of::<c_int>() * g_iCells) as c_ulong,
    );
    memset(
        piHashCount2 as *mut libc::c_void,
        0,
        (size_of::<c_int>() * g_iCells) as c_ulong,
    );
    i = 0;
    while i < iNrTrianglesIn * 3 {
        let index_0: libc::c_int = *piTriList_in_and_out.offset(i as isize);
        let vP_0 = get_position(geometry, index_0 as usize);
        let fVal: libc::c_float = if iChannel == 0i32 {
            vP_0.x
        } else if iChannel == 1i32 {
            vP_0.y
        } else {
            vP_0.z
        };
        let iCell = FindGridCell(fMin, fMax, fVal);
        let ref mut fresh5 = *piHashCount.offset(iCell as isize);
        *fresh5 += 1;
        i += 1
    }
    *piHashOffsets.offset(0isize) = 0i32;
    k = 1;
    while k < g_iCells {
        *piHashOffsets.offset(k as isize) =
            *piHashOffsets.offset((k - 1) as isize) + *piHashCount.offset((k - 1) as isize);
        k += 1
    }
    i = 0;
    while i < iNrTrianglesIn * 3 {
        let index_1: libc::c_int = *piTriList_in_and_out.offset(i as isize);
        let vP_1 = get_position(geometry, index_1 as usize);
        let fVal_0: libc::c_float = if iChannel == 0i32 {
            vP_1.x
        } else if iChannel == 1i32 {
            vP_1.y
        } else {
            vP_1.z
        };
        let iCell_0 = FindGridCell(fMin, fMax, fVal_0);
        let mut pTable: *mut libc::c_int = 0 as *mut libc::c_int;
        pTable = &mut *piHashTable.offset(*piHashOffsets.offset(iCell_0 as isize) as isize)
            as *mut libc::c_int;
        *pTable.offset(*piHashCount2.offset(iCell_0 as isize) as isize) = i as c_int;
        let ref mut fresh6 = *piHashCount2.offset(iCell_0 as isize);
        *fresh6 += 1;
        i += 1
    }
    k = 0;
    while k < g_iCells {
        k += 1
    }
    free(piHashCount2 as *mut libc::c_void);
    iMaxCount = *piHashCount.offset(0isize);
    k = 1;
    while k < g_iCells {
        if iMaxCount < *piHashCount.offset(k as isize) {
            iMaxCount = *piHashCount.offset(k as isize)
        }
        k += 1
    }
    pTmpVert = malloc(
        (::std::mem::size_of::<STmpVert>() as libc::c_ulong)
            .wrapping_mul(iMaxCount as libc::c_ulong),
    ) as *mut STmpVert;
    k = 0;
    while k < g_iCells {
        // extract table of cell k and amount of entries in it
        let mut pTable_0: *mut libc::c_int = &mut *piHashTable
            .offset(*piHashOffsets.offset(k as isize) as isize)
            as *mut libc::c_int;
        let iEntries: libc::c_int = *piHashCount.offset(k as isize);
        if !(iEntries < 2i32) {
            if !pTmpVert.is_null() {
                e = 0i32;
                while e < iEntries {
                    let mut i_0: libc::c_int = *pTable_0.offset(e as isize);
                    let vP_2 = get_position(
                        geometry,
                        *piTriList_in_and_out.offset(i_0 as isize) as usize,
                    );
                    (*pTmpVert.offset(e as isize)).vert[0usize] = vP_2.x;
                    (*pTmpVert.offset(e as isize)).vert[1usize] = vP_2.y;
                    (*pTmpVert.offset(e as isize)).vert[2usize] = vP_2.z;
                    (*pTmpVert.offset(e as isize)).index = i_0;
                    e += 1
                }
                MergeVertsFast(
                    piTriList_in_and_out,
                    pTmpVert,
                    geometry,
                    0i32,
                    iEntries - 1i32,
                );
            } else {
                MergeVertsSlow(
                    piTriList_in_and_out,
                    geometry,
                    pTable_0 as *const libc::c_int,
                    iEntries,
                );
            }
        }
        k += 1
    }
    if !pTmpVert.is_null() {
        free(pTmpVert as *mut libc::c_void);
    }
    free(piHashTable as *mut libc::c_void);
    free(piHashCount as *mut libc::c_void);
    free(piHashOffsets as *mut libc::c_void);
}
unsafe fn MergeVertsSlow<I: Geometry>(
    mut piTriList_in_and_out: *mut libc::c_int,
    geometry: &mut I,
    mut pTable: *const libc::c_int,
    iEntries: libc::c_int,
) {
    // this can be optimized further using a tree structure or more hashing.
    let mut e: libc::c_int = 0i32;
    e = 0i32;
    while e < iEntries {
        let mut i: libc::c_int = *pTable.offset(e as isize);
        let index: libc::c_int = *piTriList_in_and_out.offset(i as isize);
        let vP = get_position(geometry, index as usize);
        let vN = get_normal(geometry, index as usize);
        let vT = get_tex_coord(geometry, index as usize);
        let mut bNotFound: bool = true;
        let mut e2: libc::c_int = 0i32;
        let mut i2rec: libc::c_int = -1i32;
        while e2 < e && bNotFound {
            let i2: libc::c_int = *pTable.offset(e2 as isize);
            let index2: libc::c_int = *piTriList_in_and_out.offset(i2 as isize);
            let vP2 = get_position(geometry, index2 as usize);
            let vN2 = get_normal(geometry, index2 as usize);
            let vT2 = get_tex_coord(geometry, index2 as usize);
            i2rec = i2;
            if vP == vP2 && vN == vN2 && vT == vT2 {
                bNotFound = false
            } else {
                e2 += 1
            }
        }
        if !bNotFound {
            *piTriList_in_and_out.offset(i as isize) = *piTriList_in_and_out.offset(i2rec as isize)
        }
        e += 1
    }
}
unsafe fn MergeVertsFast<I: Geometry>(
    mut piTriList_in_and_out: *mut libc::c_int,
    mut pTmpVert: *mut STmpVert,
    geometry: &mut I,
    iL_in: libc::c_int,
    iR_in: libc::c_int,
) {
    // make bbox
    let mut c: libc::c_int = 0i32;
    let mut l: libc::c_int = 0i32;
    let mut channel: libc::c_int = 0i32;
    let mut fvMin: [libc::c_float; 3] = [0.; 3];
    let mut fvMax: [libc::c_float; 3] = [0.; 3];
    let mut dx: libc::c_float = 0i32 as libc::c_float;
    let mut dy: libc::c_float = 0i32 as libc::c_float;
    let mut dz: libc::c_float = 0i32 as libc::c_float;
    let mut fSep: libc::c_float = 0i32 as libc::c_float;
    c = 0i32;
    while c < 3i32 {
        fvMin[c as usize] = (*pTmpVert.offset(iL_in as isize)).vert[c as usize];
        fvMax[c as usize] = fvMin[c as usize];
        c += 1
    }
    l = iL_in + 1i32;
    while l <= iR_in {
        c = 0i32;
        while c < 3i32 {
            if fvMin[c as usize] > (*pTmpVert.offset(l as isize)).vert[c as usize] {
                fvMin[c as usize] = (*pTmpVert.offset(l as isize)).vert[c as usize]
            } else if fvMax[c as usize] < (*pTmpVert.offset(l as isize)).vert[c as usize] {
                fvMax[c as usize] = (*pTmpVert.offset(l as isize)).vert[c as usize]
            }
            c += 1
        }
        l += 1
    }
    dx = fvMax[0usize] - fvMin[0usize];
    dy = fvMax[1usize] - fvMin[1usize];
    dz = fvMax[2usize] - fvMin[2usize];
    channel = 0i32;
    if dy > dx && dy > dz {
        channel = 1i32
    } else if dz > dx {
        channel = 2i32
    }
    fSep = 0.5f32 * (fvMax[channel as usize] + fvMin[channel as usize]);
    if fSep >= fvMax[channel as usize] || fSep <= fvMin[channel as usize] {
        l = iL_in;
        while l <= iR_in {
            let mut i: libc::c_int = (*pTmpVert.offset(l as isize)).index;
            let index: libc::c_int = *piTriList_in_and_out.offset(i as isize);
            let vP = get_position(geometry, index as usize);
            let vN = get_normal(geometry, index as usize);
            let vT = get_tex_coord(geometry, index as usize);
            let mut bNotFound: bool = true;
            let mut l2: libc::c_int = iL_in;
            let mut i2rec: libc::c_int = -1i32;
            while l2 < l && bNotFound {
                let i2: libc::c_int = (*pTmpVert.offset(l2 as isize)).index;
                let index2: libc::c_int = *piTriList_in_and_out.offset(i2 as isize);
                let vP2 = get_position(geometry, index2 as usize);
                let vN2 = get_normal(geometry, index2 as usize);
                let vT2 = get_tex_coord(geometry, index2 as usize);
                i2rec = i2;
                if vP.x == vP2.x
                    && vP.y == vP2.y
                    && vP.z == vP2.z
                    && vN.x == vN2.x
                    && vN.y == vN2.y
                    && vN.z == vN2.z
                    && vT.x == vT2.x
                    && vT.y == vT2.y
                    && vT.z == vT2.z
                {
                    bNotFound = false
                } else {
                    l2 += 1
                }
            }
            if !bNotFound {
                *piTriList_in_and_out.offset(i as isize) =
                    *piTriList_in_and_out.offset(i2rec as isize)
            }
            l += 1
        }
    } else {
        let mut iL: libc::c_int = iL_in;
        let mut iR: libc::c_int = iR_in;
        while iL < iR {
            let mut bReadyLeftSwap: bool = false;
            let mut bReadyRightSwap: bool = false;
            while !bReadyLeftSwap && iL < iR {
                bReadyLeftSwap = !((*pTmpVert.offset(iL as isize)).vert[channel as usize] < fSep);
                if !bReadyLeftSwap {
                    iL += 1
                }
            }
            while !bReadyRightSwap && iL < iR {
                bReadyRightSwap = (*pTmpVert.offset(iR as isize)).vert[channel as usize] < fSep;
                if !bReadyRightSwap {
                    iR -= 1
                }
            }
            if bReadyLeftSwap && bReadyRightSwap {
                let sTmp: STmpVert = *pTmpVert.offset(iL as isize);
                *pTmpVert.offset(iL as isize) = *pTmpVert.offset(iR as isize);
                *pTmpVert.offset(iR as isize) = sTmp;
                iL += 1;
                iR -= 1
            }
        }
        if iL == iR {
            let bReadyRightSwap_0: bool =
                (*pTmpVert.offset(iR as isize)).vert[channel as usize] < fSep;
            if bReadyRightSwap_0 {
                iL += 1
            } else {
                iR -= 1
            }
        }
        if iL_in < iR {
            MergeVertsFast(piTriList_in_and_out, pTmpVert, geometry, iL_in, iR);
        }
        if iL < iR_in {
            MergeVertsFast(piTriList_in_and_out, pTmpVert, geometry, iL, iR_in);
        }
    };
}

const g_iCells: usize = 2048;

// it is IMPORTANT that this function is called to evaluate the hash since
// inlining could potentially reorder instructions and generate different
// results for the same effective input value fVal.
#[inline(never)]
unsafe fn FindGridCell(
    fMin: libc::c_float,
    fMax: libc::c_float,
    fVal: libc::c_float,
) -> usize {
    let fIndex = g_iCells as f32 * ((fVal - fMin) / (fMax - fMin));
    let iIndex = fIndex as isize;
    return if iIndex < g_iCells as isize {
        if iIndex >= 0 {
            iIndex as usize
        } else {
            0
        }
    } else {
        g_iCells - 1
    };
}

unsafe fn GenerateSharedVerticesIndexListSlow<I: Geometry>(
    mut piTriList_in_and_out: *mut libc::c_int,
    geometry: &mut I,
    iNrTrianglesIn: libc::c_int,
) {
    let mut iNumUniqueVerts: libc::c_int = 0i32;
    let mut t: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    t = 0i32;
    while t < iNrTrianglesIn {
        i = 0i32;
        while i < 3i32 {
            let offs: libc::c_int = t * 3i32 + i;
            let index: libc::c_int = *piTriList_in_and_out.offset(offs as isize);
            let vP = get_position(geometry, index as usize);
            let vN = get_normal(geometry, index as usize);
            let vT = get_tex_coord(geometry, index as usize);
            let mut bFound: bool = false;
            let mut t2: libc::c_int = 0i32;
            let mut index2rec: libc::c_int = -1i32;
            while !bFound && t2 <= t {
                let mut j: libc::c_int = 0i32;
                while !bFound && j < 3i32 {
                    let index2: libc::c_int =
                        *piTriList_in_and_out.offset((t2 * 3i32 + j) as isize);
                    let vP2 = get_position(geometry, index2 as usize);
                    let vN2 = get_normal(geometry, index2 as usize);
                    let vT2 = get_tex_coord(geometry, index2 as usize);
                    if vP == vP2 && vN == vN2 && vT == vT2 {
                        bFound = true
                    } else {
                        j += 1
                    }
                }
                if !bFound {
                    t2 += 1
                }
            }
            if index2rec == index {
                iNumUniqueVerts += 1
            }
            *piTriList_in_and_out.offset(offs as isize) = index2rec;
            i += 1
        }
        t += 1
    }
}
unsafe fn GenerateInitialVerticesIndexList<I: Geometry>(
    mut pTriInfos: *mut STriInfo,
    mut piTriList_out: *mut libc::c_int,
    geometry: &mut I,
    iNrTrianglesIn: libc::c_int,
) -> usize {
    let mut iTSpacesOffs: usize = 0;
    let mut f = 0;
    let mut t: libc::c_int = 0i32;
    let mut iDstTriIndex: libc::c_int = 0i32;
    f = 0;
    while f < geometry.get_num_faces() {
        let verts = geometry.get_num_vertices_of_face(f);
        if !(verts != 3 && verts != 4) {
            (*pTriInfos.offset(iDstTriIndex as isize)).iOrgFaceNumber = f as c_int;
            (*pTriInfos.offset(iDstTriIndex as isize)).iTSpacesOffs = iTSpacesOffs as c_int;
            if verts == 3 {
                let mut pVerts: *mut libc::c_uchar = (*pTriInfos.offset(iDstTriIndex as isize))
                    .vert_num
                    .as_mut_ptr();
                *pVerts.offset(0isize) = 0i32 as libc::c_uchar;
                *pVerts.offset(1isize) = 1i32 as libc::c_uchar;
                *pVerts.offset(2isize) = 2i32 as libc::c_uchar;
                *piTriList_out.offset((iDstTriIndex * 3i32 + 0i32) as isize) =
                    face_vert_to_index(f, 0) as c_int;
                *piTriList_out.offset((iDstTriIndex * 3i32 + 1i32) as isize) =
                    face_vert_to_index(f, 1) as c_int;
                *piTriList_out.offset((iDstTriIndex * 3i32 + 2i32) as isize) =
                    face_vert_to_index(f, 2) as c_int;
                iDstTriIndex += 1
            } else {
                (*pTriInfos.offset((iDstTriIndex + 1i32) as isize)).iOrgFaceNumber = f as c_int;
                (*pTriInfos.offset((iDstTriIndex + 1i32) as isize)).iTSpacesOffs =
                    iTSpacesOffs as c_int;
                let i0 = face_vert_to_index(f, 0);
                let i1 = face_vert_to_index(f, 1);
                let i2 = face_vert_to_index(f, 2);
                let i3 = face_vert_to_index(f, 3);
                let T0 = get_tex_coord(geometry, i0);
                let T1 = get_tex_coord(geometry, i1);
                let T2 = get_tex_coord(geometry, i2);
                let T3 = get_tex_coord(geometry, i3);
                let distSQ_02: libc::c_float = (T2 - T0).magnitude_squared();
                let distSQ_13: libc::c_float = (T3 - T1).magnitude_squared();
                let mut bQuadDiagIs_02: bool = false;
                if distSQ_02 < distSQ_13 {
                    bQuadDiagIs_02 = true
                } else if distSQ_13 < distSQ_02 {
                    bQuadDiagIs_02 = false
                } else {
                    let P0 = get_position(geometry, i0);
                    let P1 = get_position(geometry, i1);
                    let P2 = get_position(geometry, i2);
                    let P3 = get_position(geometry, i3);
                    let distSQ_02_0: libc::c_float = (P2 - P0).magnitude_squared();
                    let distSQ_13_0: libc::c_float = (P3 - P1).magnitude_squared();
                    bQuadDiagIs_02 = if distSQ_13_0 < distSQ_02_0 {
                        false
                    } else {
                        true
                    }
                }
                if bQuadDiagIs_02 {
                    let mut pVerts_A: *mut libc::c_uchar = (*pTriInfos
                        .offset(iDstTriIndex as isize))
                    .vert_num
                    .as_mut_ptr();
                    *pVerts_A.offset(0isize) = 0i32 as libc::c_uchar;
                    *pVerts_A.offset(1isize) = 1i32 as libc::c_uchar;
                    *pVerts_A.offset(2isize) = 2i32 as libc::c_uchar;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 0i32) as isize) = i0 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 1i32) as isize) = i1 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 2i32) as isize) = i2 as c_int;
                    iDstTriIndex += 1;
                    let mut pVerts_B: *mut libc::c_uchar = (*pTriInfos
                        .offset(iDstTriIndex as isize))
                    .vert_num
                    .as_mut_ptr();
                    *pVerts_B.offset(0isize) = 0i32 as libc::c_uchar;
                    *pVerts_B.offset(1isize) = 2i32 as libc::c_uchar;
                    *pVerts_B.offset(2isize) = 3i32 as libc::c_uchar;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 0i32) as isize) = i0 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 1i32) as isize) = i2 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 2i32) as isize) = i3 as c_int;
                    iDstTriIndex += 1
                } else {
                    let mut pVerts_A_0: *mut libc::c_uchar = (*pTriInfos
                        .offset(iDstTriIndex as isize))
                    .vert_num
                    .as_mut_ptr();
                    *pVerts_A_0.offset(0isize) = 0i32 as libc::c_uchar;
                    *pVerts_A_0.offset(1isize) = 1i32 as libc::c_uchar;
                    *pVerts_A_0.offset(2isize) = 3i32 as libc::c_uchar;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 0i32) as isize) = i0 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 1i32) as isize) = i1 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 2i32) as isize) = i3 as c_int;
                    iDstTriIndex += 1;
                    let mut pVerts_B_0: *mut libc::c_uchar = (*pTriInfos
                        .offset(iDstTriIndex as isize))
                    .vert_num
                    .as_mut_ptr();
                    *pVerts_B_0.offset(0isize) = 1i32 as libc::c_uchar;
                    *pVerts_B_0.offset(1isize) = 2i32 as libc::c_uchar;
                    *pVerts_B_0.offset(2isize) = 3i32 as libc::c_uchar;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 0i32) as isize) = i1 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 1i32) as isize) = i2 as c_int;
                    *piTriList_out.offset((iDstTriIndex * 3i32 + 2i32) as isize) = i3 as c_int;
                    iDstTriIndex += 1
                }
            }
            iTSpacesOffs += verts
        }
        f += 1
    }
    t = 0i32;
    while t < iNrTrianglesIn {
        (*pTriInfos.offset(t as isize)).iFlag = 0i32;
        t += 1
    }
    return iTSpacesOffs;
}
