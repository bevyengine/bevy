use super::SEdge;
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
    piTriList_in_and_out: &mut [i32],
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
            let pVerts = &mut pTriInfos[iDstTriIndex].vert_num;
            pVerts[0] = 0;
            pVerts[1] = 1;
            pVerts[2] = 2;
            piTriList_out[iDstTriIndex * 3] = face_vert_to_index(f, 0) as i32;
            piTriList_out[iDstTriIndex * 3 + 1] = face_vert_to_index(f, 1) as i32;
            piTriList_out[iDstTriIndex * 3 + 2] = face_vert_to_index(f, 2) as i32;
            iDstTriIndex += 1;
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
                bQuadDiagIs_02 = true;
            } else if distSQ_13 < distSQ_02 {
                bQuadDiagIs_02 = false;
            } else {
                let P0 = get_position(geometry, i0);
                let P1 = get_position(geometry, i1);
                let P2 = get_position(geometry, i2);
                let P3 = get_position(geometry, i3);
                let distSQ_02_0: f32 = (P2 - P0).length_squared();
                let distSQ_13_0: f32 = (P3 - P1).length_squared();
                bQuadDiagIs_02 = distSQ_13_0 > distSQ_02_0;
            }
            if bQuadDiagIs_02 {
                let pVerts_A = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_A[0] = 0;
                pVerts_A[1] = 1;
                pVerts_A[2] = 2;
                piTriList_out[iDstTriIndex * 3] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i2 as i32;
                iDstTriIndex += 1;

                let pVerts_B = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_B[0] = 0;
                pVerts_B[1] = 2;
                pVerts_B[2] = 3;
                piTriList_out[iDstTriIndex * 3] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i2 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1;
            } else {
                let pVerts_A_0 = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_A_0[0] = 0;
                pVerts_A_0[1] = 1;
                pVerts_A_0[2] = 3;
                piTriList_out[iDstTriIndex * 3] = i0 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1;

                let pVerts_B_0 = &mut pTriInfos[iDstTriIndex].vert_num;
                pVerts_B_0[0] = 1;
                pVerts_B_0[1] = 2;
                pVerts_B_0[2] = 3;
                piTriList_out[iDstTriIndex * 3] = i1 as i32;
                piTriList_out[iDstTriIndex * 3 + 1] = i2 as i32;
                piTriList_out[iDstTriIndex * 3 + 2] = i3 as i32;
                iDstTriIndex += 1;
            }
        }
        iTSpacesOffs += verts.num_vertices();
        assert!(iDstTriIndex <= iNrTrianglesIn);
    }

    for triangle in pTriInfos.iter_mut().take(iNrTrianglesIn) {
        triangle.iFlag = TriangleFlags::empty();
    }
    iTSpacesOffs
}

pub(crate) fn InitTriInfo(
    mut pTriInfos: &mut [STriInfo],
    piTriListIn: &[i32],
    geometry: &impl Geometry,
    iNrTrianglesIn: usize,
) {
    for triangle in pTriInfos.iter_mut().take(iNrTrianglesIn) {
        // C: assumed bad
        triangle.iFlag.insert(TriangleFlags::GROUP_WITH_ANY);
    }

    for f in 0..iNrTrianglesIn {
        let v1 = get_position(geometry, piTriListIn[f * 3] as usize);
        let v2 = get_position(geometry, piTriListIn[f * 3 + 1] as usize);
        let v3 = get_position(geometry, piTriListIn[f * 3 + 2] as usize);
        let t1 = get_tex_coord(geometry, piTriListIn[f * 3] as usize);
        let t2 = get_tex_coord(geometry, piTriListIn[f * 3 + 1] as usize);
        let t3 = get_tex_coord(geometry, piTriListIn[f * 3 + 2] as usize);
        let t21x: f32 = t2.x - t1.x;
        let t21y: f32 = t2.y - t1.y;
        let t31x: f32 = t3.x - t1.x;
        let t31y: f32 = t3.y - t1.y;
        let d1 = v2 - v1;
        let d2 = v3 - v1;
        let fSignedAreaSTx2: f32 = t21x * t31y - t21y * t31x;
        let vOs = (t31y * d1) - (t21y * d2);
        let vOt = (-t31x * d1) + (t21x * d2);
        if fSignedAreaSTx2 > 0.0 {
            pTriInfos[f].iFlag.insert(TriangleFlags::ORIENT_PRESERVING);
        }
        if fSignedAreaSTx2.is_normal() {
            let fAbsArea: f32 = fSignedAreaSTx2.abs();
            let fLenOs: f32 = vOs.length();
            let fLenOt: f32 = vOt.length();
            let fS: f32 = if !pTriInfos[f]
                .iFlag
                .contains(TriangleFlags::ORIENT_PRESERVING)
            {
                -1.0f32
            } else {
                1.0f32
            };
            if fLenOs.is_normal() {
                pTriInfos[f].vOs = (fS / fLenOs) * vOs;
            }
            if fLenOt.is_normal() {
                pTriInfos[f].vOt = (fS / fLenOt) * vOt;
            }
            pTriInfos[f].fMagS = fLenOs / fAbsArea;
            pTriInfos[f].fMagT = fLenOt / fAbsArea;
            if (pTriInfos[f].fMagS.is_normal()) && (pTriInfos[f].fMagT.is_normal()) {
                pTriInfos[f].iFlag.remove(TriangleFlags::GROUP_WITH_ANY);
            }
        }
    }
    let mut t = 0;
    while t < iNrTrianglesIn - 1 {
        let iFO_a: i32 = pTriInfos[t].iOrgFaceNumber;
        let iFO_b: i32 = pTriInfos[t + 1].iOrgFaceNumber;
        if iFO_a == iFO_b {
            let bIsDeg_a: bool = pTriInfos[t].iFlag.contains(TriangleFlags::DEGENERATE);
            let bIsDeg_b: bool = pTriInfos[(t + 1)].iFlag.contains(TriangleFlags::DEGENERATE);
            if !(bIsDeg_a || bIsDeg_b) {
                let bOrientA: bool = pTriInfos[t]
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                let bOrientB: bool = pTriInfos[t + 1]
                    .iFlag
                    .contains(TriangleFlags::ORIENT_PRESERVING);
                if bOrientA != bOrientB {
                    let bChooseOrientFirstTri = pTriInfos[t + 1]
                        .iFlag
                        .contains(TriangleFlags::GROUP_WITH_ANY)
                        || (CalcTexArea(geometry, &piTriListIn[(t * 3)..])
                            >= CalcTexArea(geometry, &piTriListIn[((t + 1) * 3)..]));
                    let t0 = if bChooseOrientFirstTri { t } else { t + 1 };
                    let t1_0 = if bChooseOrientFirstTri { t + 1 } else { t };
                    pTriInfos[t1_0].iFlag.set(
                        TriangleFlags::ORIENT_PRESERVING,
                        pTriInfos[t0]
                            .iFlag
                            .contains(TriangleFlags::ORIENT_PRESERVING),
                    );
                }
            }
            t += 2;
        } else {
            t += 1;
        }
    }

    BuildNeighborsFast(pTriInfos, piTriListIn, iNrTrianglesIn as i32);
}

