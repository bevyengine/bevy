//! Everything in this module is pending to be refactored, turned into idiomatic-rust, and moved to
//! other modules.

//! The contents of this file are a combination of transpilation and human
//! modification to Morten S. Mikkelsen's original tangent space algorithm
//! implementation written in C. The original source code can be found at
//! <https://archive.blender.org/wiki/index.php/Dev:Shading/Tangent_Space_Normal_Maps>
//! and includes the following licence:
//!
//! Copyright (C) 2011 by Morten S. Mikkelsen
//!
//! This software is provided 'as-is', without any express or implied
//! warranty.  In no event will the authors be held liable for any damages
//! arising from the use of this software.
//!
//! Permission is granted to anyone to use this software for any purpose,
//! including commercial applications, and to alter it and redistribute it
//! freely, subject to the following restrictions:
//!
//! 1. The origin of this software must not be misrepresented; you must not
//! claim that you wrote the original software. If you use this software
//! in a product, an acknowledgment in the product documentation would be
//! appreciated but is not required.
//!
//! 2. Altered source versions must be plainly marked as such, and must not be
//! misrepresented as being the original software.
//!
//! 3. This notice may not be removed or altered from any source distribution.
//!
// Comments starting with `C:` are copied as-is from the original
// Note that some comments may originate from the original but not be marked as such

#![allow(
    clippy::all,
    clippy::doc_markdown,
    clippy::redundant_else,
    clippy::match_same_arms,
    clippy::semicolon_if_nothing_returned,
    clippy::explicit_iter_loop,
    clippy::map_flatten,
    dead_code,
    non_camel_case_types,
    non_snake_case,
    unused_mut
)]

use std::{collections::BTreeMap, ptr::null_mut};

use bitflags::bitflags;
use glam::Vec3;

use crate::{
    face_vert_to_index, get_normal, get_position, get_tex_coord, ordered_vec::FiniteVec3, FaceKind,
    Geometry,
};

#[derive(Copy, Clone)]
pub struct STSpace {
    pub vOs: Vec3,
    pub fMagS: f32,
    pub vOt: Vec3,
    pub fMagT: f32,
    pub iCounter: i32,
    pub bOrient: bool,
}

impl STSpace {
    pub fn zero() -> Self {
        Self {
            vOs: Default::default(),
            fMagS: 0.0,
            vOt: Default::default(),
            fMagT: 0.0,
            iCounter: 0,
            bOrient: false,
        }
    }
}

bitflags! {
    pub struct TriangleFlags: u8 {
        /// This triangle has multiple vertices at the same point
        const DEGENERATE = 1;
        /// This triangle is part of a quad where one (but not both)
        /// of its triangles are degenerate (i.e. exactly two of the quad's
        /// vertices are in the same location)
        const QUAD_ONE_DEGENERATE_TRI = 2;
        const GROUP_WITH_ANY = 4;
        const ORIENT_PRESERVING = 8;
    }
}

#[derive(Copy, Clone)]
pub struct STriInfo {
    /// Indices of neighbouring triangles across this triangle's edges
    pub FaceNeighbors: [i32; 3],
    /// The group each vertex belongs to. TODO: Convert to index
    pub AssignedGroup: [*mut SGroup; 3],
    pub vOs: Vec3,
    pub vOt: Vec3,
    pub fMagS: f32,
    pub fMagT: f32,
    /// The face in the user's module this triangle comes from
    pub iOrgFaceNumber: i32,
    // Flags set for this triangle
    pub iFlag: TriangleFlags,
    pub iTSpacesOffs: i32,
    // The vertices of the face 'iOrgFaceNumber' this triangle covers
    // This has only a limited set of valid values - as required for quads.
    // - TODO: Convert to a repr(u8) enum to compress.
    // In theory, this could be compressed inside TriangleFlags too.
    pub vert_num: [u8; 3],
}

impl STriInfo {
    fn zero() -> Self {
        Self {
            FaceNeighbors: [0, 0, 0],
            AssignedGroup: [null_mut(), null_mut(), null_mut()],
            vOs: Default::default(),
            vOt: Default::default(),
            fMagS: 0.0,
            fMagT: 0.0,
            iOrgFaceNumber: 0,
            iFlag: TriangleFlags::empty(),
            iTSpacesOffs: 0,
            vert_num: [0, 0, 0],
        }
    }
}

#[derive(Copy, Clone)]
pub struct SGroup {
    pub iNrFaces: i32,
    pub pFaceIndices: *mut i32,
    pub iVertexRepresentitive: i32,
    pub bOrientPreservering: bool,
}

