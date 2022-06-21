use super::SEdge;
use super::SGroup;
use super::STriInfo;
use super::TriangleFlags;

use crate::face_vert_to_index;
use crate::ordered_vec::FiniteVec3;
use crate::FaceKind;

use std::collections::BTreeMap;

use crate::get_normal;
use crate::get_position;
use crate::get_tex_coord;
use crate::Geometry;

pub(crate) fn GenerateSharedVerticesIndexList(
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

pub(crate) fn GenerateInitialVerticesIndexList(
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

pub(crate) unsafe fn InitTriInfo(
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

pub(crate) unsafe fn BuildNeighborsFast(
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

pub(crate) unsafe fn GetEdge(mut indices: *const i32, i0_in: i32, i1_in: i32) -> (i32, i32, i32) {
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
