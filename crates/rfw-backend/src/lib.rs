pub use l3d::mat::Texture;
pub use raw_window_handle::HasRawWindowHandle;
pub use rfw_scene::{
    AreaLight, Camera, DeviceMaterial, DirectionalLight, Instance2D, Instance3D, Mesh2D, Mesh3D,
    PointLight, Skin, SpotLight,
};
pub use rfw_utils::collections::ChangedIterator;
use std::error::Error;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RenderMode {
    Default = 0,
    Reset = 1,
    Accumulate = 2,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::Default
    }
}

pub trait Backend {
    type Settings;

    /// Initializes renderer with surface given through a raw window handle
    fn init<T: HasRawWindowHandle>(
        window: &T,
        window_size: (usize, usize),
        render_size: (usize, usize),
    ) -> Result<Box<Self>, Box<dyn Error>>;

    /// Updates 2d meshes
    fn set_2d_meshes(&mut self, meshes: ChangedIterator<'_, Mesh2D>);

    /// Updates instances of 2d meshes
    fn set_2d_instances(&mut self, instances: ChangedIterator<'_, Instance2D>);

    /// Updates meshes
    fn set_3d_meshes(&mut self, meshes: ChangedIterator<'_, Mesh3D>);

    fn unload_3d_meshes(&mut self, ids: Vec<usize>);

    /// Sets an instance with a 4x4 transformation matrix in column-major format
    fn set_3d_instances(&mut self, instances: ChangedIterator<'_, Instance3D>);

    fn unload_3d_instances(&mut self, ids: Vec<usize>);

    /// Updates materials
    fn set_materials(&mut self, materials: ChangedIterator<'_, DeviceMaterial>);

    /// Updates textures
    fn set_textures(&mut self, textures: ChangedIterator<'_, Texture>);

    /// Synchronizes scene after updating meshes, instances, materials and lights
    /// This is an expensive step as it can involve operations such as acceleration structure rebuilds
    fn synchronize(&mut self);

    /// Renders an image to the window surface
    fn render(&mut self, camera: &Camera, mode: RenderMode);

    /// Resizes framebuffer
    fn resize<T: HasRawWindowHandle>(
        &mut self,
        window: &T,
        window_size: (usize, usize),
        render_size: (usize, usize),
    );
    /// Updates point lights, only lights with their 'changed' flag set to true have changed
    fn set_point_lights(&mut self, lights: ChangedIterator<'_, PointLight>);

    /// Updates spot lights, only lights with their 'changed' flag set to true have changed
    fn set_spot_lights(&mut self, lights: ChangedIterator<'_, SpotLight>);

    /// Updates area lights, only lights with their 'changed' flag set to true have changed
    fn set_area_lights(&mut self, lights: ChangedIterator<'_, AreaLight>);

    /// Updates directional lights, only lights with their 'changed' flag set to true have changed
    fn set_directional_lights(&mut self, lights: ChangedIterator<'_, DirectionalLight>);

    // Sets the scene skybox
    fn set_skybox(&mut self, skybox: Texture);

    // Sets skins
    fn set_skins(&mut self, skins: ChangedIterator<'_, Skin>);

    // Access backend settings
    fn settings(&mut self) -> &mut Self::Settings;
}