impl SGroup {
    fn zero() -> Self {
        Self {
            iNrFaces: 0,
            pFaceIndices: null_mut(),
            iVertexRepresentitive: 0,
            bOrientPreservering: false,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SSubGroup {
    pub iNrFaces: i32,
    pub pTriMembers: Vec<i32>,
}

impl SSubGroup {
    fn zero() -> Self {
        Self {
            iNrFaces: 0,
            pTriMembers: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SEdge {
    // The first vertex's (global) index. This is the minimum index
    pub i0: i32,
    // The second vertex's (global) index
    pub i1: i32,
    // The face this edge is associated with
    pub f: i32,
}

/// Stores a map of 'internal' triangle vertices to real 'faces' and vertices
/// This is used to deduplicate vertices with identical faces
struct TriangleMap {
    /// Packed face/vertex index of each triangle
    /// Note that this is an index to the first vertex
    /// with the given properties, rather than necessarily
    /// (Not impressed with this data layout)
    triangles: Vec<[u32; 3]>,
}

// Entry point
pub unsafe fn genTangSpace(geometry: &mut impl Geometry, fAngularThreshold: f32) -> bool {
    // TODO: Accept in radians by default here?
    let fThresCos = (fAngularThreshold.to_radians()).cos();

    let iNrFaces = geometry.num_faces();
    let mut iNrTrianglesIn = 0;
    for f in 0..iNrFaces {
        let verts = geometry.num_vertices_of_face(f);
        match verts {
            FaceKind::Triangle => iNrTrianglesIn += 1,
            FaceKind::Quad => iNrTrianglesIn += 2,
        }
    }

    if iNrTrianglesIn <= 0 {
        // Easier if we can assume there's at least one face later
        // No tangents need to be generated
        return false;
    }
    let iNrTrianglesIn = iNrTrianglesIn;
    let mut piTriListIn = vec![0i32; 3 * iNrTrianglesIn];
    let mut pTriInfos = vec![STriInfo::zero(); iNrTrianglesIn];

    // C: Make an initial triangle --> face index list
    // This also handles quads
    // TODO: Make this return triangle_info and tri_face_map
    // probably in a single structure.
    let iNrTSPaces = GenerateInitialVerticesIndexList(
        &mut pTriInfos,
        &mut piTriListIn,
        geometry,
        iNrTrianglesIn,
    );
    // C: Make a welded index list of identical positions and attributes (pos, norm, texc)
    GenerateSharedVerticesIndexList(&mut piTriListIn, geometry);

    let iTotTris = iNrTrianglesIn;
    let mut iDegenTriangles = 0;
    // C: Mark all degenerate triangles
    for t in 0..(iTotTris as usize) {
        let i0 = piTriListIn[t * 3 + 0];
        let i1 = piTriListIn[t * 3 + 1];
        let i2 = piTriListIn[t * 3 + 2];
        let p0 = get_position(geometry, i0 as usize);
        let p1 = get_position(geometry, i1 as usize);
        let p2 = get_position(geometry, i2 as usize);
        if p0 == p1 || p0 == p2 || p1 == p2 {
            pTriInfos[t].iFlag.insert(TriangleFlags::DEGENERATE);
            iDegenTriangles += 1
        }
    }
    let iNrTrianglesIn = iTotTris - iDegenTriangles;
    // C: Mark all triangle pairs that belong to a quad with only one
    // C: good triangle. These need special treatment in DegenEpilogue().
    // C: Additionally, move all good triangles to the start of
    // C: pTriInfos[] and piTriListIn[] without changing order and
    // C: put the degenerate triangles last.
    // Note: A quad can have degenerate triangles if two vertices are in the same location
    DegenPrologue(
        pTriInfos.as_mut_ptr(),
        piTriListIn.as_mut_ptr(),
        iNrTrianglesIn as i32,
        iTotTris as i32,
    );
    // C: Evaluate triangle level attributes and neighbor list
    InitTriInfo(
        pTriInfos.as_mut_ptr(),
        piTriListIn.as_ptr(),
        geometry,
        iNrTrianglesIn,
    );
    //C: Based on the 4 rules, identify groups based on connectivity
    let iNrMaxGroups = iNrTrianglesIn * 3;

    let mut pGroups = vec![SGroup::zero(); iNrMaxGroups];
    let mut piGroupTrianglesBuffer = vec![0; iNrTrianglesIn * 3];

    let iNrActiveGroups = Build4RuleGroups(
        pTriInfos.as_mut_ptr(),
        pGroups.as_mut_ptr(),
        piGroupTrianglesBuffer.as_mut_ptr(),
        piTriListIn.as_ptr(),
        iNrTrianglesIn as i32,
    );

    let mut psTspace = vec![
        STSpace {
            vOs: Vec3::new(1.0, 0.0, 0.0),
            fMagS: 1.0,
            vOt: Vec3::new(0.0, 1.0, 0.0),
            fMagT: 1.0,
            ..STSpace::zero()
        };
        iNrTSPaces
    ];

    let bRes = GenerateTSpaces(
        &mut psTspace,
        pTriInfos.as_ptr(),
        pGroups.as_ptr(),
        iNrActiveGroups,
        piTriListIn.as_ptr(),
        fThresCos,
        geometry,
    );
    if !bRes {
        return false;
    }
    DegenEpilogue(
        psTspace.as_mut_ptr(),
        pTriInfos.as_mut_ptr(),
        piTriListIn.as_mut_ptr(),
        geometry,
        iNrTrianglesIn as i32,
        iTotTris as i32,
    );
    let mut index = 0;
    for f in 0..iNrFaces {
        let verts_0 = geometry.num_vertices_of_face(f);
        for i in 0..verts_0.num_vertices() {
            let mut pTSpace: *const STSpace = &mut psTspace[index] as *mut STSpace;
            let mut tang = Vec3::new((*pTSpace).vOs.x, (*pTSpace).vOs.y, (*pTSpace).vOs.z);
            let mut bitang = Vec3::new((*pTSpace).vOt.x, (*pTSpace).vOt.y, (*pTSpace).vOt.z);
            geometry.set_tangent(
                tang.into(),
                bitang.into(),
                (*pTSpace).fMagS,
                (*pTSpace).fMagT,
                (*pTSpace).bOrient,
                f,
                i,
            );
            index += 1;
        }
    }

    return true;
}

unsafe fn DegenEpilogue(
    mut psTspace: *mut STSpace,
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *mut i32,
    geometry: &impl Geometry,
    iNrTrianglesIn: i32,
    iTotTris: i32,
) {
    // For all degenerate triangles
    for t in iNrTrianglesIn..iTotTris {
        let bSkip: bool = (*pTriInfos.offset(t as isize))
            .iFlag
            .contains(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
        if !bSkip {
            for i in 0..3i32 {
                // For all vertices on that triangle
                let index1: i32 = *piTriListIn.offset((t * 3i32 + i) as isize);
                for j in 0..(3i32 * iNrTrianglesIn) {
                    let index2: i32 = *piTriListIn.offset(j as isize);
                    // If the vertex properties are the same as another non-degenerate vertex
                    if index1 == index2 {
                        let iTri: i32 = j / 3i32;
                        let iVert: i32 = j % 3i32;
                        let iSrcVert: i32 =
                            (*pTriInfos.offset(iTri as isize)).vert_num[iVert as usize] as i32;
                        let iSrcOffs: i32 = (*pTriInfos.offset(iTri as isize)).iTSpacesOffs;
                        let iDstVert: i32 =
                            (*pTriInfos.offset(t as isize)).vert_num[i as usize] as i32;
                        let iDstOffs: i32 = (*pTriInfos.offset(t as isize)).iTSpacesOffs;
                        // Set the tangent space of this vertex to the tangent space of that vertex
                        // TODO: This is absurd - doing a linear search through all vertices for each
                        // degenerate triangle?
                        *psTspace.offset((iDstOffs + iDstVert) as isize) =
                            *psTspace.offset((iSrcOffs + iSrcVert) as isize);
                        break;
                    }
                }
            }
        }
    }
    for t in 0..iNrTrianglesIn {
        // Handle quads with a single degenerate triangle by
        if (*pTriInfos.offset(t as isize))
            .iFlag
            .contains(TriangleFlags::QUAD_ONE_DEGENERATE_TRI)
        {
            let mut pV: *mut u8 = (*pTriInfos.offset(t as isize)).vert_num.as_mut_ptr();
            let mut iFlag: i32 = 1i32 << *pV.offset(0isize) as i32
                | 1i32 << *pV.offset(1isize) as i32
                | 1i32 << *pV.offset(2isize) as i32;
            let mut iMissingIndex: i32 = 0i32;
            if iFlag & 2i32 == 0i32 {
                iMissingIndex = 1i32
            } else if iFlag & 4i32 == 0i32 {
                iMissingIndex = 2i32
            } else if iFlag & 8i32 == 0i32 {
                iMissingIndex = 3i32
            }
            let iOrgF = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
            let vDstP = get_position(
                geometry,
                face_vert_to_index(iOrgF as usize, iMissingIndex as usize),
            );

            for i_0 in 0..3i32 {
                let iVert_0: i32 = *pV.offset(i_0 as isize) as i32;
                let vSrcP = get_position(
                    geometry,
                    face_vert_to_index(iOrgF as usize, iVert_0 as usize),
                );
                if vSrcP == vDstP {
                    let iOffs: i32 = (*pTriInfos.offset(t as isize)).iTSpacesOffs;
                    *psTspace.offset((iOffs + iMissingIndex) as isize) =
                        *psTspace.offset((iOffs + iVert_0) as isize);
                    break;
                }
            }
        }
    }
}

unsafe fn GenerateTSpaces(
    psTspace: &mut [STSpace],
    mut pTriInfos: *const STriInfo,
    mut pGroups: *const SGroup,
    iNrActiveGroups: i32,
    mut piTriListIn: *const i32,
    fThresCos: f32,
    geometry: &impl Geometry,
) -> bool {
    let mut iMaxNrFaces: usize = 0;
    for g in 0..iNrActiveGroups {
        if iMaxNrFaces < (*pGroups.offset(g as isize)).iNrFaces as usize {
            iMaxNrFaces = (*pGroups.offset(g as isize)).iNrFaces as usize
        }
    }
    if iMaxNrFaces == 0 {
        return true;
    }

    let mut pSubGroupTspace = vec![STSpace::zero(); iMaxNrFaces];
    let mut pUniSubGroups = vec![SSubGroup::zero(); iMaxNrFaces];
    let mut pTmpMembers = vec![0i32; iMaxNrFaces];

    for g in 0..iNrActiveGroups {
        let mut pGroup: *const SGroup = &*pGroups.offset(g as isize) as *const SGroup;
        let mut iUniqueSubGroups = 0;

        for i in 0..(*pGroup).iNrFaces {
            let f: i32 = *(*pGroup).pFaceIndices.offset(i as isize);
            let mut tmp_group: SSubGroup = SSubGroup {
                iNrFaces: 0,
                pTriMembers: Vec::new(),
            };
            let index = if (*pTriInfos.offset(f as isize)).AssignedGroup[0usize]
                == pGroup as *mut SGroup
            {
                0i32
            } else if (*pTriInfos.offset(f as isize)).AssignedGroup[1usize] == pGroup as *mut SGroup
            {
                1i32
            } else if (*pTriInfos.offset(f as isize)).AssignedGroup[2usize] == pGroup as *mut SGroup
            {
                2i32
            } else {
                panic!()
            };
            let iVertIndex = *piTriListIn.offset((f * 3i32 + index) as isize);
            assert!(iVertIndex == (*pGroup).iVertexRepresentitive);
            let n = get_normal(geometry, iVertIndex as usize);
            let mut vOs = (*pTriInfos.offset(f as isize)).vOs
                - (n.dot((*pTriInfos.offset(f as isize)).vOs) * n);
            let mut vOt = (*pTriInfos.offset(f as isize)).vOt
                - (n.dot((*pTriInfos.offset(f as isize)).vOt) * n);
            vOs = vOs.normalize_or_zero();
            vOt = vOt.normalize_or_zero();

            let iOF_1 = (*pTriInfos.offset(f as isize)).iOrgFaceNumber;
            let mut iMembers = 0;

            for j in 0..(*pGroup).iNrFaces {
                let t: i32 = *(*pGroup).pFaceIndices.offset(j as isize);
                let iOF_2: i32 = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
                let mut vOs2 = (*pTriInfos.offset(t as isize)).vOs
                    - (n.dot((*pTriInfos.offset(t as isize)).vOs) * n);
                let mut vOt2 = (*pTriInfos.offset(t as isize)).vOt
                    - (n.dot((*pTriInfos.offset(t as isize)).vOt) * n);
                vOs2 = vOs2.normalize_or_zero();
                vOt2 = vOt2.normalize_or_zero();

                let bAny: bool = ((*pTriInfos.offset(f as isize)).iFlag
                    | (*pTriInfos.offset(t as isize)).iFlag)
                    .contains(TriangleFlags::GROUP_WITH_ANY);
                let bSameOrgFace: bool = iOF_1 == iOF_2;
                let fCosS: f32 = vOs.dot(vOs2);
                let fCosT: f32 = vOt.dot(vOt2);
                debug_assert!(f != t || bSameOrgFace); // sanity check
                if bAny || bSameOrgFace || fCosS > fThresCos && fCosT > fThresCos {
                    let fresh0 = iMembers;
                    iMembers = iMembers + 1;
                    pTmpMembers[fresh0] = t
                }
            }
            if iMembers > 1 {
                pTmpMembers[0..(iMembers - 1)].sort();
            }
            tmp_group.iNrFaces = iMembers as i32;
            tmp_group.pTriMembers = pTmpMembers.clone();

            let mut found = None;
            for l in 0..iUniqueSubGroups {
                if tmp_group == pUniSubGroups[l] {
                    found = Some(l);
                    break;
                }
            }
            let idx;
            if let Some(it) = found {
                idx = it;
            } else {
                idx = iUniqueSubGroups;
                // C: if no match was found we allocate a new subgroup
                pUniSubGroups[iUniqueSubGroups].iNrFaces = iMembers as i32;
                pUniSubGroups[iUniqueSubGroups].pTriMembers = tmp_group.pTriMembers.clone();

                pSubGroupTspace[iUniqueSubGroups] = EvalTspace(
                    tmp_group.pTriMembers.as_mut_ptr(),
                    iMembers as i32,
                    piTriListIn,
                    pTriInfos,
                    geometry,
                    (*pGroup).iVertexRepresentitive,
                );
                iUniqueSubGroups += 1
            }
            let iOffs = (*pTriInfos.offset(f as isize)).iTSpacesOffs as usize;
            let iVert = (*pTriInfos.offset(f as isize)).vert_num[index as usize] as usize;
            let mut pTS_out = &mut psTspace[iOffs + iVert];
            assert!(pTS_out.iCounter < 2);
            debug_assert!(
                (*pGroup).bOrientPreservering
                    == (*pTriInfos.offset(f as isize))
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING)
            );
            if (*pTS_out).iCounter == 1i32 {
                *pTS_out = AvgTSpace(pTS_out, &mut pSubGroupTspace[idx]);
                (*pTS_out).iCounter = 2i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering
            } else {
                debug_assert!(pTS_out.iCounter == 0);
                *pTS_out = pSubGroupTspace[idx];
                (*pTS_out).iCounter = 1i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering
            }
        }
    }
    return true;
}
unsafe fn AvgTSpace(mut pTS0: *const STSpace, mut pTS1: *const STSpace) -> STSpace {
    let mut ts_res: STSpace = STSpace {
        vOs: Vec3::new(0.0, 0.0, 0.0),
        fMagS: 0.,
        vOt: Vec3::new(0.0, 0.0, 0.0),
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
        ts_res.vOs = ts_res.vOs.normalize_or_zero();
        ts_res.vOt = ts_res.vOt.normalize_or_zero();
    }
    return ts_res;
}

unsafe fn EvalTspace(
    mut face_indices: *mut i32,
    iFaces: i32,
    mut piTriListIn: *const i32,
    mut pTriInfos: *const STriInfo,
    geometry: &impl Geometry,
    iVertexRepresentitive: i32,
) -> STSpace {
    let mut res: STSpace = STSpace::zero();
    let mut fAngleSum: f32 = 0i32 as f32;

    for face in 0..iFaces {
        let f: i32 = *face_indices.offset(face as isize);
        if !(*pTriInfos.offset(f as isize))
            .iFlag
            .contains(TriangleFlags::GROUP_WITH_ANY)
        {
            let i: i32 = if *piTriListIn.offset((3i32 * f + 0i32) as isize) == iVertexRepresentitive
            {
                0i32
            } else if *piTriListIn.offset((3i32 * f + 1i32) as isize) == iVertexRepresentitive {
                1i32
            } else if *piTriListIn.offset((3i32 * f + 2i32) as isize) == iVertexRepresentitive {
                2i32
            } else {
                panic!();
            };
            let index = *piTriListIn.offset((3i32 * f + i) as isize);
            let n = get_normal(geometry, index as usize);
            let mut vOs = (*pTriInfos.offset(f as isize)).vOs
                - (n.dot((*pTriInfos.offset(f as isize)).vOs) * n);
            let mut vOt = (*pTriInfos.offset(f as isize)).vOt
                - (n.dot((*pTriInfos.offset(f as isize)).vOt) * n);
            vOs = vOs.normalize_or_zero();
            vOt = vOt.normalize_or_zero();

            let i2 =
                *piTriListIn.offset((3i32 * f + if i < 2i32 { i + 1i32 } else { 0i32 }) as isize);
            let i1 = *piTriListIn.offset((3i32 * f + i) as isize);
            let i0 =
                *piTriListIn.offset((3i32 * f + if i > 0i32 { i - 1i32 } else { 2i32 }) as isize);
            let p0 = get_position(geometry, i0 as usize);
            let p1 = get_position(geometry, i1 as usize);
            let p2 = get_position(geometry, i2 as usize);
            let v1 = p0 - p1;
            let v2 = p2 - p1;
            let mut v1 = v1 - (n.dot(v1) * n);
            v1 = v1.normalize_or_zero();

            let mut v2 = v2 - (n.dot(v2) * n);
            v2 = v2.normalize_or_zero();
            let fCos = v1.dot(v2).clamp(-1., 1.);

            let fAngle = (fCos as f64).acos() as f32;
            let fMagS = (*pTriInfos.offset(f as isize)).fMagS;
            let fMagT = (*pTriInfos.offset(f as isize)).fMagT;
            res.vOs = res.vOs + (fAngle * vOs);
            res.vOt = res.vOt + (fAngle * vOt);
            res.fMagS += fAngle * fMagS;
            res.fMagT += fAngle * fMagT;
            fAngleSum += fAngle
        }
    }
    res.vOs = res.vOs.normalize_or_zero();
    res.vOt = res.vOt.normalize_or_zero();

    if fAngleSum > 0i32 as f32 {
        res.fMagS /= fAngleSum;
        res.fMagT /= fAngleSum
    }
    return res;
}

unsafe fn Build4RuleGroups(
    mut pTriInfos: *mut STriInfo,
    mut pGroups: *mut SGroup,
    mut piGroupTrianglesBuffer: *mut i32,
    mut piTriListIn: *const i32,
    iNrTrianglesIn: i32,
) -> i32 {
    let mut iNrActiveGroups: i32 = 0i32;
    let mut iOffset: i32 = 0i32;
    let iNrMaxGroups = iNrTrianglesIn * 3;

    for f in 0..iNrTrianglesIn {
        for i in 0..3i32 {
            if !(*pTriInfos.offset(f as isize))
                .iFlag
                .contains(TriangleFlags::GROUP_WITH_ANY)
                && (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize].is_null()
            {
                let vert_index: i32 = *piTriListIn.offset((f * 3i32 + i) as isize);
                let ref mut fresh2 = (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize];
                debug_assert!(iNrActiveGroups < iNrMaxGroups);
                *fresh2 = &mut *pGroups.offset(iNrActiveGroups as isize) as *mut SGroup;
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize])
                    .iVertexRepresentitive = vert_index;
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).bOrientPreservering =
                    (*pTriInfos.offset(f as isize))
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).iNrFaces = 0i32;
                let ref mut fresh3 =
                    (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).pFaceIndices;
                *fresh3 = &mut *piGroupTrianglesBuffer.offset(iOffset as isize) as *mut i32;
                iNrActiveGroups += 1;
                AddTriToGroup((*pTriInfos.offset(f as isize)).AssignedGroup[i as usize], f);
                let bOrPre = (*pTriInfos.offset(f as isize))
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let mut neigh_indexL = (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize];
                let mut neigh_indexR = (*pTriInfos.offset(f as isize)).FaceNeighbors
                    [(if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
                if neigh_indexL >= 0i32 {
                    let bAnswer: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexL,
                        (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize],
                    );
                    let bOrPre2: bool = (*pTriInfos.offset(neigh_indexL as isize))
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let bDiff: bool = if bOrPre != bOrPre2 { true } else { false };
                    debug_assert!(bAnswer || bDiff)
                }
                if neigh_indexR >= 0i32 {
                    let bAnswer_0: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexR,
                        (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize],
                    );
                    let bOrPre2_0: bool = (*pTriInfos.offset(neigh_indexR as isize))
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let bDiff_0: bool = if bOrPre != bOrPre2_0 { true } else { false };
                    debug_assert!(bAnswer_0 || bDiff_0)
                }
                iOffset += (*(*pTriInfos.offset(f as isize)).AssignedGroup[i as usize]).iNrFaces;
                // since the groups are disjoint a triangle can never
                // belong to more than 3 groups. Subsequently something
                // is completely screwed if this assertion ever hits.
                debug_assert!(iOffset <= iNrMaxGroups);
            }
        }
    }
    return iNrActiveGroups;
}
// ///////////////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////////////////////////////
unsafe fn AssignRecur(
    mut piTriListIn: *const i32,
    mut psTriInfos: *mut STriInfo,
    iMyTriIndex: i32,
    mut pGroup: *mut SGroup,
) -> bool {
    let mut pMyTriInfo: *mut STriInfo =
        &mut *psTriInfos.offset(iMyTriIndex as isize) as *mut STriInfo;
    // track down vertex
    let iVertRep: i32 = (*pGroup).iVertexRepresentitive;
    let mut pVerts: *const i32 =
        &*piTriListIn.offset((3i32 * iMyTriIndex + 0i32) as isize) as *const i32;
    let i = if *pVerts.offset(0isize) == iVertRep {
        0i32
    } else if *pVerts.offset(1isize) == iVertRep {
        1i32
    } else if *pVerts.offset(2isize) == iVertRep {
        2i32
    } else {
        panic!();
    };
    if (*pMyTriInfo).AssignedGroup[i as usize] == pGroup {
        return true;
    } else {
        if !(*pMyTriInfo).AssignedGroup[i as usize].is_null() {
            return false;
        }
    }
    if (*pMyTriInfo).iFlag.contains(TriangleFlags::GROUP_WITH_ANY) {
        if (*pMyTriInfo).AssignedGroup[0usize].is_null()
            && (*pMyTriInfo).AssignedGroup[1usize].is_null()
            && (*pMyTriInfo).AssignedGroup[2usize].is_null()
        {
            (*pMyTriInfo).iFlag.set(
                TriangleFlags::ORIENT_PRESERVING,
                (*pGroup).bOrientPreservering,
            );
        }
    }
    let bOrient: bool = (*pMyTriInfo)
        .iFlag
        .contains(TriangleFlags::ORIENT_PRESERVING);
    if bOrient != (*pGroup).bOrientPreservering {
        return false;
    }
    AddTriToGroup(pGroup, iMyTriIndex);
    (*pMyTriInfo).AssignedGroup[i as usize] = pGroup;
    let neigh_indexL: i32 = (*pMyTriInfo).FaceNeighbors[i as usize];
    let neigh_indexR: i32 =
        (*pMyTriInfo).FaceNeighbors[(if i > 0i32 { i - 1i32 } else { 2i32 }) as usize];
    if neigh_indexL >= 0i32 {
        AssignRecur(piTriListIn, psTriInfos, neigh_indexL, pGroup);
    }
    if neigh_indexR >= 0i32 {
        AssignRecur(piTriListIn, psTriInfos, neigh_indexR, pGroup);
    }
    return true;
}
unsafe fn AddTriToGroup(mut pGroup: *mut SGroup, iTriIndex: i32) {
    *(*pGroup).pFaceIndices.offset((*pGroup).iNrFaces as isize) = iTriIndex;
    (*pGroup).iNrFaces += 1;
}
unsafe fn InitTriInfo(
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *const i32,
    geometry: &impl Geometry,
    iNrTrianglesIn: usize,
) {
    for f in 0..iNrTrianglesIn {
        for i in 0..3i32 {
            (*pTriInfos.offset(f as isize)).FaceNeighbors[i as usize] = -1i32;
            let ref mut fresh4 = (*pTriInfos.offset(f as isize)).AssignedGroup[i as usize];
            *fresh4 = 0 as *mut SGroup;
            (*pTriInfos.offset(f as isize)).vOs.x = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOs.y = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOs.z = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.x = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.y = 0.0f32;
            (*pTriInfos.offset(f as isize)).vOt.z = 0.0f32;
            (*pTriInfos.offset(f as isize)).fMagS = 0i32 as f32;
            (*pTriInfos.offset(f as isize)).fMagT = 0i32 as f32;
            // C: assumed bad
            (*pTriInfos.offset(f as isize))
                .iFlag
                .insert(TriangleFlags::GROUP_WITH_ANY);
        }
    }
    for f in 0..iNrTrianglesIn {
        let v1 = get_position(geometry, *piTriListIn.offset((f * 3 + 0) as isize) as usize);
        let v2 = get_position(geometry, *piTriListIn.offset((f * 3 + 1) as isize) as usize);
        let v3 = get_position(geometry, *piTriListIn.offset((f * 3 + 2) as isize) as usize);
        let t1 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 0) as isize) as usize);
        let t2 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 1) as isize) as usize);
        let t3 = get_tex_coord(geometry, *piTriListIn.offset((f * 3 + 2) as isize) as usize);
        let t21x: f32 = t2.x - t1.x;
        let t21y: f32 = t2.y - t1.y;
        let t31x: f32 = t3.x - t1.x;
        let t31y: f32 = t3.y - t1.y;
        let d1 = v2 - v1;
        let d2 = v3 - v1;
        let fSignedAreaSTx2: f32 = t21x * t31y - t21y * t31x;
        let mut vOs = (t31y * d1) - (t21y * d2);
        let mut vOt = (-t31x * d1) + (t21x * d2);
        if fSignedAreaSTx2 > 0.0 {
            (*pTriInfos.offset(f as isize))
                .iFlag
                .insert(TriangleFlags::ORIENT_PRESERVING);
        }
        if fSignedAreaSTx2.is_normal() {
            let fAbsArea: f32 = fSignedAreaSTx2.abs();
            let fLenOs: f32 = vOs.length();
            let fLenOt: f32 = vOt.length();
            let fS: f32 = if !(*pTriInfos.offset(f as isize))
                .iFlag
                .contains(TriangleFlags::ORIENT_PRESERVING)
            {
                -1.0f32
            } else {
                1.0f32
            };
            if fLenOs.is_normal() {
                (*pTriInfos.offset(f as isize)).vOs = (fS / fLenOs) * vOs
            }
            if fLenOt.is_normal() {
                (*pTriInfos.offset(f as isize)).vOt = (fS / fLenOt) * vOt
            }
            (*pTriInfos.offset(f as isize)).fMagS = fLenOs / fAbsArea;
            (*pTriInfos.offset(f as isize)).fMagT = fLenOt / fAbsArea;
            if ((*pTriInfos.offset(f as isize)).fMagS.is_normal())
                && (*pTriInfos.offset(f as isize)).fMagT.is_normal()
            {
                (*pTriInfos.offset(f as isize))
                    .iFlag
                    .remove(TriangleFlags::GROUP_WITH_ANY);
            }
        }
    }
    let mut t = 0;
    while t < iNrTrianglesIn - 1 {
        let iFO_a: i32 = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
        let iFO_b: i32 = (*pTriInfos.offset((t + 1) as isize)).iOrgFaceNumber;
        if iFO_a == iFO_b {
            let bIsDeg_a: bool = (*pTriInfos.offset(t as isize))
                .iFlag
                .contains(TriangleFlags::DEGENERATE);
            let bIsDeg_b: bool = (*pTriInfos.offset((t + 1) as isize))
                .iFlag
                .contains(TriangleFlags::DEGENERATE);
            if !(bIsDeg_a || bIsDeg_b) {
                let bOrientA: bool = (*pTriInfos.offset(t as isize))
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let bOrientB: bool = (*pTriInfos.offset((t + 1) as isize))
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                if bOrientA != bOrientB {
                    let mut bChooseOrientFirstTri: bool = false;
                    if (*pTriInfos.offset((t + 1) as isize))
                        .iFlag
                        .contains(TriangleFlags::GROUP_WITH_ANY)
                    {
                        bChooseOrientFirstTri = true
                    } else if CalcTexArea(geometry, &*piTriListIn.offset((t * 3 + 0) as isize))
                        >= CalcTexArea(geometry, &*piTriListIn.offset(((t + 1) * 3 + 0) as isize))
                    {
                        bChooseOrientFirstTri = true
                    }
                    let t0 = if bChooseOrientFirstTri { t } else { t + 1 };
                    let t1_0 = if bChooseOrientFirstTri { t + 1 } else { t };
                    (*pTriInfos.offset(t1_0 as isize)).iFlag.set(
                        TriangleFlags::ORIENT_PRESERVING,
                        (*pTriInfos.offset(t0 as isize))
                            .iFlag
                            .contains(TriangleFlags::ORIENT_PRESERVING),
                    );
                }
            }
            t += 2
        } else {
            t += 1
        }
    }

    BuildNeighborsFast(pTriInfos, piTriListIn, iNrTrianglesIn as i32);
}

