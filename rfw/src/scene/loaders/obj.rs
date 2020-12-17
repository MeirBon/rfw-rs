use crate::utils::collections::TrackedStorage;
use crate::{
    math::*,
    scene::material::*,
    scene::{utils::Flags, SceneError},
    scene::{AnimatedMesh, LoadResult, Mesh, ObjectLoader, ObjectRef},
};
use l3d::mat::{Flip, Texture, TextureSource};
use std::path::PathBuf;

enum ObjFlags {
    HasNormals = 1,
    HasUvs = 2,
}

impl Into<u8> for ObjFlags {
    fn into(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ObjLoader {}

impl std::fmt::Display for ObjLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "obj-loader")
    }
}

impl Default for ObjLoader {
    fn default() -> Self {
        Self {}
    }
}

impl ObjectLoader for ObjLoader {
    fn load(
        &self,
        path: PathBuf,
        mat_manager: &mut MaterialList,
        mesh_storage: &mut TrackedStorage<Mesh>,
        _animated_mesh_storage: &mut TrackedStorage<AnimatedMesh>,
    ) -> Result<LoadResult, SceneError> {
        let object = tobj::load_obj(&path);
        if let Err(_) = object {
            return Err(SceneError::LoadError(path.to_path_buf()));
        }
        let (models, materials) = object.unwrap();
        let mut material_indices = vec![0; materials.len()];

        for (i, material) in materials.iter().enumerate() {
            let mut color = Vec3::from(material.diffuse);
            let specular = Vec3::from(material.specular);

            let roughness = (1.0 - material.shininess.log10() / 1000.0)
                .max(0.0)
                .min(1.0);
            let opacity = 1.0 - material.dissolve;
            let eta = material.optical_density;

            let parent = if let Some(p) = path.parent() {
                p.to_path_buf()
            } else {
                PathBuf::new()
            };

            let d_path = if material.diffuse_texture == "" {
                None
            } else {
                Some(parent.join(material.diffuse_texture.as_str()).to_path_buf())
            };
            let mut n_path = if material.normal_texture == "" {
                None
            } else {
                Some(parent.join(material.normal_texture.as_str()).to_path_buf())
            };

            let mut roughness_map: Option<PathBuf> = None;
            let mut metallic_map: Option<PathBuf> = None;
            let mut emissive_map: Option<PathBuf> = None;
            let mut sheen_map: Option<PathBuf> = None;

            // TODO: Alpha and specular maps
            material.unknown_param.iter().for_each(|(name, value)| {
                let key = name.to_lowercase();
                match key.as_str() {
                    "ke" => {
                        // Emissive
                        let values = value.split_ascii_whitespace();
                        let mut f_values = [0.0 as f32; 3];
                        let mut i = 0;

                        for value in values {
                            assert!(i <= 2);
                            let value: f32 = value.parse().unwrap_or(0.0);
                            f_values[i] = value;
                            i += 1;
                        }

                        let mut value: Vec3A = Vec3A::from(f_values);
                        if !value.cmpeq(Vec3A::zero()).all() && value.cmple(Vec3A::one()).all() {
                            value = value * Vec3A::splat(10.0);
                        }

                        color = value.max(color.into()).into();
                    }
                    "map_pr" => {
                        roughness_map = Some(parent.join(value.as_str()).to_path_buf());
                    }
                    "map_ke" => {
                        emissive_map = Some(parent.join(value.as_str()).to_path_buf());
                    }
                    "ps" | "map_ps" => {
                        sheen_map = Some(parent.join(value.as_str()).to_path_buf());
                    }
                    "pm" | "map_pm" => {
                        metallic_map = Some(parent.join(value.as_str()).to_path_buf());
                    }
                    "norm" | "map_ns" | "map_bump" => {
                        n_path = Some(parent.join(value.as_str()).to_path_buf());
                    }
                    _ => {}
                }
            });

            let metallic_roughness = match (roughness_map, metallic_map) {
                (Some(r), Some(m)) => {
                    let r = Texture::load(&r, Flip::FlipV).map_err(|_| SceneError::LoadError(r))?;
                    let m = Texture::load(&m, Flip::FlipV).map_err(|_| SceneError::LoadError(m))?;
                    let (r, m) = if r.width != m.width || r.height != m.height {
                        let width = r.width.max(m.width);
                        let height = r.height.max(m.height);
                        (r.resized(width, height), m.resized(width, height))
                    } else {
                        (r, m)
                    };

                    let combined = Texture::merge(Some(&r), Some(&m), None, None);
                    Some(TextureSource::Loaded(combined))
                }
                (Some(r), None) => {
                    let r = Texture::load(&r, Flip::FlipV).map_err(|_| SceneError::LoadError(r))?;
                    let combined = Texture::merge(Some(&r), None, None, None);
                    Some(TextureSource::Loaded(combined))
                }
                (None, Some(m)) => {
                    let m = Texture::load(&m, Flip::FlipV).map_err(|_| SceneError::LoadError(m))?;
                    let combined = Texture::merge(None, Some(&m), None, None);
                    Some(TextureSource::Loaded(combined))
                }
                _ => None,
            };

            let mat_index = mat_manager.add_with_maps(
                color,
                roughness,
                specular,
                opacity,
                TextureDescriptor {
                    albedo: if let Some(path) = d_path {
                        Some(TextureSource::Filesystem(path, Flip::FlipV))
                    } else {
                        None
                    },
                    normal: if let Some(path) = n_path {
                        Some(TextureSource::Filesystem(path, Flip::FlipV))
                    } else {
                        None
                    },
                    metallic_roughness_map: metallic_roughness,
                    emissive_map: if let Some(path) = emissive_map {
                        Some(TextureSource::Filesystem(path, Flip::FlipV))
                    } else {
                        None
                    },
                    sheen_map: if let Some(path) = sheen_map {
                        Some(TextureSource::Filesystem(path, Flip::FlipV))
                    } else {
                        None
                    },
                },
            );
            mat_manager.get_mut(mat_index, |m| {
                if let Some(mat) = m {
                    mat.eta = eta;
                }
            });

            material_indices[i] = mat_index;
        }

        if material_indices.is_empty() {
            material_indices.push(mat_manager.add(
                Vec3A::new(1.0, 0.0, 0.0),
                1.0,
                Vec3A::zero(),
                1.0,
            ));
        }

        let mut flags = Flags::new();
        let num_vertices: usize = models.iter().map(|m| m.mesh.indices.len()).sum();

        let mut vertices = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut material_ids = Vec::with_capacity(num_vertices);

        for m in models.iter() {
            let mesh = &m.mesh;

            let mut i = 0;
            for idx in &mesh.indices {
                let idx = *idx as usize;
                let i0 = 3 * idx;
                let i1 = i0 + 1;
                let i2 = i0 + 2;

                let pos = [mesh.positions[i0], mesh.positions[i1], mesh.positions[i2]];

                let normal = if !mesh.normals.is_empty() {
                    flags.set_flag(ObjFlags::HasNormals);
                    [mesh.normals[i0], mesh.normals[i1], mesh.normals[i2]]
                } else {
                    [0.0; 3]
                };

                let uv = if !mesh.texcoords.is_empty() {
                    flags.set_flag(ObjFlags::HasUvs);
                    [mesh.texcoords[idx * 2], mesh.texcoords[idx * 2 + 1]]
                } else {
                    [0.0; 2]
                };

                vertices.push(pos.into());
                normals.push(normal.into());
                uvs.push(uv.into());

                if i % 3 == 0 {
                    let material_id = if mesh.material_id.is_some() {
                        *material_indices
                            .get(mesh.material_id.unwrap())
                            .unwrap_or(&0)
                    } else {
                        material_indices[0]
                    };

                    material_ids.push(material_id as u32);
                }

                i = i + 1;
            }
        }

        let mesh_id = mesh_storage.allocate();
        mesh_storage[mesh_id] = Mesh::new(
            vertices,
            normals,
            uvs,
            material_ids,
            Some(String::from(path.to_str().unwrap())),
        );
        Ok(LoadResult::Object(ObjectRef::Static(mesh_id as u32)))
    }
}