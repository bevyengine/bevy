use crate::{Indices, Mesh, MeshBuilder, Meshable};
use bevy_asset::RenderAssetUsages;
use bevy_math::{ops, primitives::Capsule3d, Vec2, Vec3};
use wgpu::PrimitiveTopology;

/// Manner in which UV coordinates are distributed vertically.
#[derive(Clone, Copy, Debug, Default)]
pub enum CapsuleUvProfile {
    /// UV space is distributed by how much of the capsule consists of the hemispheres.
    #[default]
    Aspect,
    /// Hemispheres get UV space according to the ratio of latitudes to rings.
    Uniform,
    /// Upper third of the texture goes to the northern hemisphere, middle third to the cylinder
    /// and lower third to the southern one.
    Fixed,
}

/// A builder used for creating a [`Mesh`] with a [`Capsule3d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct Capsule3dMeshBuilder {
    /// The [`Capsule3d`] shape.
    pub capsule: Capsule3d,
    /// The number of horizontal lines subdividing the cylindrical part of the capsule.
    /// The default is `0`.
    pub rings: u32,
    /// The number of vertical lines subdividing the hemispheres of the capsule.
    /// The default is `32`.
    pub longitudes: u32,
    /// The number of horizontal lines subdividing the hemispheres of the capsule.
    /// The default is `16`.
    pub latitudes: u32,
    /// The manner in which UV coordinates are distributed vertically.
    /// The default is [`CapsuleUvProfile::Aspect`].
    pub uv_profile: CapsuleUvProfile,
}

impl Default for Capsule3dMeshBuilder {
    fn default() -> Self {
        Self {
            capsule: Capsule3d::default(),
            rings: 0,
            longitudes: 32,
            latitudes: 16,
            uv_profile: CapsuleUvProfile::default(),
        }
    }
}

impl Capsule3dMeshBuilder {
    /// Creates a new [`Capsule3dMeshBuilder`] from a given radius, height, longitudes, and latitudes.
    ///
    /// Note that `height` is the distance between the centers of the hemispheres.
    /// `radius` will be added to both ends to get the real height of the mesh.
    #[inline]
    pub fn new(radius: f32, height: f32, longitudes: u32, latitudes: u32) -> Self {
        Self {
            capsule: Capsule3d::new(radius, height),
            longitudes,
            latitudes,
            ..Default::default()
        }
    }

    /// Sets the number of horizontal lines subdividing the cylindrical part of the capsule.
    #[inline]
    pub const fn rings(mut self, rings: u32) -> Self {
        self.rings = rings;
        self
    }

    /// Sets the number of vertical lines subdividing the hemispheres of the capsule.
    #[inline]
    pub const fn longitudes(mut self, longitudes: u32) -> Self {
        self.longitudes = longitudes;
        self
    }

    /// Sets the number of horizontal lines subdividing the hemispheres of the capsule.
    #[inline]
    pub const fn latitudes(mut self, latitudes: u32) -> Self {
        self.latitudes = latitudes;
        self
    }

    /// Sets the manner in which UV coordinates are distributed vertically.
    #[inline]
    pub const fn uv_profile(mut self, uv_profile: CapsuleUvProfile) -> Self {
        self.uv_profile = uv_profile;
        self
    }
}

