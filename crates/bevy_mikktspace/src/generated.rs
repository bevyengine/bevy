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

#![allow(dead_code, non_camel_case_types, non_snake_case, unused_mut)]

mod degenerate;
mod setup;

use bitflags::bitflags;
use glam::Vec3;

use crate::{get_normal, get_position, FaceKind, Geometry};

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
    /// The group each vertex belongs to.
    /// This should be Option<NonMaxUsize>, but rustc hasn't added that yet
    pub AssignedGroup: [usize; 3],
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
            FaceNeighbors: [-1, -1, -1],
            AssignedGroup: [usize::MAX; 3],
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
    pub pFaceIndices: usize,
    pub iVertexRepresentitive: i32,
    pub bOrientPreservering: bool,
}

impl SGroup {
    fn zero() -> Self {
        Self {
            iNrFaces: 0,
            pFaceIndices: usize::MAX,
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
pub fn genTangSpace(geometry: &mut impl Geometry, fAngularThreshold: f32) -> bool {
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

    if iNrTrianglesIn == 0 {
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
    let iNrTSPaces = setup::GenerateInitialVerticesIndexList(
        &mut pTriInfos,
        &mut piTriListIn,
        geometry,
        iNrTrianglesIn,
    );
    // C: Make a welded index list of identical positions and attributes (pos, norm, texc)
    setup::GenerateSharedVerticesIndexList(&mut piTriListIn, geometry);

    let iTotTris = iNrTrianglesIn;
    let mut iDegenTriangles = 0;
    // C: Mark all degenerate triangles
    for t in 0..(iTotTris as usize) {
        let i0 = piTriListIn[t * 3];
        let i1 = piTriListIn[t * 3 + 1];
        let i2 = piTriListIn[t * 3 + 2];
        let p0 = get_position(geometry, i0 as usize);
        let p1 = get_position(geometry, i1 as usize);
        let p2 = get_position(geometry, i2 as usize);
        if p0 == p1 || p0 == p2 || p1 == p2 {
            pTriInfos[t].iFlag.insert(TriangleFlags::DEGENERATE);
            iDegenTriangles += 1;
        }
    }
    let iNrTrianglesIn = iTotTris - iDegenTriangles;
    // C: Mark all triangle pairs that belong to a quad with only one
    // C: good triangle. These need special treatment in DegenEpilogue().
    // C: Additionally, move all good triangles to the start of
    // C: pTriInfos[] and piTriListIn[] without changing order and
    // C: put the degenerate triangles last.
    // Note: A quad can have degenerate triangles if two vertices are in the same location
    degenerate::DegenPrologue(
        &mut pTriInfos,
        &mut piTriListIn,
        iNrTrianglesIn as i32,
        iTotTris as i32,
    );
    // C: Evaluate triangle level attributes and neighbor list
    setup::InitTriInfo(&mut pTriInfos, &piTriListIn, geometry, iNrTrianglesIn);
    //C: Based on the 4 rules, identify groups based on connectivity
    let iNrMaxGroups = iNrTrianglesIn * 3;

    let mut pGroups = vec![SGroup::zero(); iNrMaxGroups];
    let mut piGroupTrianglesBuffer = vec![0; iNrTrianglesIn * 3];

    let iNrActiveGroups = Build4RuleGroups(
        &mut pTriInfos,
        &mut pGroups,
        &mut piGroupTrianglesBuffer,
        &piTriListIn,
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
        &pTriInfos,
        &pGroups,
        iNrActiveGroups,
        &piTriListIn,
        fThresCos,
        geometry,
        &piGroupTrianglesBuffer,
    );
    if !bRes {
        return false;
    }
    degenerate::DegenEpilogue(
        &mut psTspace,
        &mut pTriInfos,
        &mut piTriListIn,
        geometry,
        iNrTrianglesIn as i32,
        iTotTris as i32,
    );

    // Output generated triangles
    let mut index = 0;
    for f in 0..iNrFaces {
        let verts_0 = geometry.num_vertices_of_face(f);
        for i in 0..verts_0.num_vertices() {
            let mut pTSpace = &psTspace[index];
            geometry.set_tangent(
                pTSpace.vOs.into(),
                pTSpace.vOt.into(),
                pTSpace.fMagS,
                pTSpace.fMagT,
                (*pTSpace).bOrient,
                f,
                i,
            );
            index += 1;
        }
    }

    true
}

fn GenerateTSpaces(
    psTspace: &mut [STSpace],
    mut pTriInfos: &[STriInfo],
    mut pGroups: &[SGroup],
    iNrActiveGroups: i32,
    mut piTriListIn: &[i32],
    fThresCos: f32,
    geometry: &impl Geometry,
    piGroupTrianglesBuffer: &[i32],
) -> bool {
    let mut iMaxNrFaces = pGroups[..iNrActiveGroups as usize]
        .iter()
        .map(|it| it.iNrFaces)
        .max();

    let iMaxNrFaces = match iMaxNrFaces {
        Some(0) | None => return true,
        Some(iMaxNrFaces) => iMaxNrFaces as usize,
    };

    let mut pSubGroupTspace = vec![STSpace::zero(); iMaxNrFaces];
    let mut pUniSubGroups = vec![SSubGroup::zero(); iMaxNrFaces];
    let mut pTmpMembers = vec![0i32; iMaxNrFaces];

    for g in 0..iNrActiveGroups {
        let mut pGroup: &SGroup = &pGroups[g as usize];
        let mut iUniqueSubGroups = 0;

        for i in 0..pGroup.iNrFaces {
            let f = piGroupTrianglesBuffer[pGroup.pFaceIndices + i as usize];
            let mut tmp_group: SSubGroup = SSubGroup {
                iNrFaces: 0,
                pTriMembers: Vec::new(),
            };
            let index = pTriInfos[f as usize]
                .AssignedGroup
                .iter()
                .position(|&it| it == g as usize)
                .unwrap();

            let iVertIndex = piTriListIn[(f * 3 + index as i32) as usize];
            assert!(iVertIndex == (*pGroup).iVertexRepresentitive);
            let n = get_normal(geometry, iVertIndex as usize);
            let mut vOs = pTriInfos[f as usize].vOs - (n.dot(pTriInfos[f as usize].vOs) * n);
            let mut vOt = pTriInfos[f as usize].vOt - (n.dot(pTriInfos[f as usize].vOt) * n);
            vOs = vOs.normalize_or_zero();
            vOt = vOt.normalize_or_zero();

            let iOF_1 = pTriInfos[f as usize].iOrgFaceNumber;
            let mut iMembers = 0;

            for j in 0..pGroup.iNrFaces {
                let t: i32 = piGroupTrianglesBuffer[pGroup.pFaceIndices + j as usize];
                let tri = &pTriInfos[t as usize];
                let iOF_2: i32 = tri.iOrgFaceNumber;
                let mut vOs2 = tri.vOs - (n.dot(tri.vOs) * n);
                let mut vOt2 = tri.vOt - (n.dot(tri.vOt) * n);
                vOs2 = vOs2.normalize_or_zero();
                vOt2 = vOt2.normalize_or_zero();

                let bAny: bool = (pTriInfos[f as usize].iFlag | pTriInfos[t as usize].iFlag)
                    .contains(TriangleFlags::GROUP_WITH_ANY);
                let bSameOrgFace: bool = iOF_1 == iOF_2;
                let fCosS: f32 = vOs.dot(vOs2);
                let fCosT: f32 = vOt.dot(vOt2);
                debug_assert!(f != t || bSameOrgFace); // sanity check
                if bAny || bSameOrgFace || fCosS > fThresCos && fCosT > fThresCos {
                    pTmpMembers[iMembers + 1] = t;
                }
            }
            if iMembers > 1 {
                pTmpMembers[0..(iMembers - 1)].sort_unstable();
            }
            tmp_group.iNrFaces = iMembers as i32;
            tmp_group.pTriMembers = pTmpMembers.clone();

            let mut found = None;
            #[allow(clippy::needless_range_loop)]
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

                pSubGroupTspace[iUniqueSubGroups] = EvalTspace(
                    &mut tmp_group.pTriMembers,
                    iMembers as i32,
                    piTriListIn,
                    pTriInfos,
                    geometry,
                    (*pGroup).iVertexRepresentitive,
                );
                // C: if no match was found we allocate a new subgroup
                pUniSubGroups[iUniqueSubGroups].iNrFaces = iMembers as i32;
                pUniSubGroups[iUniqueSubGroups].pTriMembers = tmp_group.pTriMembers;
                iUniqueSubGroups += 1;
            }
            let iOffs = pTriInfos[f as usize].iTSpacesOffs as usize;
            let iVert = pTriInfos[f as usize].vert_num[index as usize] as usize;
            let mut pTS_out = &mut psTspace[iOffs + iVert];
            assert!(pTS_out.iCounter < 2);
            debug_assert!(
                (*pGroup).bOrientPreservering
                    == pTriInfos[f as usize]
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING)
            );
            if (*pTS_out).iCounter == 1i32 {
                *pTS_out = AvgTSpace(pTS_out, &pSubGroupTspace[idx]);
                (*pTS_out).iCounter = 2i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering;
            } else {
                debug_assert!(pTS_out.iCounter == 0);
                *pTS_out = pSubGroupTspace[idx];
                (*pTS_out).iCounter = 1i32;
                (*pTS_out).bOrient = (*pGroup).bOrientPreservering;
            }
        }
    }
    true
}
fn AvgTSpace(mut pTS0: &STSpace, mut pTS1: &STSpace) -> STSpace {
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
        ts_res.vOt = (*pTS0).vOt;
    } else {
        ts_res.fMagS = 0.5f32 * ((*pTS0).fMagS + (*pTS1).fMagS);
        ts_res.fMagT = 0.5f32 * ((*pTS0).fMagT + (*pTS1).fMagT);
        ts_res.vOs = (*pTS0).vOs + (*pTS1).vOs;
        ts_res.vOt = (*pTS0).vOt + (*pTS1).vOt;
        ts_res.vOs = ts_res.vOs.normalize_or_zero();
        ts_res.vOt = ts_res.vOt.normalize_or_zero();
    }
    ts_res
}

fn EvalTspace(
    mut face_indices: &mut [i32],
    iFaces: i32,
    mut piTriListIn: &[i32],
    mut pTriInfos: &[STriInfo],
    geometry: &impl Geometry,
    iVertexRepresentitive: i32,
) -> STSpace {
    let mut res: STSpace = STSpace::zero();
    let mut fAngleSum: f32 = 0i32 as f32;

    for face in 0..iFaces {
        let f: i32 = face_indices[face as usize];
        if !pTriInfos[f as usize]
            .iFlag
            .contains(TriangleFlags::GROUP_WITH_ANY)
        {
            let i: i32 = if piTriListIn[(3i32 * f) as usize] == iVertexRepresentitive {
                0i32
            } else if piTriListIn[(3i32 * f + 1i32) as usize] == iVertexRepresentitive {
                1i32
            } else if piTriListIn[(3i32 * f + 2i32) as usize] == iVertexRepresentitive {
                2i32
            } else {
                panic!();
            };
            let index = piTriListIn[(3i32 * f + i) as usize];
            let n = get_normal(geometry, index as usize);
            let mut vOs = pTriInfos[f as usize].vOs - (n.dot(pTriInfos[f as usize].vOs) * n);
            let mut vOt = pTriInfos[f as usize].vOt - (n.dot(pTriInfos[f as usize].vOt) * n);
            vOs = vOs.normalize_or_zero();
            vOt = vOt.normalize_or_zero();

            let i2 = piTriListIn[(3i32 * f + (i + 1) % 3) as usize];
            let i1 = piTriListIn[(3i32 * f + i) as usize];
            let i0 = piTriListIn[(3i32 * f + (i + 2) % 3) as usize];
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
            let fMagS = pTriInfos[f as usize].fMagS;
            let fMagT = pTriInfos[f as usize].fMagT;
            res.vOs += fAngle * vOs;
            res.vOt += fAngle * vOt;
            res.fMagS += fAngle * fMagS;
            res.fMagT += fAngle * fMagT;
            fAngleSum += fAngle;
        }
    }
    res.vOs = res.vOs.normalize_or_zero();
    res.vOt = res.vOt.normalize_or_zero();

    if fAngleSum > 0i32 as f32 {
        res.fMagS /= fAngleSum;
        res.fMagT /= fAngleSum;
    }
    res
}

fn Build4RuleGroups(
    mut pTriInfos: &mut [STriInfo],
    mut pGroups: &mut [SGroup],
    mut piGroupTrianglesBuffer: &mut [i32],
    mut piTriListIn: &[i32],
    iNrTrianglesIn: i32,
) -> i32 {
    let mut iNrActiveGroups: i32 = 0i32;
    let mut iOffset: i32 = 0i32;
    let iNrMaxGroups = iNrTrianglesIn * 3;

    for f in 0..iNrTrianglesIn {
        for i in 0..3i32 {
            if !pTriInfos[f as usize]
                .iFlag
                .contains(TriangleFlags::GROUP_WITH_ANY)
                && pTriInfos[f as usize].AssignedGroup[i as usize] == usize::MAX
            {
                let vert_index: i32 = piTriListIn[(f * 3 + i) as usize];
                debug_assert!(iNrActiveGroups < iNrMaxGroups);
                pTriInfos[f as usize].AssignedGroup[i as usize] = iNrActiveGroups as usize;
                let group = &mut pGroups[pTriInfos[f as usize].AssignedGroup[i as usize]];
                group.iVertexRepresentitive = vert_index;
                group.bOrientPreservering = pTriInfos[f as usize]
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                group.iNrFaces = 0i32;
                group.pFaceIndices = iOffset as usize;
                iNrActiveGroups += 1;
                AddTriToGroup(piGroupTrianglesBuffer, group, f);
                let bOrPre = pTriInfos[f as usize]
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let mut neigh_indexL = pTriInfos[f as usize].FaceNeighbors[i as usize];
                let mut neigh_indexR = pTriInfos[f as usize].FaceNeighbors[((i + 2) % 3) as usize];
                if neigh_indexL >= 0i32 {
                    let index = pTriInfos[f as usize].AssignedGroup[i as usize];
                    let bAnswer: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexL,
                        &mut pGroups[index],
                        index,
                        piGroupTrianglesBuffer,
                    );
                    let bOrPre2: bool = pTriInfos[neigh_indexL as usize]
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let bDiff: bool = bOrPre != bOrPre2;
                    debug_assert!(bAnswer || bDiff);
                }
                if neigh_indexR >= 0i32 {
                    let index = pTriInfos[f as usize].AssignedGroup[i as usize];
                    let bAnswer_0: bool = AssignRecur(
                        piTriListIn,
                        pTriInfos,
                        neigh_indexR,
                        &mut pGroups[index],
                        index,
                        piGroupTrianglesBuffer,
                    );
                    let bOrPre2_0: bool = pTriInfos[neigh_indexR as usize]
                        .iFlag
                        .contains(TriangleFlags::ORIENT_PRESERVING);
                    let bDiff_0: bool = bOrPre != bOrPre2_0;
                    debug_assert!(bAnswer_0 || bDiff_0);
                }
                iOffset += pGroups[pTriInfos[f as usize].AssignedGroup[i as usize]].iNrFaces;
                // since the groups are disjoint a triangle can never
                // belong to more than 3 groups. Subsequently something
                // is completely screwed if this assertion ever hits.
                debug_assert!(iOffset <= iNrMaxGroups);
            }
        }
    }
    iNrActiveGroups
}
// ///////////////////////////////////////////////////////////////////////////////////////////////////
/////////////////////////////////////////////////////////////////////////////////////////////////////
fn AssignRecur(
    piTriListIn: &[i32],
    psTriInfos: &mut [STriInfo],
    iMyTriIndex: i32,
    pGroup: &mut SGroup,
    group_idx: usize,
    piGroupTrianglesBuffer: &mut [i32],
) -> bool {
    let mut pMyTriInfo = &mut psTriInfos[iMyTriIndex as usize];
    // track down vertex
    let iVertRep: i32 = pGroup.iVertexRepresentitive;
    let mut pVerts = &piTriListIn[3 * iMyTriIndex as usize..][..3];
    let i = pVerts.iter().position(|&it| it == iVertRep).unwrap();
    if pMyTriInfo.AssignedGroup[i as usize] == group_idx {
        return true;
    }
    if !pMyTriInfo.AssignedGroup[i as usize] == usize::MAX {
        return false;
    }

    if (*pMyTriInfo).iFlag.contains(TriangleFlags::GROUP_WITH_ANY)
        && (*pMyTriInfo).AssignedGroup[0usize] == usize::MAX
        && (*pMyTriInfo).AssignedGroup[1usize] == usize::MAX
        && (*pMyTriInfo).AssignedGroup[2usize] == usize::MAX
    {
        (*pMyTriInfo).iFlag.set(
            TriangleFlags::ORIENT_PRESERVING,
            (*pGroup).bOrientPreservering,
        );
    }
    let bOrient: bool = (*pMyTriInfo)
        .iFlag
        .contains(TriangleFlags::ORIENT_PRESERVING);
    if bOrient != (*pGroup).bOrientPreservering {
        return false;
    }
    AddTriToGroup(piGroupTrianglesBuffer, pGroup, iMyTriIndex);
    pMyTriInfo.AssignedGroup[i as usize] = group_idx;
    let neigh_indexL = (*pMyTriInfo).FaceNeighbors[i as usize];
    let neigh_indexR = (*pMyTriInfo).FaceNeighbors[(if i > 0 { i - 1 } else { 2 }) as usize];
    if neigh_indexL >= 0i32 {
        AssignRecur(
            piTriListIn,
            psTriInfos,
            neigh_indexL,
            pGroup,
            group_idx,
            piGroupTrianglesBuffer,
        );
    }
    if neigh_indexR >= 0i32 {
        AssignRecur(
            piTriListIn,
            psTriInfos,
            neigh_indexR,
            pGroup,
            group_idx,
            piGroupTrianglesBuffer,
        );
    }
    true
}
fn AddTriToGroup(faces: &mut [i32], pGroup: &mut SGroup, iTriIndex: i32) {
    faces[pGroup.iNrFaces as usize + pGroup.pFaceIndices] = iTriIndex;
    pGroup.iNrFaces += 1;
}
