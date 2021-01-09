use super::Node;
use crate::{
    render::UI_PIPELINE_HANDLE,
    widget::{Button, Image},
    ANodeLayoutCache, AnchorLayout, FocusPolicy, Interaction, MinSize,
};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_math::Vec3;
use bevy_render::{
    camera::{Camera, OrthographicProjection, VisibleEntities, WindowOrigin},
    draw::Draw,
    mesh::Mesh,
    pipeline::{RenderPipeline, RenderPipelines},
    prelude::Visible,
};
use bevy_sprite::{ColorMaterial, QUAD_HANDLE};
use bevy_text::{CalculatedSize, Text};
use bevy_transform::prelude::{GlobalTransform, Transform};

#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    pub node: Node,
    pub min_size: MinSize,
    pub anchor_layout: AnchorLayout,
    pub __cache: ANodeLayoutCache,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            min_size: Default::default(),
            node: Default::default(),
            anchor_layout: Default::default(),
            __cache: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct ImageBundle {
    pub node: Node,
    pub min_size: MinSize,
    pub anchor_layout: AnchorLayout,
    pub __cache: ANodeLayoutCache,
    pub image: Image,
    pub calculated_size: CalculatedSize,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ImageBundle {
    fn default() -> Self {
        ImageBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            node: Default::default(),
            min_size: Default::default(),
            image: Default::default(),
            calculated_size: Default::default(),
            anchor_layout: Default::default(),
            __cache: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct TextBundle {
    pub node: Node,
    pub min_size: MinSize,
    pub anchor_layout: AnchorLayout,
    pub __cache: ANodeLayoutCache,
    pub draw: Draw,
    pub visible: Visible,
    pub text: Text,
    pub calculated_size: CalculatedSize,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for TextBundle {
    fn default() -> Self {
        TextBundle {
            focus_policy: FocusPolicy::Pass,
            draw: Draw {
                ..Default::default()
            },
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            text: Default::default(),
            node: Default::default(),
            min_size: Default::default(),
            calculated_size: Default::default(),
            anchor_layout: Default::default(),
            __cache: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    pub node: Node,
    pub min_size: MinSize,
    pub button: Button,
    pub anchor_layout: AnchorLayout,
    pub __cache: ANodeLayoutCache,
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        ButtonBundle {
            button: Button,
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            interaction: Default::default(),
            focus_policy: Default::default(),
            node: Default::default(),
            min_size: Default::default(),
            anchor_layout: Default::default(),
            __cache: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Debug)]
pub struct CameraUiBundle {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for CameraUiBundle {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        CameraUiBundle {
            camera: Camera {
                name: Some(crate::camera::CAMERA_UI.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                window_origin: WindowOrigin::Center,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, far - 0.1)),
            global_transform: Default::default(),
        }
    }
}
