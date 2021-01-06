use bevy::prelude::*;

/// This example illustrates the various features of Bevy UI.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let transparent = materials.add(Color::NONE.into());
    commands
        // ui camera
        .spawn(CameraUiBundle::default())
        // root node
        .spawn(NodeBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::FULL,
                ..Default::default()
            },
            material: transparent.clone(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                // left vertical fill (border)
                .spawn(NodeBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors::LEFT_FULL,
                        constraint: Constraint::Independent {
                            x: AxisConstraint::DirectMarginAndSize(0., 200.),
                            y: Default::default(),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.65, 0.65, 0.65).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        // left vertical fill (content)
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::FULL,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DoubleMargin(2., 2.),
                                    y: AxisConstraint::DoubleMargin(2., 2.),
                                },
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // text
                            parent.spawn(TextBundle {
                                anchor_layout: AnchorLayout {
                                    anchors: Anchors::FULL,
                                    constraint: Constraint::Independent {
                                        x: AxisConstraint::DoubleMargin(5., 5.),
                                        y: AxisConstraint::FromContentSize(
                                            Alignment::ReverseMargin(5.),
                                        ),
                                    },
                                    ..Default::default()
                                },
                                text: Text {
                                    value: "Text Example".to_string(),
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    style: TextStyle {
                                        font_size: 30.0,
                                        color: Color::WHITE,
                                        ..Default::default()
                                    },
                                },
                                ..Default::default()
                            });
                        });
                })
                // right vertical fill
                .spawn(NodeBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors::RIGHT_FULL,
                        constraint: Constraint::Independent {
                            x: AxisConstraint::ReverseMarginAndSize(0., 200.),
                            y: AxisConstraint::DoubleMargin(0., 0.),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    // Dynamic layout
                    parent
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::FULL,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DoubleMargin(10., 10.),
                                    y: AxisConstraint::DoubleMargin(10., 10.),
                                },
                                children_spread: Some(SpreadConstraint {
                                    direction: Direction::Down,
                                    margin: 10.,
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            material: transparent.clone(),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(NodeBundle {
                                    material: materials.add(Color::CYAN.into()),
                                    anchor_layout: AnchorLayout {
                                        child_constraint: Some(ChildConstraint {
                                            weight: 1.,
                                            max_size: 300.,
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .spawn(NodeBundle {
                                    material: materials.add(Color::GREEN.into()),
                                    anchor_layout: AnchorLayout {
                                        child_constraint: Some(ChildConstraint {
                                            weight: 1.,
                                            min_size: 200.,
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .spawn(NodeBundle {
                                    material: materials.add(Color::TEAL.into()),
                                    anchor_layout: AnchorLayout {
                                        child_constraint: Some(ChildConstraint {
                                            weight: 2.,
                                            ..Default::default()
                                        }),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                        });
                })
                // absolute positioning
                .spawn(NodeBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors::BOTTOM_LEFT,
                        constraint: Constraint::Independent {
                            x: AxisConstraint::DirectMarginAndSize(210., 200.),
                            y: AxisConstraint::DirectMarginAndSize(10., 200.),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.4, 0.4, 1.0).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(NodeBundle {
                        anchor_layout: AnchorLayout {
                            anchors: Anchors::FULL,
                            constraint: Constraint::Independent {
                                x: AxisConstraint::DoubleMargin(20., 20.),
                                y: AxisConstraint::DoubleMargin(20., 20.),
                            },
                            ..Default::default()
                        },
                        material: materials.add(Color::rgb(0.8, 0.8, 1.0).into()),
                        ..Default::default()
                    });
                })
                // render order test: reddest in the back, whitest in the front (flex center)
                .spawn(NodeBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors::CENTER,
                        constraint: Constraint::Independent {
                            x: AxisConstraint::Centered(100.),
                            y: AxisConstraint::Centered(100.),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::BOTTOM_LEFT,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DirectMarginAndSize(20., 100.),
                                    y: AxisConstraint::DirectMarginAndSize(20., 100.),
                                },
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
                            ..Default::default()
                        })
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::BOTTOM_LEFT,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DirectMarginAndSize(40., 100.),
                                    y: AxisConstraint::DirectMarginAndSize(40., 100.),
                                },
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
                            ..Default::default()
                        })
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::BOTTOM_LEFT,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DirectMarginAndSize(60., 100.),
                                    y: AxisConstraint::DirectMarginAndSize(60., 100.),
                                },
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
                            ..Default::default()
                        })
                        // alpha test
                        .spawn(NodeBundle {
                            anchor_layout: AnchorLayout {
                                anchors: Anchors::BOTTOM_LEFT,
                                constraint: Constraint::Independent {
                                    x: AxisConstraint::DirectMarginAndSize(80., 100.),
                                    y: AxisConstraint::DirectMarginAndSize(80., 100.),
                                },
                                ..Default::default()
                            },
                            material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
                            ..Default::default()
                        });
                })
                .spawn(ImageBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors::CENTER_TOP,
                        constraint: Constraint::SetYWithX {
                            x: AxisConstraint::Centered(500.),
                            y: Alignment::ReverseMargin(0.),
                            aspect: None,
                        },
                        ..Default::default()
                    },
                    material: materials
                        .add(asset_server.load("branding/bevy_logo_dark_big.png").into()),
                    ..Default::default()
                });

            // // bevy logo (flex center)
            // .spawn(NodeBundle {
            //     style: Style {
            //         size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
            //         position_type: PositionType::Absolute,
            //         justify_content: JustifyContent::Center,
            //         align_items: AlignItems::FlexEnd,
            //         ..Default::default()
            //     },
            //     material: materials.add(Color::NONE.into()),
            //     ..Default::default()
            // })
            // .with_children(|parent| {
            //     // bevy logo (image)
            //     parent.spawn(ImageBundle {
            //         style: Style {
            //             size: Size::new(Val::Px(500.0), Val::Auto),
            //             ..Default::default()
            //         },
            //         material: materials
            //             .add(asset_server.load("branding/bevy_logo_dark_big.png").into()),
            //         ..Default::default()
            //     });
            // });
        });
}
