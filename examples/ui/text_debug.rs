use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

/// This example is for debugging text layout
fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_startup_system(infotext_system.system())
        .add_system(change_text_system.system())
        .run();
}

struct TextChanges;

fn infotext_system(commands: &mut Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn(UiCameraBundle::default());
    commands.spawn(TextBundle {
        anchor_layout: AnchorLayout {
            anchors: Anchors::TOP_LEFT,
            constraint: Constraint::Independent {
                x: AxisConstraint::FromContentSize(Alignment::DirectMargin(5.)),
                y: AxisConstraint::FromContentSize(Alignment::ReverseMargin(15.)),
            },
            ..Default::default()
        },
        text: Text::with_section(
            "This is\ntext with\nline breaks\nin the top left",
            TextStyle {
                font: font.clone(),
                font_size: 50.0,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    });
    commands.spawn(TextBundle {
        anchor_layout: AnchorLayout {
            anchors: Anchors::TOP_RIGHT,
            constraint: Constraint::Independent {
                x: AxisConstraint::ReverseMarginAndSize(15., 400.),
                y: AxisConstraint::FromContentSize(Alignment::ReverseMargin(5.)),
            },
            ..Default::default()
        },
        text: Text::with_section(
                    "This text is very long, has a limited width, is centred, is positioned in the top right and is also coloured pink.",
                        TextStyle {
                    font: font.clone(),
                    font_size: 50.0,
                    color: Color::rgb(0.8, 0.2, 0.7),
                },
            TextAlignment {
                horizontal: HorizontalAlign::Center,
                vertical: VerticalAlign::Center,
            },
        ),
        ..Default::default()
    });
    commands
        .spawn(TextBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::BOTTOM_RIGHT,
                constraint: Constraint::Independent {
                    x: AxisConstraint::FromContentSize(Alignment::ReverseMargin(15.)),
                    y: AxisConstraint::FromContentSize(Alignment::DirectMargin(5.)),
                },
                ..Default::default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "This text changes in the bottom right".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "\nThis text changes in the bottom right - ".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::RED,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::ORANGE_RED,
                        },
                    },
                    TextSection {
                        value: " fps, ".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::YELLOW,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::GREEN,
                        },
                    },
                    TextSection {
                        value: " ms/frame".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 30.0,
                            color: Color::BLUE,
                        },
                    },
                ],
                alignment: Default::default(),
            },
            ..Default::default()
        })
        .with(TextChanges);
    commands.spawn(TextBundle {
        anchor_layout: AnchorLayout {
            anchors: Anchors::BOTTOM_LEFT,
            constraint: Constraint::Independent {
                x: AxisConstraint::DirectMarginAndSize(15., 200.),
                y: AxisConstraint::FromContentSize(Alignment::DirectMargin(5.)),
            },
            ..Default::default()
        },
        text: Text::with_section(
            "This\ntext has\nline breaks and also a set width in the bottom left".to_string(),
            TextStyle {
                font,
                font_size: 50.0,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    });
}

fn change_text_system(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<TextChanges>>,
) {
    for mut text in query.iter_mut() {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_avg) = fps_diagnostic.average() {
                fps = fps_avg;
            }
        }

        let mut frame_time = time.delta_seconds_f64();
        if let Some(frame_time_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        {
            if let Some(frame_time_avg) = frame_time_diagnostic.average() {
                frame_time = frame_time_avg;
            }
        }

        text.sections[0].value = format!(
            "This text changes in the bottom right - {:.1} fps, {:.3} ms/frame",
            fps,
            frame_time * 1000.0,
        );

        text.sections[2].value = format!("{:.1}", fps);

        text.sections[4].value = format!("{:.3}", frame_time * 1000.0);
    }
}
