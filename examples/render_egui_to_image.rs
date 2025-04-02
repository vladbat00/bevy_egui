use bevy::{prelude::*, render::render_resource::LoadOp, window::PrimaryWindow};
use bevy_egui::{BevyEguiApp, EguiContexts, EguiPlugin, EguiRenderToImage, OnEguiPass};
use wgpu_types::{Extent3d, TextureUsages};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, MeshPickingPlugin));
    app.add_plugins(EguiPlugin {
        default_to_multipass: true,
    });
    app.add_systems(Startup, setup_worldspace_system);
    app.add_systems(Update, draw_gizmos_system);
    app.add_egui_system(update_screenspace_system);
    app.run();
}

struct Name(String);

impl Default for Name {
    fn default() -> Self {
        Self("%username%".to_string())
    }
}

fn update_screenspace_system(
    _trigger: Trigger<OnEguiPass>,
    mut name: Local<Name>,
    mut contexts: EguiContexts,
) {
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

fn update_worldspace_system(
    trigger: Trigger<OnEguiPass>,
    mut name: Local<Name>,
    mut contexts: EguiContexts,
) {
    egui::Window::new("Worldspace UI").show(contexts.ctx_for_entity_mut(trigger.entity()), |ui| {
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

#[derive(Resource)]
struct MaterialHandles {
    normal: Handle<StandardMaterial>,
    hovered: Handle<StandardMaterial>,
}

fn setup_worldspace_system(
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

    let material_handles = MaterialHandles {
        normal: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.4, 0.4, 0.4),
            ..default()
        }),
        hovered: materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.6, 0.6, 0.6),
            ..default()
        }),
    };

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
            // We want the "tablet" mesh behind to react to pointer inputs.
            PickingBehavior {
                should_block_lower: false,
                is_hoverable: true,
            },
        ))
        .observe(update_worldspace_system)
        .with_children(|commands| {
            // The "tablet" mesh, on top of which Egui is rendered.
            commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.1, 1.1, 0.1))),
                    MeshMaterial3d(material_handles.normal.clone()),
                    Transform::from_xyz(0.0, 0.0, -0.051),
                ))
                .observe(handle_over_system)
                .observe(handle_out_system)
                .observe(handle_drag_system);
        });

    commands.spawn((
        PointLight::default(),
        Transform::from_translation(Vec3::new(5.0, 3.0, 10.0)),
    ));

    let camera_transform = Transform::from_xyz(1.0, 1.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y);
    commands.spawn((Camera3d::default(), camera_transform));

    commands.insert_resource(material_handles);
}

fn draw_gizmos_system(
    mut gizmos: Gizmos,
    egui_mesh_query: Query<&Transform, With<EguiRenderToImage>>,
) {
    let egui_mesh_transform = egui_mesh_query.single();
    gizmos.axes(*egui_mesh_transform, 0.1);
}

fn handle_over_system(
    over: Trigger<Pointer<Over>>,
    mut mesh_material_query: Query<&mut MeshMaterial3d<StandardMaterial>>,
    material_handles: Res<MaterialHandles>,
) {
    let Ok(mut material) = mesh_material_query.get_mut(over.target) else {
        return;
    };
    *material = MeshMaterial3d(material_handles.hovered.clone());
}

fn handle_out_system(
    out: Trigger<Pointer<Out>>,
    mut mesh_material_query: Query<&mut MeshMaterial3d<StandardMaterial>>,
    material_handles: Res<MaterialHandles>,
) {
    let Ok(mut material) = mesh_material_query.get_mut(out.target) else {
        return;
    };
    *material = MeshMaterial3d(material_handles.normal.clone());
}

fn handle_drag_system(
    drag: Trigger<Pointer<Drag>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut egui_mesh_transform: Single<&mut Transform, With<EguiRenderToImage>>,
    // Need to specify `Without<EguiRenderToImage>` for `camera_query` and `egui_mesh_query` to be disjoint.
    camera_transform: Single<&Transform, (With<Camera>, Without<EguiRenderToImage>)>,
) {
    let Some(delta_normalized) = Vec3::new(drag.delta.y, drag.delta.x, 0.0).try_normalize() else {
        return;
    };

    let angle = Vec2::new(
        drag.delta.x / window.physical_width() as f32,
        drag.delta.y / window.physical_height() as f32,
    )
    .length()
        * std::f32::consts::PI
        * 2.0;
    let frame_delta = Quat::from_axis_angle(delta_normalized, angle);

    let camera_rotation = camera_transform.rotation;
    egui_mesh_transform.rotation =
        camera_rotation * frame_delta * camera_rotation.conjugate() * egui_mesh_transform.rotation;
}
