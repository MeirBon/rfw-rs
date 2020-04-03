use glam::*;

pub mod sphere;
pub mod triangle;
pub mod obj;
pub mod mesh;
pub mod instance;

pub use sphere::Sphere;
pub use triangle::Triangle;
pub use obj::Obj;
pub use mesh::Mesh;
pub use instance::Instance;
pub use mesh::ToMesh;
use bvh::Bounds;

#[derive(Copy, Clone, Debug)]
pub struct HitRecord {
    pub normal: [f32; 3],
    pub t: f32,
    pub p: [f32; 3],
    pub mat_id: u32,
    pub uv: [f32; 2],
}

pub trait Intersect: Bounds + Send + Sync {
    fn occludes(&self, origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> bool;
    fn intersect(&self, origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> Option<HitRecord>;
    fn intersect_t(&self, origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> Option<f32>;
    fn depth_test(&self, origin: Vec3, direction: Vec3, t_min: f32, t_max: f32) -> Option<(f32, u32)>;
}

pub struct Quad {
    pub normal: Vec3,
    pub position: Vec3,
    pub width: f32,
    pub height: f32,
    pub material_id: i32,

    vertices: [Vec3; 6],
    normals: [Vec3; 6],
    uvs: [Vec2; 6],
    material_ids: [u32; 2],
}

#[allow(dead_code)]
impl Quad {
    pub fn new(normal: Vec3,
               position: Vec3,
               width: f32,
               height: f32,
               material_id: i32) -> Quad {
        let material_id = material_id.max(0);
        // TODO: uvs
        let uvs = [vec2(0.0, 0.0); 6];
        let material_ids = [material_id as u32; 2];

        let (vertices, normals) = Quad::generate_render_data(position, normal, width, height);

        Quad {
            normal,
            position,
            width,
            height,
            material_id,

            vertices,
            normals,
            uvs,
            material_ids,
        }
    }

    fn generate_render_data(pos: Vec3, n: Vec3, width: f32, height: f32)
                            -> ([Vec3; 6], [Vec3; 6]) {
        let normal = n.normalize();
        let tmp = if normal.x() > 0.9 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        let tangent: Vec3 = 0.5 * width * normal.cross(tmp).normalize();
        let bi_tangent: Vec3 = 0.5 * height * tangent.normalize().cross(normal);

        let vertices: [Vec3; 6] = [
            pos - bi_tangent - tangent,
            pos + bi_tangent - tangent,
            pos - bi_tangent + tangent,
            pos + bi_tangent - tangent,
            pos + bi_tangent + tangent,
            pos - bi_tangent + tangent
        ];

        let normals = [normal.clone(); 6];

        (vertices, normals)
    }
}

impl ToMesh for Quad {
    fn into_mesh(self) -> Mesh {
        Mesh::new(&self.vertices, &self.normals, &self.uvs, &self.material_ids)
    }
}