unsafe fn BuildNeighborsFast(
    mut pTriInfos: *mut STriInfo,
    mut piTriListIn: *const i32,
    iNrTrianglesIn: i32,
) {
    let mut pEdges = Vec::with_capacity((iNrTrianglesIn * 3) as usize);
    // build array of edges
    for f in 0..iNrTrianglesIn {
        for i in 0..3i32 {
            let i0: i32 = *piTriListIn.offset((f * 3i32 + i) as isize);
            let i1: i32 = *piTriListIn.offset((f * 3i32 + (i + 1) % 3) as isize);
            // Ensure that the indices have a consistent order by making i0 the smaller
            pEdges.push(SEdge {
                i0: i0.min(i1),
                i1: i0.max(i1),
                f,
            });
        }
    }
    pEdges.sort();

    let iEntries = iNrTrianglesIn * 3i32;

    for i in 0..iEntries {
        let edge = pEdges[i as usize];
        let i0_0: i32 = edge.i0;
        let i1_0: i32 = edge.i1;
        let f_0: i32 = edge.f;

        let (i0_A, i1_A, edgenum_A) =
            GetEdge(&*piTriListIn.offset((f_0 * 3i32) as isize), i0_0, i1_0);
        let bUnassigned_A =
            (*pTriInfos.offset(f_0 as isize)).FaceNeighbors[edgenum_A as usize] == -1i32;
        if bUnassigned_A {
            let mut j: i32 = i + 1i32;

            while j < iEntries && i0_0 == pEdges[j as usize].i0 && i1_0 == pEdges[j as usize].i1 {
                let t = pEdges[j as usize].f;
                // C: Flip i1 and i0
                let (i1_B, i0_B, edgenum_B) = GetEdge(
                    &*piTriListIn.offset((t * 3i32) as isize),
                    pEdges[j as usize].i0,
                    pEdges[j as usize].i1,
                );
                let bUnassigned_B =
                    (*pTriInfos.offset(t as isize)).FaceNeighbors[edgenum_B as usize] == -1i32;
                if i0_A == i0_B && i1_A == i1_B && bUnassigned_B {
                    let mut t_0: i32 = pEdges[j as usize].f;
                    (*pTriInfos.offset(f_0 as isize)).FaceNeighbors[edgenum_A as usize] = t_0;
                    (*pTriInfos.offset(t_0 as isize)).FaceNeighbors[edgenum_B as usize] = f_0;
                    break;
                } else {
                    j += 1
                }
            }
        }
    }
}
unsafe fn GetEdge(mut indices: *const i32, i0_in: i32, i1_in: i32) -> (i32, i32, i32) {
    if *indices.offset(0isize) == i0_in || *indices.offset(0isize) == i1_in {
        if *indices.offset(1isize) == i0_in || *indices.offset(1isize) == i1_in {
            (*indices.offset(0isize), *indices.offset(1isize), 0)
        } else {
            (*indices.offset(2isize), *indices.offset(0isize), 2)
        }
    } else {
        (*indices.offset(1isize), *indices.offset(2isize), 1)
    }
}
// ///////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////////////////////

