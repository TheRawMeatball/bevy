mod anchor;
mod anchors;
pub mod entity;
mod focus;
mod margins;
mod node;
mod render;
pub mod update;
pub mod widget;

pub use anchor::*;
pub use anchors::*;
pub use focus::*;
pub use margins::*;
pub use node::*;
pub use render::*;

pub mod prelude {
    pub use crate::{anchor::*, entity::*, node::*, widget::Button, Anchors, Interaction, Margins};
}

use bevy_app::prelude::*;
use bevy_ecs::{IntoSystem, SystemStage};
use bevy_render::render_graph::RenderGraph;
use update::ui_z_system;

#[derive(Default)]
pub struct UiPlugin;

pub mod stage {
    pub const UI: &str = "ui";
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_before(
            bevy_app::stage::POST_UPDATE,
            stage::UI,
            SystemStage::parallel(),
        )
        // TODO: Change to dependency of text_system once system dependencies land
        .add_stage_before(
            stage::UI,
            "layout",
            SystemStage::serial()
                .with_system(solve_min_system.system())
                .with_system(anchor_node_system.system()),
        )
        .add_system_to_stage(bevy_app::stage::PRE_UPDATE, ui_focus_system.system())
        // add these stages to front because these must run before transform update systems
        .add_system_to_stage(stage::UI, widget::text_system.system())
        .add_system_to_stage(stage::UI, widget::image_node_system.system())
        .add_system_to_stage(stage::UI, ui_z_system.system())
        .add_system_to_stage(bevy_render::stage::DRAW, widget::draw_text_system.system());

        let resources = app.resources();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_ui_graph(resources);
    }
}
