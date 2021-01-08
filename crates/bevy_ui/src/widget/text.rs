// use crate::{Node, Style, Val};
use crate::{AnchorLayout, Aspect, AxisConstraint, Constraint, Node};
use bevy_asset::Assets;
use bevy_ecs::{Entity, Flags, Query, Res, ResMut};
use bevy_math::{Size, Vec2};
use bevy_render::{
    draw::{Draw, DrawContext, Drawable},
    mesh::Mesh,
    prelude::{Msaa, Visible},
    renderer::RenderResourceBindings,
    texture::Texture,
};
use bevy_sprite::{TextureAtlas, QUAD_HANDLE};
use bevy_text::{
    CalculatedSize, DefaultTextPipeline, DrawableText, Font, FontAtlasSet, Text, TextError,
};
use bevy_transform::{components::Parent, prelude::GlobalTransform};
use bevy_window::Windows;

#[derive(Debug, Default)]
pub struct QueuedText {
    entities: Vec<Entity>,
}

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

/// Defines how min_size, size, and max_size affects the bounds of a text
/// block.
pub fn text_constraint(node: &AnchorLayout, space: Vec2, scale_factor: f64) -> Size<f32> {
    // Needs support for percentages
    // match (min_size, size, max_size) {
    //     (_, _, Val::Px(max)) => scale_value(max, scale_factor),
    //     (Val::Px(min), _, _) => scale_value(min, scale_factor),
    //     (Val::Undefined, Val::Px(size), Val::Undefined) => scale_value(size, scale_factor),
    //     (Val::Auto, Val::Px(size), Val::Auto) => scale_value(size, scale_factor),
    //     _ => f32::MAX,
    // }

    match &node.constraint {
        Constraint::Independent { x, y } => Size::new(
            solve_value(x, space.x, node.anchors.x()) * scale_factor as f32,
            solve_value(y, space.y, node.anchors.y()) * scale_factor as f32,
        ),
        Constraint::SetXWithY { y, aspect, .. } => {
            let y = solve_value(y, space.y, node.anchors.y());
            let x = aspect.map_value(|a| y * a).unwrap_or_else(|| f32::MAX);
            Size::new(x, y) * scale_factor as f32
        }
        Constraint::SetYWithX { x, aspect, .. } => {
            let x = solve_value(x, space.x, node.anchors.x());
            let y = aspect.map_value(|a| x / a).unwrap_or_else(|| f32::MAX);
            Size::new(x, y) * scale_factor as f32
        }
        Constraint::MaxAspect(aspect) => {
            if let Aspect::Value(aspect) = aspect {
                let x_from_y = (node.anchors.y().1 - node.anchors.y().0) * space.y * aspect;
                let y_from_x = (node.anchors.x().1 - node.anchors.x().0) * space.x / aspect;
    
                if x_from_y >= space.x {
                    Size::new(space.x, y_from_x) * scale_factor as f32
                } else {
                    Size::new(x_from_y, space.y) * scale_factor as f32
                }
            } else {
                Size::new(f32::MAX, f32::MAX)
            }
        }
    }
}

fn solve_value(constraint: &AxisConstraint, space: f32, anchors: (f32, f32)) -> f32 {
    match &constraint {
        AxisConstraint::DoubleMargin(p1, p2) => space * (anchors.1 - anchors.0) - p1 - p2,
        AxisConstraint::DirectMarginAndSize(_, s) => *s,
        AxisConstraint::ReverseMarginAndSize(_, s) => *s,
        AxisConstraint::Centered(s) => *s,
        AxisConstraint::FromContentSize(_) => f32::MAX,
    }
}

/// Computes the size of a text block and updates the TextGlyphs with the
/// new computed glyphs from the layout
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Texture>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<DefaultTextPipeline>,
    nodes: Query<(&Node, Flags<Node>)>,
    mut text_query: Query<(
        Entity,
        &Text,
        Flags<Text>,
        &AnchorLayout,
        Flags<AnchorLayout>,
        Option<&Parent>,
        &mut CalculatedSize,
    )>,
) {
    let (scale_factor, window_size) = if let Some(window) = windows.get_primary() {
        (
            window.scale_factor(),
            Vec2::new(window.width(), window.height()),
        )
    } else {
        (1., Vec2::zero())
    };

    let inv_scale_factor = 1. / scale_factor;

    // Computes all text in the local queue
    let mut new_queue = Vec::new();
    for (entity, text, text_flags, layout, layout_flags, parent, mut calculated_size) in
        text_query.iter_mut()
    {
        let (parent_size, parent_changed) = parent
            .map(|&parent| nodes.get(*parent).unwrap())
            .map(|(parent, flags)| (parent.size, flags.changed()))
            .unwrap_or((window_size, false));

        if text_flags.changed() || layout_flags.changed() || parent_changed {
            let node_size = text_constraint(&layout, parent_size, scale_factor);

            match text_pipeline.queue_text(
                entity,
                text.font.clone(),
                &fonts,
                &text.value,
                scale_value(text.style.font_size, scale_factor),
                text.style.alignment,
                node_size,
                &mut *font_atlas_set_storage,
                &mut *texture_atlases,
                &mut *textures,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {}.", e);
                }
                Ok(()) => {
                    let text_layout_info = text_pipeline.get_glyphs(&entity).expect(
                        "Failed to get glyphs from the pipeline that have just been computed",
                    );
                    let size = Size {
                        width: scale_value(text_layout_info.size.width, inv_scale_factor),
                        height: scale_value(text_layout_info.size.height, inv_scale_factor),
                    };
                    if size != calculated_size.size {
                        calculated_size.dirty = true;
                        calculated_size.size = size;
                    } else {
                        calculated_size.dirty = false;
                    }
                    print!("");
                }
            }
        } else {
            calculated_size.dirty = false;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_system(
    mut context: DrawContext,
    msaa: Res<Msaa>,
    windows: Res<Windows>,
    meshes: Res<Assets<Mesh>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    text_pipeline: Res<DefaultTextPipeline>,
    mut query: Query<(Entity, &mut Draw, &Visible, &Text, &Node, &GlobalTransform)>,
) {
    let scale_factor = if let Some(window) = windows.get_primary() {
        window.scale_factor()
    } else {
        1.
    };

    let font_quad = meshes.get(&QUAD_HANDLE).unwrap();
    let vertex_buffer_descriptor = font_quad.get_vertex_buffer_descriptor();

    for (entity, mut draw, visible, text, node, global_transform) in query.iter_mut() {
        if !visible.is_visible {
            continue;
        }

        if let Some(text_glyphs) = text_pipeline.get_glyphs(&entity) {
            let position = global_transform.translation - (node.size / 2.0).extend(0.0);

            let mut drawable_text = DrawableText {
                render_resource_bindings: &mut render_resource_bindings,
                position,
                scale_factor: scale_factor as f32,
                msaa: &msaa,
                text_glyphs: &text_glyphs.glyphs,
                font_quad_vertex_descriptor: &vertex_buffer_descriptor,
                style: &text.style,
            };

            drawable_text.draw(&mut draw, &mut context).unwrap();
        }
    }
}
