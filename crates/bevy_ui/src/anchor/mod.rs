use bevy_ecs::{Entity, Flags, Local, Query, Res, With, Without};

mod solver;
mod types;
use bevy_math::Vec2;
use bevy_text::CalculatedSize;
use bevy_transform::components::{Children, Parent, Transform};
use bevy_window::Windows;
pub use types::*;

use crate::Node;

pub(crate) fn anchor_node_system(
    roots: Query<Entity, (With<AnchorLayout>, Without<Parent>)>,
    nodes: Query<(
        &AnchorLayout,
        Flags<AnchorLayout>,
        Option<&CalculatedSize>,
        Option<&Children>,
        Option<Flags<Children>>,
    )>,
    mut transforms: Query<(&mut Transform, &mut Node, &mut ANodeLayoutCache), With<AnchorLayout>>,
    windows: Res<Windows>,
    mut local: Local<Vec2>,
) {
    let window = windows.get_primary();
    if let Some(window) = window {
        let window_size = Vec2::new(window.width(), window.height());
        if window_size != *local {
            *local = window_size;
            for root in roots.iter() {
                solver::solve(root, window_size, false, &nodes, &mut transforms);
            }
        } else {
            for root in roots.iter() {
                solver::solve(root, window_size, true, &nodes, &mut transforms);
            }
        }
    }
}
