use bevy::prelude::*;

/// This example illustrates how to customize the default window settings
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            width: 500.,
            height: 300.,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(toggle_override.system())
        .add_system(change_scale_factor.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands
        // ui camera
        .spawn(CameraUiBundle::default())
        .spawn(NodeBundle {
            anchor_layout: AnchorLayout {
                anchors: Anchors::LEFT_FULL,
                constraint: Constraint::Independent {
                    x: AxisConstraint::DirectMarginAndSize(0., 200.),
                    y: AxisConstraint::DoubleMargin(0., 0.),
                },
                ..Default::default()
            },
            material: materials.add(Color::rgb(0.65, 0.65, 0.65).into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle {
                anchor_layout: AnchorLayout {
                    anchors: Anchors::FULL,
                    constraint: Constraint::Independent {
                        x: AxisConstraint::DoubleMargin(5., 5.),
                        y: AxisConstraint::FromContentSize(Alignment::ReverseMargin(5.)),
                    },
                    ..Default::default()
                },
                text: Text {
                    value: "Example text".to_string(),
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
}

/// This system toggles scale factor overrides when enter is pressed
fn toggle_override(input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if input.just_pressed(KeyCode::Return) {
        window.set_scale_factor_override(window.scale_factor_override().xor(Some(1.)));
    }
}

/// This system changes the scale factor override when up or down is pressed
fn change_scale_factor(input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if input.just_pressed(KeyCode::Up) {
        window.set_scale_factor_override(window.scale_factor_override().map(|n| n + 1.));
    } else if input.just_pressed(KeyCode::Down) {
        window.set_scale_factor_override(window.scale_factor_override().map(|n| (n - 1.).max(1.)));
    }
}
