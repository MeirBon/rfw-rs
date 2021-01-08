use rayon::prelude::*;
use rfw_math::*;
use rtbvh::{spatial_sah::SpatialTriangle, Ray, RayPacket4, AABB};
use std::{fmt::Debug, write};

#[derive(Debug, Copy, Clone)]
pub struct SkinData<'a> {
    pub name: &'a str,
    pub inverse_bind_matrices: &'a [Mat4],
    pub joint_matrices: &'a [Mat4],
}

#[derive(Debug, Copy, Clone)]
pub struct InstancesData2D<'a> {
    pub matrices: &'a [Mat4],
    pub mesh_ids: &'a [MeshID],
}

impl InstancesData2D<'_> {
    pub fn len(&self) -> usize {
        debug_assert_eq!(self.matrices.len(), self.mesh_ids.len());
        self.matrices.len()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InstancesData3D<'a> {
    pub matrices: &'a [Mat4],
    pub mesh_ids: &'a [MeshID],
    pub skin_ids: &'a [SkinID],
}

impl InstancesData3D<'_> {
    pub fn len(&self) -> usize {
        debug_assert_eq!(self.matrices.len(), self.mesh_ids.len());
        debug_assert_eq!(self.mesh_ids.len(), self.skin_ids.len());
        self.matrices.len()
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct MeshID(pub i32);

impl std::fmt::Display for MeshID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MeshID {
    pub const INVALID: Self = MeshID(-1);

    pub fn is_valid(&self) -> bool {
        self.0 >= 0
    }

    pub fn as_index(&self) -> Option<usize> {
        if self.0 >= 0 {
            Some(self.0 as usize)
        } else {
            None
        }
    }
}

impl Into<usize> for MeshID {
    fn into(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SkinID(pub i32);

impl std::fmt::Display for SkinID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SkinID {
    pub const INVALID: Self = SkinID(-1);

    pub fn is_valid(&self) -> bool {
        self.0 >= 0
    }

    pub fn as_index(&self) -> Option<usize> {
        if self.0 >= 0 {
            Some(self.0 as usize)
        } else {
            None
        }
    }
}

impl Into<usize> for SkinID {
    fn into(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum DataFormat {
    BGRA8 = 0,
    RGBA8 = 1,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TextureData<'a> {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub bytes: &'a [u8],
    pub format: DataFormat,
}

impl TextureData<'_> {
    pub fn offset_for_level(&self, mip_level: usize) -> usize {
        assert!(mip_level <= self.mip_levels as usize);
        let mut offset = 0;
        for i in 0..mip_level {
            let (w, h) = self.mip_level_width_height(i);
            offset += w * h;
        }
        offset
    }

    pub fn mip_level_width(&self, mip_level: usize) -> usize {
        let mut w = self.width as usize;
        for _ in 0..mip_level {
            w >>= 1;
        }
        w
    }

    pub fn mip_level_height(&self, mip_level: usize) -> usize {
        let mut h = self.height as usize;
        for _ in 0..mip_level {
            h >>= 1;
        }
        h
    }

    pub fn mip_level_width_height(&self, mip_level: usize) -> (usize, usize) {
        let mut w = self.width as usize;
        let mut h = self.height as usize;

        if mip_level == 0 {
            return (w, h);
        }

        for _ in 0..mip_level {
            w >>= 1;
            h >>= 1
        }

        (w, h)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[repr(C)]
pub struct Vertex3D {
    pub vertex: Vec4,
    // 16
    pub normal: Vec3,
    // 28
    pub mat_id: u32,
    // 32
    pub uv: Vec2,
    // 40
    pub tangent: Vec4,
    // 56
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
#[repr(C)]
pub struct JointData {
    pub joint: [u32; 4],
    pub weight: Vec4,
}

impl Into<([u32; 4], Vec4)> for JointData {
    fn into(self) -> ([u32; 4], Vec4) {
        (self.joint, self.weight)
    }
}

impl<T: Into<[f32; 4]>> From<([u32; 4], T)> for JointData {
    fn from(data: ([u32; 4], T)) -> Self {
        Self {
            joint: data.0,
            weight: Vec4::from(data.1.into()),
        }
    }
}

impl<T: Into<[f32; 4]>> From<([u16; 4], T)> for JointData {
    fn from(data: ([u16; 4], T)) -> Self {
        Self {
            joint: [
                data.0[0] as u32,
                data.0[1] as u32,
                data.0[2] as u32,
                data.0[3] as u32,
            ],
            weight: Vec4::from(data.1.into()),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub struct VertexMesh {
    pub first: u32,
    pub last: u32,
    pub mat_id: u32,
    pub bounds: AABB,
}

#[derive(Debug, Clone)]
pub struct MeshData3D<'a> {
    pub name: &'a str,
    pub bounds: AABB,
    pub vertices: &'a [Vertex3D],
    pub triangles: &'a [RTTriangle],
    pub ranges: &'a [VertexMesh],
    pub skin_data: &'a [JointData],
}

impl<'a> MeshData3D<'a> {
    pub fn apply_skin_vertices(&self, skin: &SkinData<'_>) -> SkinnedMesh3D {
        SkinnedMesh3D::apply(self.vertices, self.skin_data, self.ranges, skin)
    }

    pub fn apply_skin_triangles(&self, skin: &SkinData<'_>) -> SkinnedTriangles3D {
        SkinnedTriangles3D::apply(self.triangles, self.skin_data, skin)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Vertex2D {
    pub vertex: [f32; 3],
    pub has_tex: u32,
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct MeshData2D<'a> {
    pub vertices: &'a [Vertex2D],
    pub tex_id: Option<usize>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[repr(C)]
pub struct DeviceMaterial {
    pub color: [f32; 4],
    // 16
    pub absorption: [f32; 4],
    // 32
    pub specular: [f32; 4],
    // 48
    pub parameters: [u32; 4], // 64

    pub flags: u32,
    // 68
    pub diffuse_map: i32,
    // 72
    pub normal_map: i32,
    // 76
    pub metallic_roughness_map: i32, // 80

    pub emissive_map: i32,
    // 84
    pub sheen_map: i32,
    // 88
    pub _dummy: [i32; 2], // 96
}

impl Default for DeviceMaterial {
    fn default() -> Self {
        Self {
            color: [0.0; 4],
            absorption: [0.0; 4],
            specular: [0.0; 4],
            parameters: [0; 4],
            flags: 0,
            diffuse_map: -1,
            normal_map: -1,
            metallic_roughness_map: -1,
            emissive_map: -1,
            sheen_map: -1,
            _dummy: [0; 2],
        }
    }
}

impl DeviceMaterial {
    pub fn get_metallic(&self) -> f32 {
        (self.parameters[0] & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_subsurface(&self) -> f32 {
        ((self.parameters[0].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_specular_f(&self) -> f32 {
        ((self.parameters[0].overflowing_shr(16)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_roughness(&self) -> f32 {
        ((self.parameters[0].overflowing_shr(24)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_specular_tint(&self) -> f32 {
        (self.parameters[1] & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_anisotropic(&self) -> f32 {
        ((self.parameters[1].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_sheen(&self) -> f32 {
        ((self.parameters[1].overflowing_shr(16)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_sheen_tint(&self) -> f32 {
        ((self.parameters[1].overflowing_shr(24)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_clearcoat(&self) -> f32 {
        (self.parameters[2] & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_clearcoat_gloss(&self) -> f32 {
        ((self.parameters[2].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }
    pub fn get_transmission(&self) -> f32 {
        ((self.parameters[2].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_eta(&self) -> f32 {
        ((self.parameters[2].overflowing_shr(24)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_custom0(&self) -> f32 {
        (self.parameters[3] & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_custom1(&self) -> f32 {
        ((self.parameters[3].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_custom2(&self) -> f32 {
        ((self.parameters[3].overflowing_shr(8)).0 & 255) as f32 * 1.0 / 255.0
    }

    pub fn get_custom3(&self) -> f32 {
        ((self.parameters[3].overflowing_shr(24)).0 & 255) as f32 * 1.0 / 255.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CameraView {
    pub pos: [f32; 3],
    // 12
    pub right: [f32; 3],
    // 24
    pub up: [f32; 3],
    // 36
    pub p1: [f32; 3],
    //48
    pub direction: [f32; 3],
    // 60
    pub lens_size: f32,
    // 64
    pub spread_angle: f32,
    pub epsilon: f32,
    pub inv_width: f32,
    pub inv_height: f32,
    // 80
    pub near_plane: f32,
    pub far_plane: f32,
    pub aspect_ratio: f32,
    // FOV in radians
    pub fov: f32,
    // 96
}

#[allow(dead_code)]
impl CameraView {
    pub fn generate_lens_ray(&self, x: u32, y: u32, r0: f32, r1: f32, r2: f32, r3: f32) -> Ray {
        let blade = (r0 * 9.0).round();
        let r2 = (r2 - blade * (1.0 / 9.0)) * 9.0;
        let pi_over_4dot5 = std::f32::consts::PI / 4.5;
        let blade_param = blade * pi_over_4dot5;

        let (x1, y1) = blade_param.sin_cos();
        let blade_param = (blade + 1.0) * pi_over_4dot5;
        let (x2, y2) = blade_param.sin_cos();

        let (r2, r3) = {
            if (r2 + r3) > 1.0 {
                (1.0 - r2, 1.0 - r3)
            } else {
                (r2, r3)
            }
        };

        let xr = x1 * r2 + x2 * r3;
        let yr = y1 * r2 + y2 * r3;

        let origin = Vec3::from(self.pos)
            + self.lens_size * (Vec3::from(self.right) * xr + Vec3::from(self.up) * yr);
        let u = (x as f32 + r0) * self.inv_width;
        let v = (y as f32 + r1) * self.inv_height;
        let point_on_pixel =
            Vec3::from(self.p1) + u * Vec3::from(self.right) + v * Vec3::from(self.up);
        let direction = (point_on_pixel - origin).normalize();

        Ray::new(origin.into(), direction.into())
    }

    pub fn generate_ray(&self, x: u32, y: u32) -> Ray {
        let u = x as f32 * self.inv_width;
        let v = y as f32 * self.inv_height;
        let point_on_pixel =
            Vec3::from(self.p1) + u * Vec3::from(self.right) + v * Vec3::from(self.up);
        let direction = (point_on_pixel - Vec3::from(self.pos)).normalize();

        Ray::new(self.pos.into(), direction.into())
    }

    pub fn generate_lens_ray4(
        &self,
        x: [u32; 4],
        y: [u32; 4],
        r0: [f32; 4],
        r1: [f32; 4],
        r2: [f32; 4],
        r3: [f32; 4],
        width: u32,
    ) -> RayPacket4 {
        let ids = [
            x[0] + y[0] * width,
            x[1] + y[1] * width,
            x[2] + y[2] * width,
            x[3] + y[3] * width,
        ];

        let r0 = Vec4::from(r0);
        let r1 = Vec4::from(r1);
        let r2 = Vec4::from(r2);
        let r3 = Vec4::from(r3);

        let blade: Vec4 = r0 * Vec4::splat(9.0);
        let r2: Vec4 = (r2 - blade * (1.0 / 9.0)) * 9.0;
        let pi_over_4dot5: Vec4 = Vec4::splat(std::f32::consts::PI / 4.5);
        let blade_param: Vec4 = blade * pi_over_4dot5;

        // #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        let (x1, y1) = {
            let mut x = [0.0 as f32; 4];
            let mut y = [0.0 as f32; 4];
            for i in 0..4 {
                let (cos, sin) = blade_param[i].sin_cos();
                x[i] = cos;
                y[i] = sin;
            }

            (Vec4::from(x), Vec4::from(y))
        };

        let blade_param = (blade + Vec4::one()) * pi_over_4dot5;
        let (x2, y2) = {
            let mut x = [0.0 as f32; 4];
            let mut y = [0.0 as f32; 4];
            for i in 0..4 {
                let (cos, sin) = blade_param[i].sin_cos();
                x[i] = cos;
                y[i] = sin;
            }

            (Vec4::from(x), Vec4::from(y))
        };

        let (r2, r3) = {
            let mask: Vec4Mask = (r2 + r3).cmpgt(Vec4::one());
            (
                mask.select(Vec4::one() - r2, r2),
                mask.select(Vec4::one() - r3, r3),
            )
        };

        let x = Vec4::from([x[0] as f32, x[1] as f32, x[2] as f32, x[3] as f32]);
        let y = Vec4::from([y[0] as f32, y[1] as f32, y[2] as f32, y[3] as f32]);

        let xr = x1 * r2 + x2 * r2;
        let yr = y1 * r2 + y2 * r3;

        let u = (x + r0) * self.inv_width;
        let v = (y + r1) * self.inv_height;

        let p_x = Vec4::from([self.p1[0]; 4]) + u * self.p1[0] + v * self.up[0];
        let p_y = Vec4::from([self.p1[1]; 4]) + u * self.p1[1] + v * self.up[1];
        let p_z = Vec4::from([self.p1[2]; 4]) + u * self.p1[2] + v * self.up[2];

        let direction_x = p_x - Vec4::from([self.pos[0]; 4]);
        let direction_y = p_y - Vec4::from([self.pos[1]; 4]);
        let direction_z = p_z - Vec4::from([self.pos[2]; 4]);

        let length_squared = direction_x * direction_x;
        let length_squared = length_squared + direction_y * direction_y;
        let length_squared = length_squared + direction_z * direction_z;

        let length = vec4_sqrt(length_squared);

        let inv_length = Vec4::one() / length;

        let direction_x = (direction_x * inv_length).into();
        let direction_y = (direction_y * inv_length).into();
        let direction_z = (direction_z * inv_length).into();

        let origin_x = Vec4::splat(self.pos[0]);
        let origin_y = Vec4::splat(self.pos[1]);
        let origin_z = Vec4::splat(self.pos[2]);

        let lens_size = Vec4::splat(self.lens_size);
        let right_x = Vec4::splat(self.p1[0]);
        let right_y = Vec4::splat(self.p1[1]);
        let right_z = Vec4::splat(self.p1[2]);
        let up_x = Vec4::splat(self.up[0]);
        let up_y = Vec4::splat(self.up[1]);
        let up_z = Vec4::splat(self.up[2]);

        let origin_x = origin_x + lens_size * (right_x * xr + up_x * yr);
        let origin_y = origin_y + lens_size * (right_y * xr + up_y * yr);
        let origin_z = origin_z + lens_size * (right_z * xr + up_z * yr);

        RayPacket4 {
            origin_x: origin_x.into(),
            origin_y: origin_y.into(),
            origin_z: origin_z.into(),
            direction_x,
            direction_y,
            direction_z,
            t: [1e34 as f32; 4],
            pixel_ids: ids,
        }
    }

    pub fn generate_ray4(&self, x: [u32; 4], y: [u32; 4], width: u32) -> RayPacket4 {
        let ids = [
            x[0] + y[0] * width,
            x[1] + y[1] * width,
            x[2] + y[2] * width,
            x[3] + y[3] * width,
        ];

        let x = [x[0] as f32, x[1] as f32, x[2] as f32, x[3] as f32];
        let y = [y[0] as f32, y[1] as f32, y[2] as f32, y[3] as f32];

        let x = Vec4::from(x);
        let y = Vec4::from(y);

        let u = x * self.inv_width;
        let v = y * self.inv_height;

        let p_x = Vec4::from([self.p1[0]; 4]) + u * self.p1[0] + v * self.up[0];
        let p_y = Vec4::from([self.p1[1]; 4]) + u * self.p1[1] + v * self.up[1];
        let p_z = Vec4::from([self.p1[2]; 4]) + u * self.p1[2] + v * self.up[2];

        let direction_x = p_x - Vec4::from([self.pos[0]; 4]);
        let direction_y = p_y - Vec4::from([self.pos[1]; 4]);
        let direction_z = p_z - Vec4::from([self.pos[2]; 4]);

        let length_squared = direction_x * direction_x;
        let length_squared = length_squared + direction_y * direction_y;
        let length_squared = length_squared + direction_z * direction_z;

        let length = vec4_sqrt(length_squared);

        let inv_length = Vec4::one() / length;

        let direction_x = (direction_x * inv_length).into();
        let direction_y = (direction_y * inv_length).into();
        let direction_z = (direction_z * inv_length).into();

        let origin_x = [self.pos[0]; 4];
        let origin_y = [self.pos[1]; 4];
        let origin_z = [self.pos[2]; 4];

        RayPacket4 {
            origin_x,
            origin_y,
            origin_z,
            direction_x,
            direction_y,
            direction_z,
            t: [1e34 as f32; 4],
            pixel_ids: ids,
        }
    }

    fn calculate_matrix(&self) -> (Vec3, Vec3, Vec3) {
        let y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
        let z: Vec3 = Vec3::from(self.direction).normalize();
        let x: Vec3 = z.cross(y).normalize();
        let y: Vec3 = x.cross(z).normalize();
        (x, y, z)
    }

    pub fn get_rh_matrix(&self) -> Mat4 {
        let up = Vec3::new(0.0, 1.0, 0.0);

        let projection =
            Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.near_plane, self.far_plane);

        let pos = Vec3::from(self.pos);
        let dir = Vec3::from(self.direction);

        let view = Mat4::look_at_rh(pos.into(), (pos + dir).into(), up);

        projection * view
    }

    pub fn get_lh_matrix(&self) -> Mat4 {
        let up = Vec3::new(0.0, 1.0, 0.0);

        let projection =
            Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near_plane, self.far_plane);

        let pos = Vec3::from(self.pos);
        let dir = Vec3::from(self.direction);

        let view = Mat4::look_at_lh(pos.into(), (pos + dir).into(), up);

        projection * view
    }

    pub fn get_rh_projection(&self) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.near_plane, self.far_plane)
    }

    pub fn get_lh_projection(&self) -> Mat4 {
        Mat4::perspective_lh(self.fov, self.aspect_ratio, self.near_plane, self.far_plane)
    }

    pub fn get_rh_view_matrix(&self) -> Mat4 {
        let up = Vec3::new(0.0, 1.0, 0.0);

        let pos = Vec3::from(self.pos);
        let dir = Vec3::from(self.direction);

        Mat4::look_at_rh(pos.into(), (pos + dir).into(), up)
    }

    pub fn get_lh_view_matrix(&self) -> Mat4 {
        let up = Vec3::new(0.0, 1.0, 0.0);

        let pos = Vec3::from(self.pos);
        let dir = Vec3::from(self.direction);

        Mat4::look_at_lh(pos.into(), (pos + dir).into(), up)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct SkinnedMesh3D {
    pub vertices: Vec<Vertex3D>,
    pub ranges: Vec<VertexMesh>,
}

impl SkinnedMesh3D {
    pub fn apply(
        vertices: &[Vertex3D],
        skin_data: &[JointData],
        ranges: &[VertexMesh],
        skin: &SkinData,
    ) -> Self {
        let mut vertices = vertices.to_vec();
        let ranges = ranges.to_vec();
        let matrices = &skin.joint_matrices;

        vertices.par_iter_mut().enumerate().for_each(|(i, v)| {
            let (joint, weight) = skin_data[i].into();
            let matrix = weight[0] * matrices[joint[0] as usize];
            let matrix = matrix + (weight[1] * matrices[joint[1] as usize]);
            let matrix = matrix + (weight[2] * matrices[joint[2] as usize]);
            let matrix = matrix + (weight[3] * matrices[joint[3] as usize]);

            v.vertex = matrix * v.vertex;
            let matrix = matrix.inverse().transpose();
            v.normal = (matrix * Vec3A::from(v.normal).extend(0.0))
                .truncate()
                .into();
            let tangent =
                (matrix * Vec3A::new(v.tangent[0], v.tangent[1], v.tangent[2]).extend(0.0)).xyz();
            v.tangent = Vec4::new(tangent[0], tangent[1], tangent[2], v.tangent[3]);
        });

        SkinnedMesh3D { vertices, ranges }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct SkinnedTriangles3D {
    pub triangles: Vec<RTTriangle>,
}

impl SkinnedTriangles3D {
    pub fn apply(triangles: &[RTTriangle], skin_data: &[JointData], skin: &SkinData) -> Self {
        let mut triangles = triangles.to_vec();
        let matrices = &skin.joint_matrices;

        triangles.iter_mut().enumerate().for_each(|(i, t)| {
            let i0 = i / 3;
            let i1 = i + 1;
            let i2 = i + 2;

            let (joint, weight) = skin_data[i0].into();
            let matrix: Mat4 = weight[0] * matrices[joint[0] as usize];
            let matrix: Mat4 = matrix + (weight[1] * matrices[joint[1] as usize]);
            let matrix: Mat4 = matrix + (weight[2] * matrices[joint[2] as usize]);
            let matrix: Mat4 = matrix + (weight[3] * matrices[joint[3] as usize]);
            let n_matrix: Mat4 = matrix.inverse().transpose();

            t.vertex0 = (matrix * t.vertex0.extend(1.0)).truncate();
            t.n0 = (n_matrix * t.n0.extend(0.0)).truncate();
            t.tangent0 = (n_matrix * t.tangent0.xyz().extend(0.0))
                .truncate()
                .extend(t.tangent2[3]);

            let (joint, weight) = skin_data[i1].into();
            let matrix: Mat4 = weight[0] * matrices[joint[0] as usize];
            let matrix: Mat4 = matrix + (weight[1] * matrices[joint[1] as usize]);
            let matrix: Mat4 = matrix + (weight[2] * matrices[joint[2] as usize]);
            let matrix: Mat4 = matrix + (weight[3] * matrices[joint[3] as usize]);
            let n_matrix: Mat4 = matrix.inverse().transpose();

            t.vertex1 = (matrix * t.vertex1.extend(1.0)).truncate();
            t.n1 = (n_matrix * t.n1.extend(0.0)).truncate();
            t.tangent1 = (n_matrix * t.tangent1.xyz().extend(0.0))
                .truncate()
                .extend(t.tangent2[3]);

            let (joint, weight) = skin_data[i2].into();
            let matrix: Mat4 = weight[0] * matrices[joint[0] as usize];
            let matrix: Mat4 = matrix + (weight[1] * matrices[joint[1] as usize]);
            let matrix: Mat4 = matrix + (weight[2] * matrices[joint[2] as usize]);
            let matrix: Mat4 = matrix + (weight[3] * matrices[joint[3] as usize]);
            let n_matrix: Mat4 = matrix.inverse().transpose();

            t.vertex2 = (matrix * t.vertex2.extend(1.0)).truncate();
            t.n2 = (n_matrix * t.n2.extend(0.0)).truncate();
            t.tangent2 = (n_matrix * t.tangent2.xyz().extend(0.0))
                .truncate()
                .extend(t.tangent2[3]);

            t.normal = RTTriangle::normal(t.vertex0, t.vertex1, t.vertex2);
        });

        SkinnedTriangles3D { triangles }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct RTTriangle {
    pub vertex0: Vec3,
    pub u0: f32,
    // 16
    pub vertex1: Vec3,
    pub u1: f32,
    // 32
    pub vertex2: Vec3,
    pub u2: f32,
    // 48
    pub normal: Vec3,
    pub v0: f32,
    // 64
    pub n0: Vec3,
    pub v1: f32,
    // 80
    pub n1: Vec3,
    pub v2: f32,
    // 96
    pub n2: Vec3,
    pub id: i32,
    // 112
    pub tangent0: Vec4,
    // 128
    pub tangent1: Vec4,
    // 144
    pub tangent2: Vec4,
    // 160
    pub light_id: i32,
    pub mat_id: i32,
    pub lod: f32,
    pub area: f32,
    // 176

    // GLSL structs' size are rounded up to the base alignment of vec4s
    // Thus, we pad these triangles to become 160 bytes and 16-byte (vec4) aligned
}

impl Default for RTTriangle {
    fn default() -> Self {
        // assert_eq!(std::mem::size_of::<RTTriangle>() % 16, 0);
        Self {
            vertex0: Vec3::zero(),
            u0: 0.0,
            vertex1: Vec3::zero(),
            u1: 0.0,
            vertex2: Vec3::zero(),
            u2: 0.0,
            normal: Vec3::zero(),
            v0: 0.0,
            n0: Vec3::zero(),
            v1: 0.0,
            n1: Vec3::zero(),
            v2: 0.0,
            n2: Vec3::zero(),
            id: 0,
            tangent0: Vec4::zero(),
            tangent1: Vec4::zero(),
            tangent2: Vec4::zero(),
            light_id: 0,
            mat_id: 0,
            lod: 0.0,
            area: 0.0,
        }
    }
}

impl SpatialTriangle for RTTriangle {
    fn vertex0(&self) -> [f32; 3] {
        self.vertex0.into()
    }

    fn vertex1(&self) -> [f32; 3] {
        self.vertex1.into()
    }

    fn vertex2(&self) -> [f32; 3] {
        self.vertex2.into()
    }
}

#[allow(dead_code)]
impl RTTriangle {
    pub fn vertices(&self) -> (Vec3, Vec3, Vec3) {
        (self.vertex0, self.vertex1, self.vertex2)
    }

    #[inline]
    pub fn normal(v0: Vec3, v1: Vec3, v2: Vec3) -> Vec3 {
        let a = v1 - v0;
        let b = v2 - v0;
        a.cross(b).normalize()
    }

    #[inline]
    pub fn area(v0: Vec3, v1: Vec3, v2: Vec3) -> f32 {
        let a = (v1 - v0).length();
        let b = (v2 - v1).length();
        let c = (v0 - v2).length();
        let s = (a + b + c) * 0.5;
        (s * (s - a) * (s - b) * (s - c)).sqrt()
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        let (v0, v1, v2) = self.vertices();
        (v0 + v1 + v2) * (1.0 / 3.0)
    }

    #[inline(always)]
    pub fn bary_centrics(
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        edge1: Vec3,
        edge2: Vec3,
        p: Vec3,
        n: Vec3,
    ) -> (f32, f32) {
        let abc = n.dot((edge1).cross(edge2));
        let pbc = n.dot((v1 - p).cross(v2 - p));
        let pca = n.dot((v2 - p).cross(v0 - p));
        (pbc / abc, pca / abc)
    }

    // Transforms triangle using given matrix and normal_matrix (transposed of inverse of matrix)
    pub fn transform(&self, matrix: Mat4, normal_matrix: Mat3) -> RTTriangle {
        let vertex0 = Vec3::from(self.vertex0).extend(1.0);
        let vertex1 = Vec3::from(self.vertex1).extend(1.0);
        let vertex2 = Vec3::from(self.vertex2).extend(1.0);

        let vertex0 = matrix * vertex0;
        let vertex1 = matrix * vertex1;
        let vertex2 = matrix * vertex2;

        let n0 = normal_matrix * Vec3::from(self.n0);
        let n1 = normal_matrix * Vec3::from(self.n1);
        let n2 = normal_matrix * Vec3::from(self.n2);

        RTTriangle {
            vertex0: vertex0.truncate().into(),
            vertex1: vertex1.truncate().into(),
            vertex2: vertex2.truncate().into(),
            n0: n0.into(),
            n1: n1.into(),
            n2: n2.into(),
            ..(*self)
        }
    }

    // #[inline(always)]
    // pub fn occludes(&self, ray: Ray, t_min: f32, t_max: f32) -> bool {
    //     let origin = Vec3::from(ray.origin);
    //     let direction = Vec3::from(ray.direction);

    //     let vertex0 = Vec3::from(self.vertex0);
    //     let vertex1 = Vec3::from(self.vertex1);
    //     let vertex2 = Vec3::from(self.vertex2);

    //     let edge1 = vertex1 - vertex0;
    //     let edge2 = vertex2 - vertex0;

    //     let h = direction.cross(edge2);
    //     let a = edge1.dot(h);
    //     if a > -1e-6 && a < 1e-6 {
    //         return false;
    //     }

    //     let f = 1.0 / a;
    //     let s = origin - vertex0;
    //     let u = f * s.dot(h);
    //     if u < 0.0 || u > 1.0 {
    //         return false;
    //     }

    //     let q = s.cross(edge1);
    //     let v = f * direction.dot(q);
    //     if v < 0.0 || (u + v) > 1.0 {
    //         return false;
    //     }

    //     let t = f * edge2.dot(q);
    //     t > t_min && t < t_max
    // }

    // #[inline(always)]
    // pub fn intersect(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
    //     let origin = Vec3::from(ray.origin);
    //     let direction = Vec3::from(ray.direction);

    //     let vertex0 = Vec3::from(self.vertex0);
    //     let vertex1 = Vec3::from(self.vertex1);
    //     let vertex2 = Vec3::from(self.vertex2);

    //     let edge1 = vertex1 - vertex0;
    //     let edge2 = vertex2 - vertex0;

    //     let h = direction.cross(edge2);
    //     let a = edge1.dot(h);
    //     if a > -1e-6 && a < 1e-6 {
    //         return None;
    //     }

    //     let f = 1.0 / a;
    //     let s = origin - vertex0;
    //     let u = f * s.dot(h);
    //     let q = s.cross(edge1);
    //     let v = f * direction.dot(q);

    //     if u < 0.0 || u > 1.0 || v < 0.0 || (u + v) > 1.0 {
    //         return None;
    //     }

    //     let t = f * edge2.dot(q);
    //     if t <= t_min || t >= t_max {
    //         return None;
    //     }

    //     let p = origin + direction * t;

    //     let gnormal = Vec3::from(self.normal);
    //     let inv_denom = 1.0 / gnormal.dot(gnormal);
    //     let (u, v) = (u * inv_denom, v * inv_denom);

    //     let w = 1.0 - u - v;
    //     let normal = Vec3::from(self.n0) * u + Vec3::from(self.n1) * v + Vec3::from(self.n2) * w;
    //     let uv = Vec2::new(
    //         self.u0 * u + self.u1 * v + self.u2 * w,
    //         self.v0 * u + self.v1 * v + self.v2 * w,
    //     );

    //     Some(HitRecord {
    //         g_normal: self.normal,
    //         normal: normal.into(),
    //         t,
    //         p: p.into(),
    //         mat_id: 0,
    //         uv: uv.into(),
    //     })
    // }

    // #[inline(always)]
    // pub fn intersect_t(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<f32> {
    //     let (origin, direction) = ray.get_vectors::<Vec3>();

    //     let vertex0 = Vec3::from(self.vertex0);
    //     let vertex1 = Vec3::from(self.vertex1);
    //     let vertex2 = Vec3::from(self.vertex2);

    //     let edge1 = vertex1 - vertex0;
    //     let edge2 = vertex2 - vertex0;

    //     let h = direction.cross(edge2);
    //     let a = edge1.dot(h);
    //     if a > -1e-6 && a < 1e-6 {
    //         return None;
    //     }

    //     let f = 1.0 / a;
    //     let s = origin - vertex0;
    //     let u = f * s.dot(h);
    //     if u < 0.0 || u > 1.0 {
    //         return None;
    //     }

    //     let q = s.cross(edge1);
    //     let v = f * direction.dot(q);
    //     if v < 0.0 || (u + v) > 1.0 {
    //         return None;
    //     }

    //     let t = f * edge2.dot(q);
    //     if t <= t_min || t >= t_max {
    //         return None;
    //     }

    //     Some(t)
    // }

    // #[inline(always)]
    // pub fn depth_test(&self, ray: Ray, t_min: f32, t_max: f32) -> Option<(f32, u32)> {
    //     if let Some(t) = self.intersect_t(ray, t_min, t_max) {
    //         return Some((t, 1));
    //     }
    //     None
    // }

    // #[inline(always)]
    // pub fn intersect4(&self, packet: &mut RayPacket4, t_min: &[f32; 4]) -> Option<[i32; 4]> {
    //     #[allow(single)]
    //     let zero = Vec4::zero();
    //     let one = Vec4::one();

    //     let org_x = Vec4::from(packet.origin_x);
    //     let org_y = Vec4::from(packet.origin_y);
    //     let org_z = Vec4::from(packet.origin_z);

    //     let dir_x = Vec4::from(packet.direction_x);
    //     let dir_y = Vec4::from(packet.direction_y);
    //     let dir_z = Vec4::from(packet.direction_z);

    //     let p0_x = Vec4::from([self.vertex0[0]; 4]);
    //     let p0_y = Vec4::from([self.vertex0[1]; 4]);
    //     let p0_z = Vec4::from([self.vertex0[2]; 4]);

    //     let p1_x = Vec4::from([self.vertex1[0]; 4]);
    //     let p1_y = Vec4::from([self.vertex1[1]; 4]);
    //     let p1_z = Vec4::from([self.vertex1[2]; 4]);

    //     let p2_x = Vec4::from([self.vertex2[0]; 4]);
    //     let p2_y = Vec4::from([self.vertex2[1]; 4]);
    //     let p2_z = Vec4::from([self.vertex2[2]; 4]);

    //     let edge1_x = p1_x - p0_x;
    //     let edge1_y = p1_y - p0_y;
    //     let edge1_z = p1_z - p0_z;

    //     let edge2_x = p2_x - p0_x;
    //     let edge2_y = p2_y - p0_y;
    //     let edge2_z = p2_z - p0_z;

    //     let h_x = (dir_y * edge2_z) - (dir_z * edge2_y);
    //     let h_y = (dir_z * edge2_x) - (dir_x * edge2_z);
    //     let h_z = (dir_x * edge2_y) - (dir_y * edge2_x);

    //     let a = (edge1_x * h_x) + (edge1_y * h_y) + (edge1_z * h_z);
    //     let 1e-6 = Vec4::from([1e-6 as f32; 4]);
    //     let mask = a.cmple(-1e-6) | a.cmpge(1e-6);
    //     if mask.bitmask() == 0 {
    //         return None;
    //     }

    //     let f = one / a;
    //     let s_x = org_x - p0_x;
    //     let s_y = org_y - p0_y;
    //     let s_z = org_z - p0_z;

    //     let u = f * ((s_x * h_x) + (s_y * h_y) + (s_z * h_z));
    //     let mask = mask.bitand(u.cmpge(zero) & u.cmple(one));
    //     if mask.bitmask() == 0 {
    //         return None;
    //     }

    //     let q_x = s_y * edge1_z - s_z * edge1_y;
    //     let q_y = s_z * edge1_x - s_x * edge1_z;
    //     let q_z = s_x * edge1_y - s_y * edge1_x;

    //     let v = f * ((dir_x * q_x) + (dir_y * q_y) + (dir_z * q_z));
    //     let mask = mask.bitand(v.cmpge(zero) & (u + v).cmple(one));
    //     if mask.bitmask() == 0 {
    //         return None;
    //     }

    //     let t_min = Vec4::from(*t_min);

    //     let t = f * ((edge2_x * q_x) + (edge2_y * q_y) + (edge2_z * q_z));
    //     let mask = mask.bitand(t.cmpge(t_min) & t.cmplt(packet.t.into()));
    //     let bitmask = mask.bitmask();
    //     if bitmask == 0 {
    //         return None;
    //     }
    //     packet.t = mask.select(t, packet.t.into()).into();

    //     let x = if bitmask & 1 != 0 { self.id } else { -1 };
    //     let y = if bitmask & 2 != 0 { self.id } else { -1 };
    //     let z = if bitmask & 4 != 0 { self.id } else { -1 };
    //     let w = if bitmask & 8 != 0 { self.id } else { -1 };
    //     Some([x, y, z, w])
    // }

    // #[inline(always)]
    // pub fn get_hit_record(&self, ray: Ray, t: f32, _: u32) -> HitRecord {
    //     let (origin, direction) = ray.get_vectors::<Vec3>();
    //     let vertex0 = Vec3::from(self.vertex0);
    //     let vertex1 = Vec3::from(self.vertex1);
    //     let vertex2 = Vec3::from(self.vertex2);
    //     let edge1 = vertex1 - vertex0;
    //     let edge2 = vertex2 - vertex0;

    //     let p = origin + direction * t;
    //     let (u, v) = Self::bary_centrics(
    //         vertex0,
    //         vertex1,
    //         vertex2,
    //         edge1,
    //         edge2,
    //         p,
    //         Vec3::from(self.normal),
    //     );
    //     let w = 1.0 - u - v;
    //     let normal = Vec3::from(self.n0) * u + Vec3::from(self.n1) * v + Vec3::from(self.n2) * w;
    //     let uv = Vec2::new(
    //         self.u0 * u + self.u1 * v + self.u2 * w,
    //         self.v0 * u + self.v1 * v + self.v2 * w,
    //     );

    //     HitRecord {
    //         g_normal: self.normal,
    //         normal: normal.into(),
    //         t,
    //         p: p.into(),
    //         mat_id: 0,
    //         uv: uv.into(),
    //     }
    // }
}