impl MeshBuilder for Capsule3dMeshBuilder {
    fn build(&self) -> Mesh {
        // code adapted from https://behreajj.medium.com/making-a-capsule-mesh-via-script-in-five-3d-environments-c2214abf02db
        let Capsule3dMeshBuilder {
            capsule,
            rings,
            longitudes,
            latitudes,
            uv_profile,
        } = *self;
        let Capsule3d {
            radius,
            half_length,
        } = capsule;

        let calc_middle = rings > 0;
        let half_latitudes = latitudes / 2;
        let half_latitudes_n1 = half_latitudes - 1;
        let half_latitudes_n2 = half_latitudes - 2;
        let rings_p1 = rings + 1;
        let longitudes_p1 = longitudes + 1;
        let summit = half_length + radius;

        // Vertex index offsets.
        let vert_offset_north_hemisphere = longitudes;
        let vert_offset_north_equator =
            vert_offset_north_hemisphere + longitudes_p1 * half_latitudes_n1;
        let vert_offset_cylinder = vert_offset_north_equator + longitudes_p1;
        let vert_offset_south_equator = if calc_middle {
            vert_offset_cylinder + longitudes_p1 * rings
        } else {
            vert_offset_cylinder
        };
        let vert_offset_south_hemisphere = vert_offset_south_equator + longitudes_p1;
        let vert_offset_south_polar =
            vert_offset_south_hemisphere + longitudes_p1 * half_latitudes_n2;
        let vert_offset_south_cap = vert_offset_south_polar + longitudes_p1;

        // Initialize arrays.
        let vert_len = (vert_offset_south_cap + longitudes) as usize;

        let mut vs: Vec<Vec3> = vec![Vec3::ZERO; vert_len];
        let mut vts: Vec<Vec2> = vec![Vec2::ZERO; vert_len];
        let mut vns: Vec<Vec3> = vec![Vec3::ZERO; vert_len];

        let to_theta = 2.0 * core::f32::consts::PI / longitudes as f32;
        let to_phi = core::f32::consts::PI / latitudes as f32;
        let to_texture_horizontal = 1.0 / longitudes as f32;
        let to_texture_vertical = 1.0 / half_latitudes as f32;

        let vt_aspect_ratio = match uv_profile {
            CapsuleUvProfile::Aspect => radius / (2.0 * half_length + radius + radius),
            CapsuleUvProfile::Uniform => half_latitudes as f32 / (rings_p1 + latitudes) as f32,
            CapsuleUvProfile::Fixed => 1.0 / 3.0,
        };
        let vt_aspect_north = 1.0 - vt_aspect_ratio;
        let vt_aspect_south = vt_aspect_ratio;

        let mut theta_cartesian: Vec<Vec2> = vec![Vec2::ZERO; longitudes as usize];
        let mut rho_theta_cartesian: Vec<Vec2> = vec![Vec2::ZERO; longitudes as usize];
        let mut south_texture_cache: Vec<f32> = vec![0.0; longitudes_p1 as usize];

        for j in 0..longitudes as usize {
            let jf = j as f32;
            let south_texture_polar = 1.0 - ((jf + 0.5) * to_texture_horizontal);
            let theta = jf * to_theta;

            theta_cartesian[j] = Vec2::from_angle(theta);
            rho_theta_cartesian[j] = radius * theta_cartesian[j];

            // North.
            vs[j] = Vec3::new(0.0, summit, 0.0);
            vts[j] = Vec2::new(south_texture_polar, 1.0);
            vns[j] = Vec3::Y;

            // South.
            let idx = vert_offset_south_cap as usize + j;
            vs[idx] = Vec3::new(0.0, -summit, 0.0);
            vts[idx] = Vec2::new(south_texture_polar, 0.0);
            vns[idx] = Vec3::new(0.0, -1.0, 0.0);
        }

        // Equatorial vertices.
        for (j, south_texture_cache_j) in south_texture_cache
            .iter_mut()
            .enumerate()
            .take(longitudes_p1 as usize)
        {
            let south_texture = 1.0 - j as f32 * to_texture_horizontal;
            *south_texture_cache_j = south_texture;

            // Wrap to first element upon reaching last.
            let j_mod = j % longitudes as usize;
            let tc = theta_cartesian[j_mod];
            let rtc = rho_theta_cartesian[j_mod];

            // North equator.
            let index_north = vert_offset_north_equator as usize + j;
            vs[index_north] = Vec3::new(rtc.x, half_length, -rtc.y);
            vts[index_north] = Vec2::new(south_texture, vt_aspect_north);
            vns[index_north] = Vec3::new(tc.x, 0.0, -tc.y);

            // South equator.
            let index_south = vert_offset_south_equator as usize + j;
            vs[index_south] = Vec3::new(rtc.x, -half_length, -rtc.y);
            vts[index_south] = Vec2::new(south_texture, vt_aspect_south);
            vns[index_south] = Vec3::new(tc.x, 0.0, -tc.y);
        }

        // Hemisphere vertices.
        for i in 0..half_latitudes_n1 {
            let i_plus1 = i as f32 + 1.0;
            let phi = i_plus1 * to_phi;

            // For coordinates.
            let (sin_phi_south, cos_phi_south) = ops::sin_cos(phi);

            // Symmetrical hemispheres mean cosine and sine only needs
            // to be calculated once.
            let cos_phi_north = sin_phi_south;
            let sin_phi_north = -cos_phi_south;

            let rho_cos_phi_north = radius * cos_phi_north;
            let rho_sin_phi_north = radius * sin_phi_north;
            let z_offset_north = half_length - rho_sin_phi_north;

            let rho_cos_phi_south = radius * cos_phi_south;
            let rho_sin_phi_south = radius * sin_phi_south;
            let z_offset_south = -half_length - rho_sin_phi_south;

            // For texture coordinates.
            let to_texture_factor = i_plus1 * to_texture_vertical;
            let complement_texture_factor = 1.0 - to_texture_factor;
            let t_texture_north = complement_texture_factor + vt_aspect_north * to_texture_factor;
            let t_texture_south = complement_texture_factor * vt_aspect_south;

            let i_longitudes_p1 = i * longitudes_p1;
            let vert_current_lat_north = vert_offset_north_hemisphere + i_longitudes_p1;
            let vert_current_lat_south = vert_offset_south_hemisphere + i_longitudes_p1;

            for (j, south_texture) in south_texture_cache
                .iter()
                .enumerate()
                .take(longitudes_p1 as usize)
            {
                let j_mod = j % longitudes as usize;

                let tc = theta_cartesian[j_mod];

                // North hemisphere.
                let index_north = vert_current_lat_north as usize + j;
                vs[index_north] = Vec3::new(
                    rho_cos_phi_north * tc.x,
                    z_offset_north,
                    -rho_cos_phi_north * tc.y,
                );
                vts[index_north] = Vec2::new(*south_texture, t_texture_north);
                vns[index_north] =
                    Vec3::new(cos_phi_north * tc.x, -sin_phi_north, -cos_phi_north * tc.y);

                // South hemisphere.
                let index_south = vert_current_lat_south as usize + j;
                vs[index_south] = Vec3::new(
                    rho_cos_phi_south * tc.x,
                    z_offset_south,
                    -rho_cos_phi_south * tc.y,
                );
                vts[index_south] = Vec2::new(*south_texture, t_texture_south);
                vns[index_south] =
                    Vec3::new(cos_phi_south * tc.x, -sin_phi_south, -cos_phi_south * tc.y);
            }
        }

        // Cylinder vertices.
        if calc_middle {
            // Exclude both origin and destination edges
            // (North and South equators) from the interpolation.
            let to_factor = 1.0 / rings_p1 as f32;
            let mut idx_cyl_lat = vert_offset_cylinder as usize;

            for h in 1..rings_p1 {
                let factor = h as f32 * to_factor;
                let complement_factor = 1.0 - factor;
                let t_texture = complement_factor * vt_aspect_north + factor * vt_aspect_south;
                let z = half_length - 2.0 * half_length * factor;

                for (j, south_texture) in south_texture_cache
                    .iter()
                    .enumerate()
                    .take(longitudes_p1 as usize)
                {
                    let j_mod = j % longitudes as usize;
                    let tc = theta_cartesian[j_mod];
                    let rtc = rho_theta_cartesian[j_mod];

                    vs[idx_cyl_lat] = Vec3::new(rtc.x, z, -rtc.y);
                    vts[idx_cyl_lat] = Vec2::new(*south_texture, t_texture);
                    vns[idx_cyl_lat] = Vec3::new(tc.x, 0.0, -tc.y);

                    idx_cyl_lat += 1;
                }
            }
        }

        // Triangle indices.

        // Stride is 3 for polar triangles;
        // stride is 6 for two triangles forming a quad.
        let longitudes3 = longitudes * 3;
        let longitudes6 = longitudes * 6;
        let hemisphere_longitudes = half_latitudes_n1 * longitudes6;

        let tri_offset_north_hemisphere = longitudes3;
        let tri_offset_cylinder = tri_offset_north_hemisphere + hemisphere_longitudes;
        let tri_offset_south_hemisphere = tri_offset_cylinder + rings_p1 * longitudes6;
        let tri_offset_south_cap = tri_offset_south_hemisphere + hemisphere_longitudes;

        let fs_len = tri_offset_south_cap + longitudes3;
        let mut triangles: Vec<u32> = vec![0; fs_len as usize];

        // Polar caps.
        let mut i = 0;
        let mut k = 0;
        let mut m = tri_offset_south_cap as usize;
        while i < longitudes {
            // North.
            triangles[k] = i;
            triangles[k + 1] = vert_offset_north_hemisphere + i;
            triangles[k + 2] = vert_offset_north_hemisphere + i + 1;

            // South.
            triangles[m] = vert_offset_south_cap + i;
            triangles[m + 1] = vert_offset_south_polar + i + 1;
            triangles[m + 2] = vert_offset_south_polar + i;

            i += 1;
            k += 3;
            m += 3;
        }

        // Hemispheres.

        let mut i = 0;
        let mut k = tri_offset_north_hemisphere as usize;
        let mut m = tri_offset_south_hemisphere as usize;

        while i < half_latitudes_n1 {
            let i_longitudes_p1 = i * longitudes_p1;

            let vert_current_lat_north = vert_offset_north_hemisphere + i_longitudes_p1;
            let vert_next_lat_north = vert_current_lat_north + longitudes_p1;

            let vert_current_lat_south = vert_offset_south_equator + i_longitudes_p1;
            let vert_next_lat_south = vert_current_lat_south + longitudes_p1;

            let mut j = 0;
            while j < longitudes {
                // North.
                let north00 = vert_current_lat_north + j;
                let north01 = vert_next_lat_north + j;
                let north11 = vert_next_lat_north + j + 1;
                let north10 = vert_current_lat_north + j + 1;

                triangles[k] = north00;
                triangles[k + 1] = north11;
                triangles[k + 2] = north10;

                triangles[k + 3] = north00;
                triangles[k + 4] = north01;
                triangles[k + 5] = north11;

                // South.
                let south00 = vert_current_lat_south + j;
                let south01 = vert_next_lat_south + j;
                let south11 = vert_next_lat_south + j + 1;
                let south10 = vert_current_lat_south + j + 1;

                triangles[m] = south00;
                triangles[m + 1] = south11;
                triangles[m + 2] = south10;

                triangles[m + 3] = south00;
                triangles[m + 4] = south01;
                triangles[m + 5] = south11;

                j += 1;
                k += 6;
                m += 6;
            }

            i += 1;
        }

        // Cylinder.
        let mut i = 0;
        let mut k = tri_offset_cylinder as usize;

        while i < rings_p1 {
            let vert_current_lat = vert_offset_north_equator + i * longitudes_p1;
            let vert_next_lat = vert_current_lat + longitudes_p1;

            let mut j = 0;
            while j < longitudes {
                let cy00 = vert_current_lat + j;
                let cy01 = vert_next_lat + j;
                let cy11 = vert_next_lat + j + 1;
                let cy10 = vert_current_lat + j + 1;

                triangles[k] = cy00;
                triangles[k + 1] = cy11;
                triangles[k + 2] = cy10;

                triangles[k + 3] = cy00;
                triangles[k + 4] = cy01;
                triangles[k + 5] = cy11;

                j += 1;
                k += 6;
            }

            i += 1;
        }

        let vs: Vec<[f32; 3]> = vs.into_iter().map(Into::into).collect();
        let vns: Vec<[f32; 3]> = vns.into_iter().map(Into::into).collect();
        let vts: Vec<[f32; 2]> = vts.into_iter().map(Into::into).collect();

        assert_eq!(vs.len(), vert_len);
        assert_eq!(triangles.len(), fs_len as usize);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vns)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vts)
        .with_inserted_indices(Indices::U32(triangles))
    }
}

impl Meshable for Capsule3d {
    type Output = Capsule3dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Capsule3dMeshBuilder {
            capsule: *self,
            ..Default::default()
        }
    }
}

impl From<Capsule3d> for Mesh {
    fn from(capsule: Capsule3d) -> Self {
        capsule.mesh().build()
    }
}
