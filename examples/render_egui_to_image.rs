use bevy::{
    input::mouse::MouseMotion, prelude::*, render::render_resource::LoadOp, window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiRenderToImage};
use wgpu_types::{Extent3d, TextureUsages};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(EguiPlugin);
    app.add_systems(Startup, setup_worldspace);
    app.add_systems(
        Update,
        (
            update_screenspace,
            update_worldspace,
            handle_dragging,
            draw_gizmos.after(handle_dragging),
        ),
    );
    app.run();
}

struct Name(String);

impl Default for Name {
    fn default() -> Self {
        Self("%username%".to_string())
    }
}

fn update_screenspace(mut name: Local<Name>, mut contexts: EguiContexts) {
    egui::Window::new("Screenspace UI").show(contexts.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Your name:");
            ui.text_edit_singleline(&mut name.0);
        });
        ui.label(format!(
            "Hi {}, I'm rendering to an image in screenspace!",
            name.0
        ));
    });
}

fn update_worldspace(
    mut name: Local<Name>,
    mut ctx: Single<&mut bevy_egui::EguiContext, With<EguiRenderToImage>>,
) {
    egui::Window::new("Worldspace UI").show(ctx.get_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Your name:");
            ui.text_edit_singleline(&mut name.0);
        });
        ui.label(format!(
            "Hi {}, I'm rendering to an image in worldspace!",
            name.0
        ));
    });
}

fn setup_worldspace(
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
) {
    for (_, config, _) in config_store.iter_mut() {
        config.depth_bias = -1.0;
    }

    let image = images.add({
        let size = Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        };
        let mut image = Image {
            // You should use `0` so that the pixels are transparent.
            data: vec![0; (size.width * size.height * 4) as usize],
            ..default()
        };
        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        image.texture_descriptor.size = size;
        image
    });

    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(0.5)).mesh())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(Handle::clone(&image)),
                alpha_mode: AlphaMode::Blend,
                // Remove this if you want it to use the world's lighting.
                unlit: true,
                ..default()
            })),
            EguiRenderToImage {
                handle: image,
                load_op: LoadOp::Clear(Color::srgb_u8(43, 44, 47).to_linear().into()),
            },
        ))
        .with_child((
            Mesh3d(meshes.add(Cuboid::new(1.1, 1.1, 0.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.4, 0.4, 0.4),
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, -0.051),
        ));

    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(5.0, 3.0, 10.0)),
    ));

    let camera_transform = Transform::from_xyz(1.0, 1.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y);
    commands.spawn((Camera3d::default(), camera_transform));
}

fn draw_gizmos(mut gizmos: Gizmos, egui_mesh_query: Query<&Transform, With<EguiRenderToImage>>) {
    let egui_mesh_transform = egui_mesh_query.single();
    gizmos.axes(*egui_mesh_transform, 0.1);
}

fn handle_dragging(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut egui_mesh_query: Query<&mut Transform, With<EguiRenderToImage>>,
    // Need to specify `Without<EguiRenderToImage>` for `camera_query` and `egui_mesh_query` to be disjoint.
    camera_query: Query<&Transform, (With<Camera>, Without<EguiRenderToImage>)>,
    mut local_state: Local<(Quat, Quat)>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        mouse_motion_events.clear();
        return;
    }

    let window = window_query.single();
    let camera_transform = camera_query.single();

    let mut egui_mesh_transform = egui_mesh_query.single_mut();

    let (initial_rotation, delta) = &mut *local_state;

    if mouse_button_input.just_pressed(MouseButton::Left) {
        *initial_rotation = egui_mesh_transform.rotation;
        *delta = Quat::IDENTITY;
    }

    for ev in mouse_motion_events.read() {
        let angle = Vec2::new(
            ev.delta.x / window.physical_width() as f32,
            ev.delta.y / window.physical_height() as f32,
        )
        .length()
            * std::f32::consts::PI
            * 2.0;
        let frame_delta =
            Quat::from_axis_angle(Vec3::new(ev.delta.y, ev.delta.x, 0.0).normalize(), angle);
        *delta = frame_delta * *delta;

        let camera_rotation = camera_transform.rotation;
        egui_mesh_transform.rotation =
            camera_rotation * *delta * camera_rotation.conjugate() * *initial_rotation;
    }
}
