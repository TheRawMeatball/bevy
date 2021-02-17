pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::{IntoSystem, ParallelSystemDescriptorCoercion};
use bevy_reflect::RegisterTypeBuilder;
use prelude::{parent_update_system, Children, GlobalTransform, Parent, PreviousParent, Transform};

#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<PreviousParent>()
            .register_type::<Transform>()
            .register_type::<GlobalTransform>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                parent_update_system.system().label("parent_update_system"),
            )
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                transform_propagate_system::transform_propagate_system
                    .system()
                    .label("transform_propagate_system")
                    .after("parent_update_system"),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                parent_update_system.system().label("parent_update_system"),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                transform_propagate_system::transform_propagate_system
                    .system()
                    .label("transform_propagate_system")
                    .after("parent_update_system"),
            );
    }
}
