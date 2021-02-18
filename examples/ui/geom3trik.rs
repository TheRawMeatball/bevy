use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run()
}

fn setup(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn(UiCameraBundle::default());

    commands
        .spawn(NodeBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::TOP_LEFT,
                constraint: Constraint::Independent {
                    x: AxisConstraint::DirectMarginAndSize(0., 300.),
                    y: AxisConstraint::ReverseMarginAndSize(0., 100.),
                },
                children_spread: SpreadConstraint::Flex {
                    margin: 0.0,
                    direction: Direction::Right,
                },
                ..Default::default()
            },
            material: materials.add(Color::RED.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    material: materials.add(Color::GREEN.into()),
                    anchor_layout: AnchorLayout {
                        child_constraint: Some(ChildConstraint {
                            flex_shrink: 1.,
                            flex_basis: 200.,
                            ..Default::default()
                        }),
                        constraint: Constraint::Independent {
                            x: AxisConstraint::DoubleMargin(0., 0.),
                            y: AxisConstraint::ReverseMarginAndSize(0., 50.),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .spawn(NodeBundle {
                    material: materials.add(Color::BLUE.into()),
                    anchor_layout: AnchorLayout {
                        child_constraint: Some(ChildConstraint {
                            flex_shrink: 1.,
                            flex_basis: 300.,
                            ..Default::default()
                        }),
                        constraint: Constraint::Independent {
                            x: AxisConstraint::DoubleMargin(0., 0.),
                            y: AxisConstraint::ReverseMarginAndSize(0., 50.),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                });
        });
}