pub(crate) fn BuildNeighborsFast(
    mut pTriInfos: &mut [STriInfo],
    piTriListIn: &[i32],
    iNrTrianglesIn: i32,
) {
    let mut pEdges = Vec::with_capacity((iNrTrianglesIn * 3) as usize);
    // build array of edges
    for f in 0..iNrTrianglesIn {
        for i in 0..3i32 {
            let i0: i32 = piTriListIn[(f * 3 + i) as usize];
            let i1: i32 = piTriListIn[(f * 3 + (i + 1) % 3) as usize];
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

        let (i0_A, i1_A, edgenum_A) = GetEdge(&piTriListIn[(f_0 * 3) as usize..], i0_0, i1_0);
        let bUnassigned_A = pTriInfos[f_0 as usize].FaceNeighbors[edgenum_A as usize] == -1i32;
        if bUnassigned_A {
            let mut j: i32 = i + 1i32;

            while j < iEntries && i0_0 == pEdges[j as usize].i0 && i1_0 == pEdges[j as usize].i1 {
                let t = pEdges[j as usize].f;
                // C: Flip i1 and i0
                let (i1_B, i0_B, edgenum_B) = GetEdge(
                    &piTriListIn[(t * 3) as usize..],
                    pEdges[j as usize].i0,
                    pEdges[j as usize].i1,
                );
                let bUnassigned_B =
                    pTriInfos[t as usize].FaceNeighbors[edgenum_B as usize] == -1i32;
                if i0_A == i0_B && i1_A == i1_B && bUnassigned_B {
                    let t_0: i32 = pEdges[j as usize].f;
                    pTriInfos[f_0 as usize].FaceNeighbors[edgenum_A as usize] = t_0;
                    pTriInfos[t_0 as usize].FaceNeighbors[edgenum_B as usize] = f_0;
                    break;
                }
                j += 1;
            }
        }
    }
}

pub(crate) fn GetEdge(indices: &[i32], i0_in: i32, i1_in: i32) -> (i32, i32, i32) {
    let indices_to_find = [i0_in, i1_in];
    match (
        indices_to_find.contains(&indices[0]),
        indices_to_find.contains(&indices[1]),
    ) {
        (true, true) => (indices[0], indices[1], 0),
        (true, false) => (indices[2], indices[0], 2),
        (false, true) => (indices[1], indices[2], 1),
        (false, false) => unreachable!(),
    }
}
// returns the texture area times 2
fn CalcTexArea(geometry: &impl Geometry, indices: &[i32]) -> f32 {
    let t1 = get_tex_coord(geometry, indices[0] as usize);
    let t2 = get_tex_coord(geometry, indices[1] as usize);
    let t3 = get_tex_coord(geometry, indices[2] as usize);
    let t21x: f32 = t2.x - t1.x;
    let t21y: f32 = t2.y - t1.y;
    let t31x: f32 = t3.x - t1.x;
    let t31y: f32 = t3.y - t1.y;
    let fSignedAreaSTx2: f32 = t21x * t31y - t21y * t31x;
    if fSignedAreaSTx2 < 0i32 as f32 {
        -fSignedAreaSTx2
    } else {
        fSignedAreaSTx2
    }
}
