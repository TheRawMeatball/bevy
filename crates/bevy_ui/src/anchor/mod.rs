use bevy_ecs::{Entity, Flags, Local, Query, Res, With, Without};

mod solve_min;
mod solver;
mod types;
use bevy_math::{Rect, Vec2};
use bevy_text::CalculatedSize;
use bevy_transform::components::{Children, Parent, Transform};
use bevy_window::Windows;
pub use types::*;

use crate::{MinSize, Node};

pub(crate) fn anchor_node_system(
    roots: Query<Entity, (With<AnchorLayout>, Without<Parent>)>,
    nodes: Query<(
        &AnchorLayout,
        Flags<AnchorLayout>,
        &MinSize,
        Flags<MinSize>,
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
                solver::solve(root, window_size, Rect::all(0.), false, &nodes, &mut transforms);
            }
        } else {
            for root in roots.iter() {
                solver::solve(root, window_size, Rect::all(0.), true, &nodes, &mut transforms);
            }
        }
    }
    println!(" ------------------------ ");
    for (t, n, ..) in transforms.iter_mut() {
        println!("{:?} {:?}", t.translation, n.size);
    }
}

pub(crate) fn solve_min_system(
    roots: Query<Entity, (With<AnchorLayout>, Without<Parent>)>,
    nodes: Query<(&AnchorLayout, Option<&Children>, Option<&CalculatedSize>)>,
    mut mutable: Query<&mut MinSize>,
) {
    for root in roots.iter() {
        solve_min::solve(root, &nodes, &mut mutable);
    }
}

impl Aspect {
    pub fn unwrap_or_else(&self, f: impl FnOnce() -> f32) -> f32 {
        match self {
            Aspect::Value(v) => *v,
            Aspect::FromContentSize => f(),
        }
    }

    pub fn map_value(self, f: impl FnOnce(f32) -> f32) -> Self {
        match self {
            Aspect::Value(v) => Aspect::Value(f(v)),
            Aspect::FromContentSize => self,
        }
    }
}