// returns the texture area times 2
unsafe fn CalcTexArea(geometry: &impl Geometry, mut indices: *const i32) -> f32 {
    let t1 = get_tex_coord(geometry, *indices.offset(0isize) as usize);
    let t2 = get_tex_coord(geometry, *indices.offset(1isize) as usize);
    let t3 = get_tex_coord(geometry, *indices.offset(2isize) as usize);
    let t21x: f32 = t2.x - t1.x;
    let t21y: f32 = t2.y - t1.y;
    let t31x: f32 = t3.x - t1.x;
    let t31y: f32 = t3.y - t1.y;
    let fSignedAreaSTx2: f32 = t21x * t31y - t21y * t31x;
    return if fSignedAreaSTx2 < 0i32 as f32 {
        -fSignedAreaSTx2
    } else {
        fSignedAreaSTx2
    };
}

// degen triangles
unsafe fn DegenPrologue(
    mut pTriInfos: *mut STriInfo,
    mut piTriList_out: *mut i32,
    iNrTrianglesIn: i32,
    iTotTris: i32,
) {
    // locate quads with only one good triangle
    let mut t: i32 = 0i32;
    while t < iTotTris - 1i32 {
        let iFO_a: i32 = (*pTriInfos.offset(t as isize)).iOrgFaceNumber;
        let iFO_b: i32 = (*pTriInfos.offset((t + 1i32) as isize)).iOrgFaceNumber;
        if iFO_a == iFO_b {
            let bIsDeg_a: bool = (*pTriInfos.offset(t as isize))
                .iFlag
                .contains(TriangleFlags::DEGENERATE);
            let bIsDeg_b: bool = (*pTriInfos.offset((t + 1i32) as isize))
                .iFlag
                .contains(TriangleFlags::DEGENERATE);
            // If exactly one is degenerate, mark both as QUAD_ONE_DEGENERATE_TRI, i.e. that the other triangle
            // (If both are degenerate, this)
            if bIsDeg_a ^ bIsDeg_b {
                (*pTriInfos.offset(t as isize))
                    .iFlag
                    .insert(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
                (*pTriInfos.offset((t + 1i32) as isize))
                    .iFlag
                    .insert(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
            }
            t += 2i32
        } else {
            t += 1
        }
    }

    // reorder list so all degen triangles are moved to the back
    // without reordering the good triangles
    // That is, a semi-stable partition, e.g. as described at
    // https://dlang.org/library/std/algorithm/sorting/partition.html
    // TODO: Use `Vec::retain` with a second vec here - not perfect,
    // but good enough and safe.
    // TODO: Consider using `sort_by_key` on Vec instead (which is stable) - it might be
    // technically slower, but it's much easier to reason about
    let mut iNextGoodTriangleSearchIndex = 1i32;
    t = 0i32;
    let mut bStillFindingGoodOnes = true;
    while t < iNrTrianglesIn && bStillFindingGoodOnes {
        let bIsGood: bool = !(*pTriInfos.offset(t as isize))
            .iFlag
            .contains(TriangleFlags::DEGENERATE);
        if bIsGood {
            if iNextGoodTriangleSearchIndex < t + 2i32 {
                iNextGoodTriangleSearchIndex = t + 2i32
            }
        } else {
            let mut bJustADegenerate: bool = true;
            while bJustADegenerate && iNextGoodTriangleSearchIndex < iTotTris {
                let bIsGood_0: bool = !(*pTriInfos.offset(iNextGoodTriangleSearchIndex as isize))
                    .iFlag
                    .contains(TriangleFlags::DEGENERATE);
                if bIsGood_0 {
                    bJustADegenerate = false
                } else {
                    iNextGoodTriangleSearchIndex += 1
                }
            }
            let t0 = t;
            let t1 = iNextGoodTriangleSearchIndex;
            iNextGoodTriangleSearchIndex += 1;
            debug_assert!(iNextGoodTriangleSearchIndex > (t + 1));
            // Swap t0 and t1
            if !bJustADegenerate {
                for i in 0..3i32 {
                    let index: i32 = *piTriList_out.offset((t0 * 3i32 + i) as isize);
                    *piTriList_out.offset((t0 * 3i32 + i) as isize) =
                        *piTriList_out.offset((t1 * 3i32 + i) as isize);
                    *piTriList_out.offset((t1 * 3i32 + i) as isize) = index;
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
    debug_assert!(iNrTrianglesIn == t);
    debug_assert!(bStillFindingGoodOnes);
}
fn GenerateSharedVerticesIndexList(
    // The input vertex index->face/vert mappings
    // Identical face/verts will have each vertex index
    // point to the same (arbitrary?) face/vert
    // TODO: This seems overly complicated - storing vertex properties in a
    // side channel seems much easier.
    // Hopefully implementation can be changed to just use a btreemap or
    // something too.
    mut piTriList_in_and_out: &mut [i32],
    geometry: &impl Geometry,
) {
    let mut map = BTreeMap::new();
    for vertex_index in piTriList_in_and_out {
        let index = *vertex_index as usize;
        let vertex_properties = [
            get_position(geometry, index),
            get_normal(geometry, index),
            get_tex_coord(geometry, index),
        ]
        // We need to make the vertex properties finite to be able to use them in a btreemap
        // Technically, these unwraps aren't ideal, but the original puts absolutely no thought into its handling of
        // NaN and infinity, so it's probably *fine*. (I strongly suspect that infinity or NaN would have
        //  lead to UB somewhere)
        .map(|prop| FiniteVec3::new(prop).unwrap());

        *vertex_index = *(map.entry(vertex_properties).or_insert(*vertex_index));
    }
}

fn GenerateInitialVerticesIndexList(
    pTriInfos: &mut [STriInfo],
    piTriList_out: &mut [i32],
    geometry: &impl Geometry,
    iNrTrianglesIn: usize,
) -> usize {
    let mut iTSpacesOffs: usize = 0;
    let mut iDstTriIndex = 0;
    for f in 0..geometry.num_faces() {
        let verts = geometry.num_vertices_of_face(f);

        pTriInfos[iDstTriIndex].iOrgFaceNumber = f as i32;
        pTriInfos[iDstTriIndex].iTSpacesOffs = iTSpacesOffs as i32;
        if let FaceKind::Triangle = verts {
            let mut pVerts = &mut pTriInfos[iDstTriIndex].vert_num;
            pVerts[0] = 0;
            pVerts[1] = 1;
            pVerts[2] = 2;
            piTriList_out[iDstTriIndex * 3 + 0] = face_vert_to_index(f, 0) as i32;
            piTriList_out[iDstTriIndex * 3 + 1] = face_vert_to_index(f, 1) as i32;
            piTriList_out[iDstTriIndex * 3 + 2] = face_vert_to_index(f, 2) as i32;
            iDstTriIndex += 1
        } else {
            pTriInfos[iDstTriIndex + 1].iOrgFaceNumber = f as i32;
            pTriInfos[iDstTriIndex + 1].iTSpacesOffs = iTSpacesOffs as i32;
            let i0 = face_vert_to_index(f, 0);
            let i1 = face_vert_to_index(f, 1);
            let i2 = face_vert_to_index(f, 2);
            let i3 = face_vert_to_index(f, 3);
            let T0 = get_tex_coord(geometry, i0);
            let T1 = get_tex_coord(geometry, i1);
            let T2 = get_tex_coord(geometry, i2);
            let T3 = get_tex_coord(geometry, i3);
            let distSQ_02: f32 = (T2 - T0).length_squared();
            let distSQ_13: f32 = (T3 - T1).length_squared();
            let bQuadDiagIs_02: bool;
            if distSQ_02 < distSQ_13 {
                bQuadDiagIs_02 = true
            } else if distSQ_13 < distSQ_02 {
                bQuadDiagIs_02 = false
            } else {
                let P0 = get_position(geometry, i0);
                let P1 = get_position(geometry, i1);
                let P2 = get_position(geometry, i2);
                let P3 = get_position(geometry, i3);
                let distSQ_02_0: f32 = (P2 - P0).length_squared();
                let distSQ_13_0: f32 = (P3 - P1).length_squared();
                bQuadDiagIs_02 = if distSQ_13_0 < distSQ_02_0 {
                    false
                } else {
                    true
                }
            }
            if bQuadDiagIs_02 {
                let mut pVerts_A = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_A[0] = 0;
                pVerts_A[1] = 1;
                pVerts_A[2] = 2;
                piTriList_out[iDstTriIndex * 3 + 0] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i2 as i32;
                iDstTriIndex += 1;

                let mut pVerts_B = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_B[0] = 0;
                pVerts_B[1] = 2;
                pVerts_B[2] = 3;
                piTriList_out[iDstTriIndex * 3 + 0] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i2 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1
            } else {
                let mut pVerts_A_0 = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_A_0[0] = 0;
                pVerts_A_0[1] = 1;
                pVerts_A_0[2] = 3;
                piTriList_out[iDstTriIndex * 3 + 0] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1;

                let mut pVerts_B_0 = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_B_0[0] = 1;
                pVerts_B_0[1] = 2;
                pVerts_B_0[2] = 3;
                piTriList_out[iDstTriIndex * 3 + 0] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i2 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1
            }
        }
        iTSpacesOffs += verts.num_vertices();
        assert!(iDstTriIndex <= iNrTrianglesIn);
    }

    for t in 0..iNrTrianglesIn {
        pTriInfos[t].iFlag = TriangleFlags::empty();
    }
    return iTSpacesOffs;
}
