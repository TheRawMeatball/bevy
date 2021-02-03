use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

/// This example illustrates how to create UI text and update it in a system. It displays the
/// current FPS in the top left corner, as well as text that changes colour in the bottom right.
/// For text within a scene, please see the text2d example.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_startup_system(setup.system())
        .add_system(text_update_system.system())
        .add_system(text_color_system.system())
        .run();
}

// A unit struct to help identify the FPS UI component, since there may be many Text components
struct FpsText;

// A unit struct to help identify the color-changing Text component
struct ColorText;

fn setup(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        // UI camera
        .spawn(UiCameraBundle::default())
        // Text with one section
        .spawn(TextBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::TOP_LEFT,
                constraint: Constraint::Independent {
                    x: AxisConstraint::FromContentSize(Alignment::DirectMargin(0.)),
                    y: AxisConstraint::FromContentSize(Alignment::ReverseMargin(0.)),
                },
                ..Default::default()
            },
            // Use the `Text::with_section` constructor
            text: Text::with_section(
                // Accepts a `String` or any type that converts into a `String`, such as `&str`
                "hello\nbevy!",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 100.0,
                    color: Color::WHITE,
                },
                // Note: You can use `Default::default()` in place of the `TextAlignment`
                TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .with(ColorText)
        // Rich text with multiple sections
        .spawn(TextBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::TOP_RIGHT,
                constraint: Constraint::Independent {
                    x: AxisConstraint::FromContentSize(Alignment::DirectMargin(0.)),
                    y: AxisConstraint::FromContentSize(Alignment::ReverseMargin(0.)),
                },
                ..Default::default()
            },
            // Use `Text` directly
            text: Text {
                // Construct a `Vec` of `TextSection`s
                sections: vec![
                    TextSection {
                        value: "FPS: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 60.0,
                            color: Color::WHITE,
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                            font_size: 60.0,
                            color: Color::GOLD,
                        },
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        })
        .with(FpsText);
}

fn text_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                // Update the value of the second section
                text.sections[1].value = format!("{:.2}", average);
            }
        }
    }
}

fn text_color_system(time: Res<Time>, mut query: Query<&mut Text, With<ColorText>>) {
    for mut text in query.iter_mut() {
        let seconds = time.seconds_since_startup() as f32;
        // We used the `Text::with_section` helper method, but it is still just a `Text`,
        // so to update it, we are still updating the one and only section
        text.sections[0]
            .style
            .color
            .set_r((1.25 * seconds).sin() / 2.0 + 0.5)
            .set_g((0.75 * seconds).sin() / 2.0 + 0.5)
            .set_b((0.50 * seconds).sin() / 2.0 + 0.5);
    }
}
