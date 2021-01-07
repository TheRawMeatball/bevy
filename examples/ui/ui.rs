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
                                    anchors: Anchors::TOP_FULL,
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
                });
            // right vertical fill
            parent
                .spawn(NodeBundle {
                    anchor_layout: AnchorLayout {
                        anchors: Anchors {
                            left: 0.7,
                            right: 1.,
                            bottom: 0.,
                            top: 1.,
                        },
                        constraint: Constraint::Independent {
                            x: AxisConstraint::DoubleMargin(0., 0.),
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
                                            max_size: ConstraintSize::Pixels(80.),
                                            min_size: ConstraintSize::FromContent,
                                            ..Default::default()
                                        }),
                                        children_spread: Some(SpreadConstraint {
                                            direction: Direction::Right,
                                            margin: 5.,
                                            ..Default::default()
                                        }),
                                        padding: Rect::all(5.),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with_children(|parent| {
                                    parent
                                        .spawn(NodeBundle {
                                            material: materials.add(Color::GRAY.into()),
                                            anchor_layout: AnchorLayout {
                                                anchors: Anchors::CENTER,
                                                padding: Rect::all(13.),
                                                constraint: Constraint::Independent {
                                                    x: AxisConstraint::FromContentSize(
                                                        Alignment::Offset(0.),
                                                    ),
                                                    y: AxisConstraint::FromContentSize(
                                                        Alignment::Offset(0.),
                                                    ),
                                                },
                                                child_constraint: Some(ChildConstraint {
                                                    max_size: ConstraintSize::FromContent,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        })
                                        .with_children(|parent| {
                                            parent.spawn(TextBundle {
                                                text: Text {
                                                    font: asset_server
                                                        .load("fonts/FiraSans-Bold.ttf"),
                                                    value: "Dynamic layout!".into(),
                                                    style: TextStyle {
                                                        font_size: 20.,
                                                        color: Color::WHITE,
                                                        ..Default::default()
                                                    },
                                                },
                                                anchor_layout: AnchorLayout {
                                                    constraint: Constraint::Independent {
                                                        x: AxisConstraint::FromContentSize(
                                                            Alignment::Offset(0.),
                                                        ),
                                                        y: AxisConstraint::FromContentSize(
                                                            Alignment::Offset(0.),
                                                        ),
                                                    },
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });
                                        });

                                    parent
                                        .spawn(NodeBundle {
                                            material: materials.add(Color::GRAY.into()),
                                            anchor_layout: AnchorLayout {
                                                anchors: Anchors::CENTER_FULL_HORIZONTAL,
                                                padding: Rect::all(13.),
                                                constraint: Constraint::Independent {
                                                    x: AxisConstraint::DoubleMargin(0., 0.),
                                                    y: AxisConstraint::FromContentSize(
                                                        Alignment::Offset(0.),
                                                    ),
                                                },
                                                child_constraint: Some(ChildConstraint {
                                                    min_size: ConstraintSize::FromContent,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        })
                                        .with_children(|parent| {
                                            parent.spawn(TextBundle {
                                                text: Text {
                                                    font: asset_server
                                                        .load("fonts/FiraSans-Bold.ttf"),
                                                    value: "This is a longer string!".into(),
                                                    style: TextStyle {
                                                        font_size: 20.,
                                                        color: Color::WHITE,
                                                        ..Default::default()
                                                    },
                                                },
                                                anchor_layout: AnchorLayout {
                                                    constraint: Constraint::Independent {
                                                        x: AxisConstraint::FromContentSize(
                                                            Alignment::Offset(0.),
                                                        ),
                                                        y: AxisConstraint::FromContentSize(
                                                            Alignment::Offset(0.),
                                                        ),
                                                    },
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });
                                        });
                                });

                            parent.spawn(NodeBundle {
                                material: materials.add(Color::GREEN.into()),
                                anchor_layout: AnchorLayout {
                                    child_constraint: Some(ChildConstraint {
                                        weight: 1.,
                                        min_size: ConstraintSize::Pixels(200.),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                            parent.spawn(NodeBundle {
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
                });
            // absolute positioning
            parent
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
                });
            // render order test: reddest in the back, whitest in the front
            parent
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
                    parent.spawn(NodeBundle {
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
                    });
                    parent.spawn(NodeBundle {
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
                    });
                    parent.spawn(NodeBundle {
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
                    });
                    // alpha test
                    parent.spawn(NodeBundle {
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
                });
            parent.spawn(ImageBundle {
                anchor_layout: AnchorLayout {
                    anchors: Anchors::CENTER_TOP,
                    constraint: Constraint::SetYWithX {
                        x: AxisConstraint::Centered(500.),
                        y: Alignment::ReverseMargin(0.),
                        aspect: Aspect::FromContentSize,
                    },
                    ..Default::default()
                },
                material: materials
                    .add(asset_server.load("branding/bevy_logo_dark_big.png").into()),
                ..Default::default()
            });
        });
}
