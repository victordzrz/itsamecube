//! Renders a 2D scene containing a single, moving sprite.

use appsink::{AppSinkImage, AppSinkImageLoader};
use gst::traits::GstObjectExt;
use std::f32::consts::PI;

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};
mod appsink;

#[derive(Default)]
struct State {
    appsink_handle: Handle<AppSinkImage>,
    image_handle: Handle<Image>,
    material_handle: Handle<StandardMaterial>,
}

impl State {
    fn copy_image(&self, appsinks: Res<Assets<AppSinkImage>>, mut images: ResMut<Assets<Image>>) {
        if let (Some(imagesink), Some(image)) = (
            appsinks.get(&self.appsink_handle),
            images.get_mut(&self.image_handle),
        ) {
            if let Ok(vide_image) = imagesink.image_raw.read() {
                image.data = vide_image.to_vec();
            }
        } else {
            println!("Not loaded")
        }
        images.set_changed();
    }

    fn update_material(
        &self,
        images: Res<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        if let (Some(image), Some(material)) = (
            images.get(&self.image_handle),
            materials.get_mut(&self.material_handle),
        ) {
            material.base_color_texture = Some(self.image_handle.clone_weak());
        }
        materials.set_changed();
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(State::default())
        .add_asset::<AppSinkImage>()
        .init_asset_loader::<AppSinkImageLoader>()
        .add_startup_system(setup)
        .add_system(copy_texture)
        .add_system(update_material)
        .add_system(cube_rotator_system)
        //.add_system(monitor_bus)
        .run();
}
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in &mut query {
        transform.rotate_x(1.0 * time.delta_seconds());
        transform.rotate_y(0.7 * time.delta_seconds());
    }
}
#[derive(Component)]
enum Direction {
    Up,
    Down,
}
// Marks the main pass cube, to which the texture is applied.
#[derive(Component)]
struct MainPassCube;

// fn setup(
//     mut commands: Commands,
//     mut state: ResMut<State>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut images: ResMut<Assets<Image>>,
//     asset_server: Res<AssetServer>,
// ) {
//     let size = Extent3d {
//         width: 200,
//         height: 200,
//         ..default()
//     };

//     // This is the texture that will be rendered to.
//     let mut image = Image {
//         texture_descriptor: TextureDescriptor {
//             label: None,
//             size,
//             dimension: TextureDimension::D2,
//             format: TextureFormat::Rgba8UnormSrgb,
//             mip_level_count: 1,
//             sample_count: 1,
//             usage: TextureUsages::TEXTURE_BINDING
//                 | TextureUsages::COPY_DST
//                 | TextureUsages::RENDER_ATTACHMENT,
//         },
//         ..default()
//     };

//     // fill image.data with zeroes
//     image.resize(size);

//     let image_handle = images.add(image);

//     // // Light
//     // // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
//     // commands.spawn_bundle(PointLightBundle {
//     //     transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
//     //     ..default()
//     // });

//     let cube_size = 4.0;
//     let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

//     // This material has the texture that has been rendered.
//     let material_handle = materials.add(StandardMaterial {
//         base_color_texture: Some(image_handle.clone_weak()),
//         // reflectance: 0.02,
//         //unlit: false,
//         ..default()
//     });

//     // Main pass cube, with material containing the rendered first pass texture.
//     commands
//         .spawn_bundle(PbrBundle {
//             mesh: cube_handle,
//             material: material_handle,
//             transform: Transform::from_xyz(0.0, 0.0, 1.5)
//                 .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
//             ..default()
//         })
//         .insert(MainPassCube);

//     commands.spawn_bundle(SpriteBundle {
//         texture: image_handle.clone_weak(),
//         transform: Transform::from_xyz(100., 0., 0.),
//         ..default()
//     });

//     // The main pass camera.
//     commands.spawn_bundle(Camera3dBundle {
//         transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
//         ..default()
//     });

//     state.appsink_handle = asset_server.load("test.sinkimage");
//     state.image_handle = image_handle;
// }

fn setup(
    mut state: ResMut<State>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = Extent3d {
        width: 176,
        height: 144,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..default()
    };

    image.resize(size);
    let image_handle = images.add(image);

    //commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(SpriteBundle {
            texture: image_handle.clone_weak(),
            transform: Transform::from_xyz(100., 0., 0.),
            ..default()
        })
        .insert(Direction::Up);

    //3d stuff
    let cube_size = 4.0;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle.clone_weak()),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    // Main pass cube, with material containing the rendered first pass texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: material_handle.clone_weak(),
            transform: Transform::from_xyz(0.0, 0.0, 1.5)
                .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
            ..default()
        })
        .insert(MainPassCube);

    commands.spawn_bundle(SpriteBundle {
        texture: image_handle.clone_weak(),
        transform: Transform::from_xyz(100., 0., 0.),
        ..default()
    });

    // The main pass camera.
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    state.appsink_handle = asset_server.load("test.sinkimage");
    state.image_handle = image_handle;
    state.material_handle = material_handle;
}

/// The sprite is animated by changing its translation depending on the time that has passed since
/// the last frame.
fn copy_texture(
    state: Res<State>,
    appsinks: Res<Assets<AppSinkImage>>,
    images: ResMut<Assets<Image>>,
) {
    state.copy_image(appsinks, images);
}

fn update_material(
    state: Res<State>,
    images: Res<Assets<Image>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    state.update_material(images, materials);
}

use appsink::ErrorMessage;

fn monitor_bus(state: Res<State>, appsinks: Res<Assets<AppSinkImage>>) {
    if let Some(appsink) = appsinks.get(&state.appsink_handle) {
        if let Some(msg) = appsink.bus.timed_pop(gst::ClockTime::NONE) {
            use gst::MessageView;

            match msg.view() {
                MessageView::Eos(..) => println!("eos"),
                MessageView::Error(err) => {
                    println!(
                        "{:?}",
                        ErrorMessage {
                            src: msg
                                .src()
                                .map(|s| String::from(s.path_string()))
                                .unwrap_or_else(|| String::from("None")),
                            error: err.error().to_string(),
                            debug: err.debug(),
                            source: err.error(),
                        }
                    );
                }
                _ => (),
            }
        }
    }
}

// fn update_mesh(
//     state: Res<State>,
//     materials: Res<Assets<StandardMaterial>>,
//     meshes: ResMut<Assets<Mesh>>,
// ) {
//     state.update_material(images, materials);
// }
