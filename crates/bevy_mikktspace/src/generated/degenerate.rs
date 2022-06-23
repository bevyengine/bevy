use crate::face_vert_to_index;
use crate::get_position;
use crate::Geometry;

use super::STSpace;
use super::STriInfo;
use super::TriangleFlags;

pub(crate) fn DegenPrologue(
    pTriInfos: &mut [STriInfo],
    piTriList_out: &mut [i32],
    iNrTrianglesIn: i32,
    iTotTris: i32,
) {
    // locate quads with only one good triangle
    let mut t = 0;
    while t < (iTotTris as usize) - 1 {
        let [a, b] = if let [a, b] = &mut pTriInfos[t..=t + 1] {
            [a, b]
        } else {
            unreachable!()
        };
        if a.iOrgFaceNumber == b.iOrgFaceNumber {
            let bIsDeg_a: bool = a.iFlag.contains(TriangleFlags::DEGENERATE);
            let bIsDeg_b: bool = b.iFlag.contains(TriangleFlags::DEGENERATE);
            // If exactly one is degenerate, mark both as QUAD_ONE_DEGENERATE_TRI, i.e. that the other triangle
            // (If both are degenerate, this doesn't matter ?)
            if bIsDeg_a ^ bIsDeg_b {
                a.iFlag.insert(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
                b.iFlag.insert(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
            }
            t += 2;
        } else {
            t += 1;
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
    let mut iNextGoodTriangleSearchIndex = 1;
    let mut t = 0;
    let mut bStillFindingGoodOnes = true;
    while t < iNrTrianglesIn as usize && bStillFindingGoodOnes {
        let bIsGood: bool = !pTriInfos[t].iFlag.contains(TriangleFlags::DEGENERATE);
        if bIsGood {
            if iNextGoodTriangleSearchIndex < t + 2 {
                iNextGoodTriangleSearchIndex = t + 2;
            }
        } else {
            let mut bJustADegenerate: bool = true;
            while bJustADegenerate && iNextGoodTriangleSearchIndex < iTotTris as usize {
                let bIsGood_0: bool = !pTriInfos[iNextGoodTriangleSearchIndex]
                    .iFlag
                    .contains(TriangleFlags::DEGENERATE);
                if bIsGood_0 {
                    bJustADegenerate = false;
                } else {
                    iNextGoodTriangleSearchIndex += 1;
                }
            }
            let t0 = t;
            let t1 = iNextGoodTriangleSearchIndex;
            iNextGoodTriangleSearchIndex += 1;
            debug_assert!(iNextGoodTriangleSearchIndex > (t + 1));
            // Swap t0 and t1
            if !bJustADegenerate {
                let (start, end) = piTriList_out.split_at_mut(t1 * 3);

                start[t0 * 3..t0 * 3 + 3].swap_with_slice(&mut end[0..3]);
                pTriInfos.swap(t0, t1);
            } else {
                bStillFindingGoodOnes = false;
            }
        }
        if bStillFindingGoodOnes {
            t += 1;
        }
    }
    debug_assert!(iNrTrianglesIn as usize == t);
    debug_assert!(bStillFindingGoodOnes);
}

pub(crate) fn DegenEpilogue(
    psTspace: &mut [STSpace],
    pTriInfos: &mut [STriInfo],
    piTriListIn: &mut [i32],
    geometry: &impl Geometry,
    iNrTrianglesIn: i32,
    iTotTris: i32,
) {
    // For all degenerate triangles
    for t in iNrTrianglesIn..iTotTris {
        let bSkip = pTriInfos[t as usize]
            .iFlag
            .contains(TriangleFlags::QUAD_ONE_DEGENERATE_TRI);
        if !bSkip {
            for i in 0..3i32 {
                // For all vertices on that triangle
                let index1 = piTriListIn[(t * 3 + i) as usize];
                for j in 0..(3 * iNrTrianglesIn) {
                    let index2 = piTriListIn[j as usize];
                    // If the vertex properties are the same as another non-degenerate vertex
                    if index1 == index2 {
                        // assert_eq!(j, index1);
                        let iTri = j / 3i32;
                        let iVert = j % 3i32;
                        let iSrcVert = pTriInfos[iTri as usize].vert_num[iVert as usize] as i32;
                        let iSrcOffs = pTriInfos[iTri as usize].iTSpacesOffs;
                        let iDstVert = pTriInfos[t as usize].vert_num[i as usize] as i32;
                        let iDstOffs: i32 = pTriInfos[t as usize].iTSpacesOffs;
                        // Set the tangent space of this vertex to the tangent space of that vertex
                        // TODO: This is absurd - doing a linear search through all vertices for each
                        // degenerate triangle?
                        psTspace[(iDstOffs + iDstVert) as usize] =
                            psTspace[(iSrcOffs + iSrcVert) as usize];
                        break;
                    }
                }
            }
        }
    }
    for t in 0..iNrTrianglesIn {
        // Handle quads with a single degenerate triangle by
        if pTriInfos[t as usize]
            .iFlag
            .contains(TriangleFlags::QUAD_ONE_DEGENERATE_TRI)
        {
            let pV = &mut pTriInfos[t as usize].vert_num;
            let iFlag: i32 = 1i32 << pV[0] as i32 | 1i32 << pV[1] as i32 | 1i32 << pV[2] as i32;
            let mut iMissingIndex: i32 = 0i32;
            if iFlag & 2i32 == 0i32 {
                iMissingIndex = 1i32;
            } else if iFlag & 4i32 == 0i32 {
                iMissingIndex = 2i32;
            } else if iFlag & 8i32 == 0i32 {
                iMissingIndex = 3i32;
            }
            let iOrgF = pTriInfos[t as usize].iOrgFaceNumber;
            let vDstP = get_position(
                geometry,
                face_vert_to_index(iOrgF as usize, iMissingIndex as usize),
            );

            for &iVert_0 in pV.iter().take(3) {
                let vSrcP = get_position(
                    geometry,
                    face_vert_to_index(iOrgF as usize, iVert_0 as usize),
                );
                if vSrcP == vDstP {
                    let iOffs: i32 = pTriInfos[t as usize].iTSpacesOffs;
                    psTspace[(iOffs + iMissingIndex) as usize] =
                        psTspace[(iOffs + iVert_0 as i32) as usize];
                    break;
                }
            }
        }
    }
}
