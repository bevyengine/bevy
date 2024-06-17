use bevy_math::Vec2;

// Generates texture that allows to obtain the area for a certain pattern and distances
// to the left and to right of the line.
/// based on Jorge Jimenez code <https://github.com/iryoku/smaa/blob/master/Scripts/AreaTex.py>
pub fn area_data() -> Vec<u8> {
    // Subsample offsets for orthogonal and diagonal areas:
    const SUBSAMPLE_OFFSETS_ORTHO: [f32; 7] = [
        0.0,    //0
        -0.25,  //1
        0.25,   //2
        -0.125, //3
        0.125,  //4
        -0.375, //5
        0.375,  //6
    ];
    const SUBSAMPLE_OFFSETS_DIAG: [(f32, f32); 5] = [
        (0.00, 0.00),    //0
        (0.25, -0.25),   //1
        (-0.25, 0.25),   //2
        (0.125, -0.125), //3
        (-0.125, 0.125), //4
    ];
    // Texture sizes:
    // (it's quite possible that this is not easily configurable)
    const SIZE_ORTHO: usize = 16; // * 5 slots = 80
    const SIZE_DIAG: usize = 20; // * 4 slots = 80
    const PIXEL_SIZE: usize = 2;
    const PIXELS_IN_ROW: usize = 2 * 5 /* slots */ * SIZE_ORTHO;
    const ROW_LENGTH: usize = PIXELS_IN_ROW * PIXEL_SIZE;
    const ROW_COUNT: usize = SUBSAMPLE_OFFSETS_ORTHO.len() * 5 * SIZE_ORTHO;

    // Number of samples for calculating areas in the diagonal textures:
    // (diagonal areas are calculated using brute force sampling)
    const SAMPLES_DIAG: usize = 30;

    // Maximum distance for smoothing u-shapes:
    const SMOOTH_MAX_DISTANCE: f32 = 32.;

    fn sqrt(value: Vec2) -> Vec2 {
        Vec2::new(value.x.sqrt(), value.y.sqrt())
    }

    // Smoothing function for small u-patterns:
    fn smooth_area(d: f32, a1: Vec2, a2: Vec2) -> (Vec2, Vec2) {
        let b1 = sqrt(a1 * 2.0) * 0.5;
        let b2 = sqrt(a2 * 2.0) * 0.5;
        let p = (d / SMOOTH_MAX_DISTANCE).clamp(0.0, 1.0);
        (b1.lerp(a1, p), b2.lerp(a2, p))
    }

    //------------------------------------------------------------------------------
    // Mapping Functions (for placing each pattern subtexture into its place)

    const EDGES_ORTHO: [(usize, usize); 16] = [
        (0, 0),
        (3, 0),
        (0, 3),
        (3, 3),
        (1, 0),
        (4, 0),
        (1, 3),
        (4, 3),
        (0, 1),
        (3, 1),
        (0, 4),
        (3, 4),
        (1, 1),
        (4, 1),
        (1, 4),
        (4, 4),
    ];

    const EDGES_DIAG: [(usize, usize); 16] = [
        (0, 0),
        (1, 0),
        (0, 2),
        (1, 2),
        (2, 0),
        (3, 0),
        (2, 2),
        (3, 2),
        (0, 1),
        (1, 1),
        (0, 3),
        (1, 3),
        (2, 1),
        (3, 1),
        (2, 3),
        (3, 3),
    ];
    let mut data = vec![0; ROW_LENGTH * ROW_COUNT];

    //------------------------------------------------------------------------------
    // Horizontal/Vertical Areas

    // Calculates the area for a given pattern and distances to the left and to the
    // right, biased by an offset
    fn area_ortho(pattern: usize, left: usize, right: usize, offset: f32) -> (f32, f32) {
        // Calculates the area under the line p1->p2, for the pixel x..x+1
        fn area(p1: (f32, f32), p2: (f32, f32), x: usize) -> (f32, f32) {
            let d = (p2.0 - p1.0, p2.1 - p1.1);
            let x1 = x as f32;
            let x2 = x1 + 1.0;
            let y1 = p1.1 + d.1 * (x1 - p1.0) / d.0;
            let y2 = p1.1 + d.1 * (x2 - p1.0) / d.0;

            let inside = (x1 >= p1.0 && x1 < p2.0) || (x2 > p1.0 && x2 <= p2.0);
            if inside {
                let is_trapezoid = y1.signum() == y2.signum() || y1.abs() < 1e-4 || y2.abs() < 1e-4;
                if is_trapezoid {
                    let a = (y1 + y2) / 2.0;
                    if a < 0.0 {
                        (a.abs(), 0.0)
                    } else {
                        (0.0, a.abs())
                    }
                } else {
                    // Then, we got two triangles
                    let x = -p1.1 * d.0 / d.1 + p1.0;
                    let a1 = if x > p1.0 { y1 * x.fract() / 2.0 } else { 0.0 };
                    let a2 = if x < p2.0 {
                        y2 * (1.0 - x.fract()) / 2.0
                    } else {
                        0.0
                    };
                    let a = if a1.abs() > a2.abs() { a1 } else { -a2 };
                    if a < 0.0 {
                        (a1.abs(), a2.abs())
                    } else {
                        (a2.abs(), a1.abs())
                    }
                }
            } else {
                (0.0, 0.0)
            }
        }

        // o1           |
        //      .-------´
        // o2   |
        //
        //      <---d--->
        let d = (left + right + 1) as f32;

        let o1 = 0.5 + offset;
        let o2 = 0.5 + offset - 1.0;

        match pattern {
            0 => {
                //
                //    ------
                //
                (0.0, 0.0)
            }
            1 => {
                //
                //   .------
                //   |
                //
                // We only offset L patterns in the crossing edge side, to make it
                // converge with the unfiltered pattern 0 (we don't want to filter the
                // pattern 0 to avoid artifacts).
                if left <= right {
                    area((0.0, o2), (d / 2.0, 0.0), left)
                } else {
                    (0.0, 0.0)
                }
            }
            2 => {
                //
                //    ------.
                //          |
                if left >= right {
                    area((d / 2.0, 0.0), (d, o2), left)
                } else {
                    (0.0, 0.0)
                }
            }
            3 => {
                let a1 = Vec2::from(area((0.0, o2), (d / 2.0, 0.0), left));
                let a2 = Vec2::from(area((d / 2.0, 0.0), (d, o2), left));
                let (a1, a2) = smooth_area(d, a1, a2);
                (a1 + a2).into()
            }
            4 => {
                //   |
                //   `------
                //
                if left <= right {
                    area((0.0, o1), (d / 2.0, 0.0), left)
                } else {
                    (0.0, 0.0)
                }
            }
            5 => {
                //   |
                //   +------
                //   |
                (0.0, 0.0)
            }
            6 => {
                //   |
                //   `------.
                //          |
                //
                // A problem of not offsetting L patterns (see above), is that for certain
                // max search distances, the pixels in the center of a Z pattern will
                // detect the full Z pattern, while the pixels in the sides will detect a
                // L pattern. To avoid discontinuities, we blend the full offsetted Z
                // revectorization with partially offsetted L patterns.
                if offset.abs() > 0.0 {
                    let a1 = Vec2::from(area((0.0, o1), (d, o2), left));
                    let a2 = Vec2::from(area((0.0, o1), (d / 2.0, 0.0), left))
                        + Vec2::from(area((d / 2.0, 0.0), (d, o2), left));
                    ((a1 + a2) / 2.0).into()
                } else {
                    area((0.0, o1), (d, o2), left)
                }
            }
            7 => {
                //   |
                //   +------.
                //   |      |
                area((0.0, o1), (d, o2), left)
            }
            8 => {
                //          |
                //    ------´
                //
                if left >= right {
                    area((d / 2.0, 0.0), (d, o1), left)
                } else {
                    (0.0, 0.0)
                }
            }
            9 => {
                //          |
                //   .------´
                //   |
                if offset.abs() > 0.0 {
                    let a1 = Vec2::from(area((0.0, o2), (d, o1), left));
                    let a2 = Vec2::from(area((0.0, o2), (d / 2.0, 0.0), left))
                        + Vec2::from(area((d / 2.0, 0.0), (d, o1), left));
                    ((a1 + a2) / 2.0).into()
                } else {
                    area((0.0, o2), (d, o1), left)
                }
            }
            10 => {
                //          |
                //    ------+
                //          |
                (0.0, 0.0)
            }
            11 => {
                //          |
                //   .------+
                //   |      |
                area((0.0, o2), (d, o1), left)
            }
            12 => {
                //   |      |
                //   `------´
                //
                let a1 = Vec2::from(area((0.0, o1), (d / 2.0, 0.0), left));
                let a2 = Vec2::from(area((d / 2.0, 0.0), (d, o1), left));
                let (a1, a2) = smooth_area(d, a1, a2);
                (a1 + a2).into()
            }
            13 => {
                //   |      |
                //   +------´
                //   |
                area((0.0, o2), (d, o1), left)
            }
            14 => {
                //   |      |
                //   `------+
                //          |
                area((0.0, o1), (d, o2), left)
            }
            15 => {
                //   |      |
                //   +------+
                //   |      |
                (0.0, 0.0)
            }
            _ => unimplemented!("Patterns should be in range 0..16"),
        }
    }

    //------------------------------------------------------------------------------
    // Diagonal Areas

    // Calculates the area for a given pattern and distances to the left and to the
    // right, biased by an offset
    fn area_diag(pattern: usize, left: f32, right: f32, offset: (f32, f32)) -> Vec2 {
        // Calculates the area under the line p1->p2 for the pixel 'p' using brute
        // force sampling:
        // (quick and dirty solution, but it works)
        fn area_brute(p1: Vec2, p2: Vec2, p: Vec2) -> f32 {
            let inside = |Vec2 { x, y }| {
                if p1 != p2 {
                    let Vec2 { x: xm, y: ym } = (p1 + p2) / 2.0;
                    let a = p2.y - p1.y;
                    let b = p1.x - p2.x;
                    let c = a * (x - xm) + b * (y - ym);
                    c > 0.0
                } else {
                    true
                }
            };
            let mut a = 0.0;
            for x in 0..SAMPLES_DIAG {
                for y in 0..SAMPLES_DIAG {
                    let o = Vec2 {
                        x: x as f32,
                        y: y as f32,
                    } / (SAMPLES_DIAG - 1) as f32;
                    a += if inside(p + o) { 1.0 } else { 0.0 };
                }
            }
            a / SAMPLES_DIAG.pow(2) as f32
        }

        // Calculates the area under the line p1->p2:
        // (includes the pixel and its opposite)
        let area = |mut p1, mut p2| {
            let (e1, e2) = EDGES_DIAG[pattern];
            if e1 > 0 {
                p1 += Vec2::from(offset);
            }
            if e2 > 0 {
                p2 += Vec2::from(offset);
            }
            let a1 = area_brute(p1, p2, Vec2::X + Vec2::splat(left));
            let a2 = area_brute(p1, p2, Vec2::ONE + Vec2::splat(left));
            Vec2::new(1.0 - a1, a2)
        };

        let d = Vec2::splat(left + right + 1.0);
        match pattern {
            0 => {
                // There is some Black Magic around diagonal area calculations. Unlike
                // orthogonal patterns, the 'null' pattern (one without crossing edges) must be
                // filtered, and the ends of both the 'null' and L patterns are not known: L
                // and U patterns have different endings, and we don't know what is the
                // adjacent pattern. So, what we do is calculate a blend of both possibilities.
                //
                //         .-´
                //       .-´
                //     .-´
                //   .-´
                //   ´
                //
                let a1 = area(Vec2::ONE, Vec2::ONE + d); // 1st possibility
                let a2 = area(Vec2::X, Vec2::X + d); // 2st possibility
                (a1 + a2) / 2.0
            }
            1 => {
                //
                //         .-´
                //       .-´
                //     .-´
                //   .-´
                //   |
                //   |
                let a1 = area(Vec2::X, Vec2::ZERO + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            2 => {
                //
                //         .----
                //       .-´
                //     .-´
                //   .-´
                //   ´
                //
                let a1 = area(Vec2::ZERO, Vec2::X + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            3 => {
                //
                //         .----
                //       .-´
                //     .-´
                //   .-´
                //   |
                //   |
                area(Vec2::X, Vec2::X + d)
            }
            4 => {
                //
                //         .-´
                //       .-´
                //     .-´
                // ----´
                //
                //
                let a1 = area(Vec2::ONE, Vec2::ZERO + d);
                let a2 = area(Vec2::ONE, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            5 => {
                //
                //         .-´
                //       .-´
                //     .-´
                // --.-´
                //   |
                //   |
                let a1 = area(Vec2::ONE, Vec2::ZERO + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            6 => {
                //
                //         .----
                //       .-´
                //     .-´
                // ----´
                //
                //
                area(Vec2::ONE, Vec2::X + d)
            }
            7 => {
                //
                //         .----
                //       .-´
                //     .-´
                // --.-´
                //   |
                //   |
                let a1 = area(Vec2::ONE, Vec2::X + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            8 => {
                //         |
                //         |
                //       .-´
                //     .-´
                //   .-´
                //   ´
                //
                let a1 = area(Vec2::ZERO, Vec2::ONE + d);
                let a2 = area(Vec2::X, Vec2::ONE + d);
                (a1 + a2) / 2.0
            }
            9 => {
                //         |
                //         |
                //       .-´
                //     .-´
                //   .-´
                //   |
                //   |
                area(Vec2::X, Vec2::ONE + d)
            }
            10 => {
                //         |
                //         .----
                //       .-´
                //     .-´
                //   .-´
                //   ´
                //
                let a1 = area(Vec2::ZERO, Vec2::ONE + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            11 => {
                //         |
                //         .----
                //       .-´
                //     .-´
                //   .-´
                //   |
                //   |
                let a1 = area(Vec2::X, Vec2::ONE + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            12 => {
                //         |
                //         |
                //       .-´
                //     .-´
                // ----´
                //
                //
                area(Vec2::ONE, Vec2::ONE + d)
            }
            13 => {
                //         |
                //         |
                //       .-´
                //     .-´
                // --.-´
                //   |
                //   |
                let a1 = area(Vec2::ONE, Vec2::ONE + d);
                let a2 = area(Vec2::X, Vec2::ONE + d);
                (a1 + a2) / 2.0
            }
            14 => {
                //         |
                //         .----
                //       .-´
                //     .-´
                // ----´
                //
                //
                let a1 = area(Vec2::ONE, Vec2::ONE + d);
                let a2 = area(Vec2::ONE, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            15 => {
                //         |
                //         .----
                //       .-´
                //     .-´
                // --.-´
                //   |
                //   |
                let a1 = area(Vec2::ONE, Vec2::ONE + d);
                let a2 = area(Vec2::X, Vec2::X + d);
                (a1 + a2) / 2.0
            }
            _ => unimplemented!("Patterns should be in range 0..16"),
        }
    }

    for (subsample_index, &offset) in SUBSAMPLE_OFFSETS_ORTHO.iter().enumerate() {
        for (pattern, edge) in EDGES_ORTHO.iter().enumerate() {
            let subsample_pos = (0, 5 * SIZE_ORTHO * subsample_index);
            for y in 0..SIZE_ORTHO {
                let global_y = subsample_pos.1 + y + SIZE_ORTHO * edge.1;
                for x in 0..SIZE_ORTHO {
                    let pixel = area_ortho(pattern, x.pow(2), y.pow(2), offset);
                    let global_x = subsample_pos.0 + x + SIZE_ORTHO * edge.0;
                    let index = global_y * ROW_LENGTH + global_x * PIXEL_SIZE;
                    data[index] = (pixel.0 * 255.0) as u8;
                    data[index + 1] = (pixel.1 * 255.0) as u8;
                }
            }
        }
    }

    for (subsample_index, &offset) in SUBSAMPLE_OFFSETS_DIAG.iter().enumerate() {
        for (pattern, edge) in EDGES_DIAG.iter().enumerate() {
            let subsample_pos = (5 * SIZE_ORTHO, 4 * SIZE_DIAG * subsample_index);
            for y in 0..SIZE_DIAG {
                let global_y = subsample_pos.1 + y + SIZE_DIAG * edge.1;
                for x in 0..SIZE_DIAG {
                    let pixel = area_diag(pattern, x as f32, y as f32, offset);
                    let global_x = subsample_pos.0 + x + SIZE_DIAG * edge.0;
                    let index = global_y * ROW_LENGTH + global_x * PIXEL_SIZE;
                    data[index] = (pixel.x * 255.0) as u8;
                    data[index + 1] = (pixel.y * 255.0) as u8;
                }
            }
        }
    }

    data
}

/// Generates texture, that allows to know how many pixels we must advance in the last step
/// of SMAA line search algorithm, with single fetch
/// based on Jorge Jimenez code <https://github.com/iryoku/smaa/blob/master/Scripts/SearchTex.py>
pub fn search_data() -> Vec<u8> {
    // Calculates the bilinear fetch for a certain edge combination
    // e[0]       e[1]
    //
    //          x <-------- Sample position:    (-0.25,-0.125)
    // e[2]       e[3] <--- Current pixel [3]:  (  0.0, 0.0  )
    fn bilinear(e: (u8, u8, u8, u8)) -> f32 {
        use bevy_math::FloatExt;
        let up = f32::lerp(e.0 as f32, e.1 as f32, 1.0 - 0.25);
        let down = f32::lerp(e.2 as f32, e.3 as f32, 1.0 - 0.25);
        f32::lerp(up, down, 1.0 - 0.125)
    }

    // This map returns which edges are active for a certain bilinear fetch
    // (it's the reverse lookup of the bilinear function)
    let mut edges = Vec::new();

    for a in 0..=1 {
        for b in 0..=1 {
            for c in 0..=1 {
                for d in 0..=1 {
                    let edge_combination = (a, b, c, d);
                    edges.push((bilinear(edge_combination), edge_combination));
                }
            }
        }
    }

    let get_edge = |bilinear| {
        edges
            .iter()
            .find_map(|&(key, value)| if key == bilinear { Some(value) } else { None })
    };

    // Delta distance to add in the last step of searches to the left
    fn delta_left(left: (u8, u8, u8, u8), top: (u8, u8, u8, u8)) -> u8 {
        // If there is an edge, continue
        if top.3 == 1 {
            // If we previously found and edge, there is another edge
            // and no crossing edges, continue
            if top.2 == 1 && left.1 != 1 && left.3 != 1 {
                2
            } else {
                1
            }
        } else {
            0
        }
    }

    // Delta distance to add in the last step of searches to the right
    fn delta_right(left: (u8, u8, u8, u8), top: (u8, u8, u8, u8)) -> u8 {
        // If there is an edge, and no crossing edges, continue
        if top.3 == 1 && left.1 != 1 && left.3 != 1 {
            // If we previously found and edge, there is another edge
            // and no crossing edges, continue
            if top.2 == 1 && left.0 != 1 && left.2 != 1 {
                2
            } else {
                1
            }
        } else {
            0
        }
    }

    let mut data = vec![0u8; 1024];

    for x in 0..=32 {
        for y in 0..16 {
            if let (Some(x_edge), Some(y_edge)) = (
                get_edge(0.03125 * x as f32),
                get_edge(0.03125 * (32 - y) as f32),
            ) {
                data[x + y * 64] = 127 * delta_left(x_edge, y_edge);
                if x < 31 {
                    data[x + 33 + y * 64] = 127 * delta_right(x_edge, y_edge);
                }
            }
        }
    }

    data
}
