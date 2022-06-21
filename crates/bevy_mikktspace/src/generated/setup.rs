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
