use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_render::renderer::RenderResources;

#[derive(Debug, Clone, Default, RenderResources, Reflect)]
#[reflect(Component)]
pub struct Node {
    pub size: Vec2,
}

#[derive(Debug, Clone, Default, Reflect)]
pub struct MinSize {
    /// Used internally, DO NOT set manually
    pub(crate) size: Vec2
}