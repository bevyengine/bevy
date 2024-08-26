use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};
use bevy_math::{primitives::Capsule3d, Vec2, Vec3};
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
        let half_lats = latitudes / 2;
        let half_latsn1 = half_lats - 1;
        let half_latsn2 = half_lats - 2;
        let ringsp1 = rings + 1;
        let lonsp1 = longitudes + 1;
        let summit = half_length + radius;

        // Vertex index offsets.
        let vert_offset_north_hemi = longitudes;
        let vert_offset_north_equator = vert_offset_north_hemi + lonsp1 * half_latsn1;
        let vert_offset_cylinder = vert_offset_north_equator + lonsp1;
        let vert_offset_south_equator = if calc_middle {
            vert_offset_cylinder + lonsp1 * rings
        } else {
            vert_offset_cylinder
        };
        let vert_offset_south_hemi = vert_offset_south_equator + lonsp1;
        let vert_offset_south_polar = vert_offset_south_hemi + lonsp1 * half_latsn2;
        let vert_offset_south_cap = vert_offset_south_polar + lonsp1;

        // Initialize arrays.
        let vert_len = (vert_offset_south_cap + longitudes) as usize;

        let mut vs: Vec<Vec3> = vec![Vec3::ZERO; vert_len];
        let mut vts: Vec<Vec2> = vec![Vec2::ZERO; vert_len];
        let mut vns: Vec<Vec3> = vec![Vec3::ZERO; vert_len];

        let to_theta = 2.0 * std::f32::consts::PI / longitudes as f32;
        let to_phi = std::f32::consts::PI / latitudes as f32;
        let to_tex_horizontal = 1.0 / longitudes as f32;
        let to_tex_vertical = 1.0 / half_lats as f32;

        let vt_aspect_ratio = match uv_profile {
            CapsuleUvProfile::Aspect => radius / (2.0 * half_length + radius + radius),
            CapsuleUvProfile::Uniform => half_lats as f32 / (ringsp1 + latitudes) as f32,
            CapsuleUvProfile::Fixed => 1.0 / 3.0,
        };
        let vt_aspect_north = 1.0 - vt_aspect_ratio;
        let vt_aspect_south = vt_aspect_ratio;

        let mut theta_cartesian: Vec<Vec2> = vec![Vec2::ZERO; longitudes as usize];
        let mut rho_theta_cartesian: Vec<Vec2> = vec![Vec2::ZERO; longitudes as usize];
        let mut s_texture_cache: Vec<f32> = vec![0.0; lonsp1 as usize];

        for j in 0..longitudes as usize {
            let jf = j as f32;
            let s_texture_polar = 1.0 - ((jf + 0.5) * to_tex_horizontal);
            let theta = jf * to_theta;

            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            theta_cartesian[j] = Vec2::new(cos_theta, sin_theta);
            rho_theta_cartesian[j] = Vec2::new(radius * cos_theta, radius * sin_theta);

            // North.
            vs[j] = Vec3::new(0.0, summit, 0.0);
            vts[j] = Vec2::new(s_texture_polar, 1.0);
            vns[j] = Vec3::Y;

            // South.
            let idx = vert_offset_south_cap as usize + j;
            vs[idx] = Vec3::new(0.0, -summit, 0.0);
            vts[idx] = Vec2::new(s_texture_polar, 0.0);
            vns[idx] = Vec3::new(0.0, -1.0, 0.0);
        }

        // Equatorial vertices.
        for (j, s_texture_cache_j) in s_texture_cache.iter_mut().enumerate().take(lonsp1 as usize) {
            let s_texture = 1.0 - j as f32 * to_tex_horizontal;
            *s_texture_cache_j = s_texture;

            // Wrap to first element upon reaching last.
            let j_mod = j % longitudes as usize;
            let tc = theta_cartesian[j_mod];
            let rtc = rho_theta_cartesian[j_mod];

            // North equator.
            let idxn = vert_offset_north_equator as usize + j;
            vs[idxn] = Vec3::new(rtc.x, half_length, -rtc.y);
            vts[idxn] = Vec2::new(s_texture, vt_aspect_north);
            vns[idxn] = Vec3::new(tc.x, 0.0, -tc.y);

            // South equator.
            let idxs = vert_offset_south_equator as usize + j;
            vs[idxs] = Vec3::new(rtc.x, -half_length, -rtc.y);
            vts[idxs] = Vec2::new(s_texture, vt_aspect_south);
            vns[idxs] = Vec3::new(tc.x, 0.0, -tc.y);
        }

        // Hemisphere vertices.
        for i in 0..half_latsn1 {
            let ip1f = i as f32 + 1.0;
            let phi = ip1f * to_phi;

            // For coordinates.
            let cos_phi_south = phi.cos();
            let sin_phi_south = phi.sin();

            // Symmetrical hemispheres mean cosine and sine only needs
            // to be calculated once.
            let cos_phi_north = sin_phi_south;
            let sin_phi_north = -cos_phi_south;

            let rho_cos_phi_north = radius * cos_phi_north;
            let rho_sin_phi_north = radius * sin_phi_north;
            let z_offset_north = half_length - rho_sin_phi_north;

            let rho_cos_phi_south = radius * cos_phi_south;
            let rho_sin_phi_south = radius * sin_phi_south;
            let z_offset_sout = -half_length - rho_sin_phi_south;

            // For texture coordinates.
            let t_tex_fac = ip1f * to_tex_vertical;
            let cmpl_tex_fac = 1.0 - t_tex_fac;
            let t_tex_north = cmpl_tex_fac + vt_aspect_north * t_tex_fac;
            let t_tex_south = cmpl_tex_fac * vt_aspect_south;

            let i_lonsp1 = i * lonsp1;
            let vert_curr_lat_north = vert_offset_north_hemi + i_lonsp1;
            let vert_curr_lat_south = vert_offset_south_hemi + i_lonsp1;

            for (j, s_texture) in s_texture_cache.iter().enumerate().take(lonsp1 as usize) {
                let j_mod = j % longitudes as usize;

                let tc = theta_cartesian[j_mod];

                // North hemisphere.
                let idxn = vert_curr_lat_north as usize + j;
                vs[idxn] = Vec3::new(
                    rho_cos_phi_north * tc.x,
                    z_offset_north,
                    -rho_cos_phi_north * tc.y,
                );
                vts[idxn] = Vec2::new(*s_texture, t_tex_north);
                vns[idxn] = Vec3::new(cos_phi_north * tc.x, -sin_phi_north, -cos_phi_north * tc.y);

                // South hemisphere.
                let idxs = vert_curr_lat_south as usize + j;
                vs[idxs] = Vec3::new(
                    rho_cos_phi_south * tc.x,
                    z_offset_sout,
                    -rho_cos_phi_south * tc.y,
                );
                vts[idxs] = Vec2::new(*s_texture, t_tex_south);
                vns[idxs] = Vec3::new(cos_phi_south * tc.x, -sin_phi_south, -cos_phi_south * tc.y);
            }
        }

        // Cylinder vertices.
        if calc_middle {
            // Exclude both origin and destination edges
            // (North and South equators) from the interpolation.
            let to_fac = 1.0 / ringsp1 as f32;
            let mut idx_cyl_lat = vert_offset_cylinder as usize;

            for h in 1..ringsp1 {
                let fac = h as f32 * to_fac;
                let cmpl_fac = 1.0 - fac;
                let t_texture = cmpl_fac * vt_aspect_north + fac * vt_aspect_south;
                let z = half_length - 2.0 * half_length * fac;

                for (j, s_texture) in s_texture_cache.iter().enumerate().take(lonsp1 as usize) {
                    let j_mod = j % longitudes as usize;
                    let tc = theta_cartesian[j_mod];
                    let rtc = rho_theta_cartesian[j_mod];

                    vs[idx_cyl_lat] = Vec3::new(rtc.x, z, -rtc.y);
                    vts[idx_cyl_lat] = Vec2::new(*s_texture, t_texture);
                    vns[idx_cyl_lat] = Vec3::new(tc.x, 0.0, -tc.y);

                    idx_cyl_lat += 1;
                }
            }
        }

        // Triangle indices.

        // Stride is 3 for polar triangles;
        // stride is 6 for two triangles forming a quad.
        let lons3 = longitudes * 3;
        let lons6 = longitudes * 6;
        let hemi_lons = half_latsn1 * lons6;

        let tri_offset_north_hemi = lons3;
        let tri_offset_cylinder = tri_offset_north_hemi + hemi_lons;
        let tri_offset_south_hemi = tri_offset_cylinder + ringsp1 * lons6;
        let tri_offset_south_cap = tri_offset_south_hemi + hemi_lons;

        let fs_len = tri_offset_south_cap + lons3;
        let mut tris: Vec<u32> = vec![0; fs_len as usize];

        // Polar caps.
        let mut i = 0;
        let mut k = 0;
        let mut m = tri_offset_south_cap as usize;
        while i < longitudes {
            // North.
            tris[k] = i;
            tris[k + 1] = vert_offset_north_hemi + i;
            tris[k + 2] = vert_offset_north_hemi + i + 1;

            // South.
            tris[m] = vert_offset_south_cap + i;
            tris[m + 1] = vert_offset_south_polar + i + 1;
            tris[m + 2] = vert_offset_south_polar + i;

            i += 1;
            k += 3;
            m += 3;
        }

        // Hemispheres.

        let mut i = 0;
        let mut k = tri_offset_north_hemi as usize;
        let mut m = tri_offset_south_hemi as usize;

        while i < half_latsn1 {
            let i_lonsp1 = i * lonsp1;

            let vert_curr_lat_north = vert_offset_north_hemi + i_lonsp1;
            let vert_next_lat_north = vert_curr_lat_north + lonsp1;

            let vert_curr_lat_south = vert_offset_south_equator + i_lonsp1;
            let vert_next_lat_south = vert_curr_lat_south + lonsp1;

            let mut j = 0;
            while j < longitudes {
                // North.
                let north00 = vert_curr_lat_north + j;
                let north01 = vert_next_lat_north + j;
                let north11 = vert_next_lat_north + j + 1;
                let north10 = vert_curr_lat_north + j + 1;

                tris[k] = north00;
                tris[k + 1] = north11;
                tris[k + 2] = north10;

                tris[k + 3] = north00;
                tris[k + 4] = north01;
                tris[k + 5] = north11;

                // South.
                let south00 = vert_curr_lat_south + j;
                let south01 = vert_next_lat_south + j;
                let south11 = vert_next_lat_south + j + 1;
                let south10 = vert_curr_lat_south + j + 1;

                tris[m] = south00;
                tris[m + 1] = south11;
                tris[m + 2] = south10;

                tris[m + 3] = south00;
                tris[m + 4] = south01;
                tris[m + 5] = south11;

                j += 1;
                k += 6;
                m += 6;
            }

            i += 1;
        }

        // Cylinder.
        let mut i = 0;
        let mut k = tri_offset_cylinder as usize;

        while i < ringsp1 {
            let vert_curr_lat = vert_offset_north_equator + i * lonsp1;
            let vert_next_lat = vert_curr_lat + lonsp1;

            let mut j = 0;
            while j < longitudes {
                let cy00 = vert_curr_lat + j;
                let cy01 = vert_next_lat + j;
                let cy11 = vert_next_lat + j + 1;
                let cy10 = vert_curr_lat + j + 1;

                tris[k] = cy00;
                tris[k + 1] = cy11;
                tris[k + 2] = cy10;

                tris[k + 3] = cy00;
                tris[k + 4] = cy01;
                tris[k + 5] = cy11;

                j += 1;
                k += 6;
            }

            i += 1;
        }

        let vs: Vec<[f32; 3]> = vs.into_iter().map(Into::into).collect();
        let vns: Vec<[f32; 3]> = vns.into_iter().map(Into::into).collect();
        let vts: Vec<[f32; 2]> = vts.into_iter().map(Into::into).collect();

        assert_eq!(vs.len(), vert_len);
        assert_eq!(tris.len(), fs_len as usize);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vns)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vts)
        .with_inserted_indices(Indices::U32(tris))
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
