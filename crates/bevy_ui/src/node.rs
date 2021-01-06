use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_render::renderer::RenderResources;

#[derive(Debug, Clone, Default, RenderResources, Reflect)]
#[reflect(Component)]
pub struct Node {
    pub size: Vec2,
}
