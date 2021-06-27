use bevy::{
    asset::LoadState,
    ecs::{
        pattern_literal,
        schedule::{PatternLiteral, StateChange},
    },
    prelude::*,
    sprite::TextureAtlasBuilder,
};

/// In this example we generate a new texture atlas (sprite sheet) from a folder containing
/// individual sprites
fn main() {
    App::build()
        .init_resource::<RpgSpriteHandles>()
        .add_plugins(DefaultPlugins)
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::SETUP).with_system(load_textures.system()))
        .add_system_set(SystemSet::on_update(AppState::SETUP).with_system(check_textures.system()))
        .add_system_set(SystemSet::on_enter(AppState::FINISHED).with_system(setup.system()))
        .run();
}

#[derive(Clone, Copy)]
enum AppState {
    Setup,
    Finished,
}

impl AppState {
    const SETUP: PatternLiteral<Self> = pattern_literal!(AppState::Setup);
    const FINISHED: PatternLiteral<Self> = pattern_literal!(AppState::Finished);
}

#[derive(Default)]
struct RpgSpriteHandles {
    handles: Vec<HandleUntyped>,
}

fn load_textures(mut rpg_sprite_handles: ResMut<RpgSpriteHandles>, asset_server: Res<AssetServer>) {
    rpg_sprite_handles.handles = asset_server.load_folder("textures/rpg").unwrap();
}

fn check_textures(
    mut state: EventWriter<StateChange<AppState>>,
    rpg_sprite_handles: ResMut<RpgSpriteHandles>,
    asset_server: Res<AssetServer>,
) {
    if let LoadState::Loaded =
        asset_server.get_group_load_state(rpg_sprite_handles.handles.iter().map(|handle| handle.id))
    {
        state.send(StateChange::to(AppState::Finished));
    }
}

fn setup(
    mut commands: Commands,
    rpg_sprite_handles: Res<RpgSpriteHandles>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut texture_atlas_builder = TextureAtlasBuilder::default();
    for handle in rpg_sprite_handles.handles.iter() {
        let texture = textures.get(handle).unwrap();
        texture_atlas_builder.add_texture(handle.clone_weak().typed::<Texture>(), texture);
    }

    let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
    let texture_atlas_texture = texture_atlas.texture.clone();
    let vendor_handle = asset_server.get_handle("textures/rpg/chars/vendor/generic-rpg-vendor.png");
    let vendor_index = texture_atlas.get_texture_index(&vendor_handle).unwrap();
    let atlas_handle = texture_atlases.add(texture_atlas);

    // set up a scene to display our texture atlas
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    // draw a sprite from the atlas
    commands.spawn_bundle(SpriteSheetBundle {
        transform: Transform {
            translation: Vec3::new(150.0, 0.0, 0.0),
            scale: Vec3::splat(4.0),
            ..Default::default()
        },
        sprite: TextureAtlasSprite::new(vendor_index as u32),
        texture_atlas: atlas_handle,
        ..Default::default()
    });
    // draw the atlas itself
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_atlas_texture.into()),
        transform: Transform::from_xyz(-300.0, 0.0, 0.0),
        ..Default::default()
    });
}
