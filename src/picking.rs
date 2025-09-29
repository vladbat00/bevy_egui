use crate::{
    EguiContext, helpers,
    input::{EguiContextPointerPosition, HoveredNonWindowEguiContext},
};
use bevy_asset::Assets;
use bevy_ecs::{
    change_detection::Res,
    component::Component,
    entity::Entity,
    error::Result,
    observer::Trigger,
    prelude::{AnyOf, Commands, Query, With},
};
use bevy_math::{Ray3d, Vec2};
use bevy_picking::{
    Pickable,
    events::{Move, Out, Over, Pointer},
    mesh_picking::ray_cast::RayMeshHit,
    prelude::{MeshRayCast, MeshRayCastSettings, RayCastVisibility},
};
use bevy_render::{
    camera::{Camera, NormalizedRenderTarget},
    mesh::{Indices, Mesh, Mesh2d, Mesh3d, VertexAttributeValues},
};
use bevy_transform::components::GlobalTransform;
use bevy_window::PrimaryWindow;
use wgpu_types::PrimitiveTopology;

/// This component marks an Entity that displays Egui as an image for [`bevy_picking`] integration
/// (currently, only [`bevy_render::mesh::Mesh2d`] or [`bevy_render::mesh::Mesh3d`] are supported for picking).
#[derive(Component)]
#[require(Pickable)]
pub struct PickableEguiContext(pub Entity);

/// Ray-casts a mesh rendering a pickable Egui context and updates its [`EguiContextPointerPosition`] component.
pub fn handle_move_system(
    trigger: Trigger<Pointer<Move>>,
    mut mesh_ray_cast: MeshRayCast,
    mut egui_pointers: Query<&mut EguiContextPointerPosition>,
    egui_contexts: Query<(&Camera, &GlobalTransform), With<EguiContext>>,
    pickable_egui_context_query: Query<(&PickableEguiContext, AnyOf<(&Mesh2d, &Mesh3d)>)>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    meshes: Res<Assets<Mesh>>,
) -> Result {
    let NormalizedRenderTarget::Window(_) = trigger.pointer_location.target else {
        return Ok(());
    };

    // Ray-cast attempting to find the context again.
    // TODO: track https://github.com/bevyengine/bevy/issues/19883 - once it's fixed, we can avoid the double-work with ray-casting again.
    let Ok((context_camera, global_transform)) = egui_contexts.get(trigger.hit.camera) else {
        return Ok(());
    };
    let settings = MeshRayCastSettings {
        visibility: RayCastVisibility::Any,
        filter: &|entity| pickable_egui_context_query.contains(entity),
        early_exit_test: &|_| true,
    };
    let Some(ray) = make_ray(
        &primary_window_query,
        context_camera,
        global_transform,
        &bevy_picking::pointer::PointerLocation {
            location: Some(trigger.pointer_location.clone()),
        },
    ) else {
        return Ok(());
    };
    let &[
        (
            hit_entity,
            RayMeshHit {
                triangle_index: Some(triangle_index),
                barycentric_coords,
                ..
            },
        ),
    ] = mesh_ray_cast.cast_ray(ray, &settings)
    else {
        return Ok(());
    };

    // At this point, we expect that the context exists, since we checked that with the ray cast filter.
    let (&PickableEguiContext(context), mesh) = pickable_egui_context_query.get(hit_entity)?;
    let (egui_mesh_camera, _) = egui_contexts.get(context)?;

    // Read triangle indices and the respective UVs of the mesh.
    let handle = match mesh {
        (Some(handle), None) => handle.0.clone(),
        (None, Some(handle)) => handle.0.clone(),
        _ => unreachable!(),
    };
    let Some(mesh) = meshes.get(handle.id()) else {
        return Ok(());
    };
    // The bevy_picking ray cast backend expects only the TriangleList primitive topology (at least that was the case at the moment of writing).
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        panic!(
            "Unexpected primitive topology for a picked mesh ({:?}): {:?}",
            trigger.target,
            mesh.primitive_topology()
        );
    }
    let Some(indices) = mesh.indices() else {
        return Ok(());
    };
    let Some(uv_values) =
        mesh.attribute(Mesh::ATTRIBUTE_UV_0)
            .and_then(|values| match (values, indices) {
                (VertexAttributeValues::Float32x2(uvs), Indices::U16(indices)) => {
                    uv_values_for_triangle(indices, triangle_index, uvs)
                }
                (VertexAttributeValues::Float32x2(uvs), Indices::U32(indices)) => {
                    uv_values_for_triangle(indices, triangle_index, uvs)
                }
                _ => None,
            })
    else {
        return Ok(());
    };

    // Interpolate UVs based on the barycentric coordinates.
    let uv = Vec2::from_array(uv_values[0]) * barycentric_coords.x
        + Vec2::from_array(uv_values[1]) * barycentric_coords.y
        + Vec2::from_array(uv_values[2]) * barycentric_coords.z;

    // The only thing we need to do here from the Egui context perspective is to update the `EguiContextPointerPosition` component.
    // Other input systems will take care of the rest.
    let Some(viewport_size) = egui_mesh_camera.logical_target_size() else {
        return Ok(());
    };
    egui_pointers.get_mut(context)?.position = helpers::vec2_into_egui_pos2(viewport_size * uv);

    Ok(())
}

/// Inserts the [`HoveredNonWindowEguiContext`] resource containing the hovered Egui context.
pub fn handle_over_system(
    trigger: Trigger<Pointer<Over>>,
    pickable_egui_context_query: Query<&PickableEguiContext>,
    mut commands: Commands,
) {
    if let Ok(&PickableEguiContext(context)) = pickable_egui_context_query.get(trigger.target) {
        commands.insert_resource(HoveredNonWindowEguiContext(context));
    }
}

/// Removes the [`HoveredNonWindowEguiContext`] resource if it contains the Egui context that the pointer has left.
pub fn handle_out_system(
    trigger: Trigger<Pointer<Out>>,
    pickable_egui_context_query: Query<&PickableEguiContext>,
    mut commands: Commands,
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
) {
    if let Ok(&PickableEguiContext(context)) = pickable_egui_context_query.get(trigger.target) {
        if hovered_non_window_egui_context
            .as_deref()
            .is_some_and(|&HoveredNonWindowEguiContext(hovered_context)| hovered_context == context)
        {
            commands.remove_resource::<HoveredNonWindowEguiContext>();
        }
    }
}

fn uv_values_for_triangle<I: TryInto<usize> + Clone + Copy>(
    indices: &[I],
    triangle_index: usize,
    values: &[[f32; 2]],
) -> Option<[[f32; 2]; 3]> {
    if indices.len() % 3 != 0 || triangle_index >= indices.len() {
        return None;
    }

    let i0 = indices[triangle_index * 3].try_into().ok()?;
    let i1 = indices[triangle_index * 3 + 1].try_into().ok()?;
    let i2 = indices[triangle_index * 3 + 2].try_into().ok()?;

    Some([*values.get(i1)?, *values.get(i2)?, *values.get(i0)?])
}

fn make_ray(
    primary_window_entity: &Query<Entity, With<PrimaryWindow>>,
    camera: &Camera,
    camera_tfm: &GlobalTransform,
    pointer_loc: &bevy_picking::pointer::PointerLocation,
) -> Option<Ray3d> {
    let pointer_loc = pointer_loc.location()?;
    if !pointer_loc.is_in_viewport(camera, primary_window_entity) {
        return None;
    }
    let mut viewport_pos = pointer_loc.position;
    if let Some(viewport) = &camera.viewport {
        let viewport_logical = camera.to_logical(viewport.physical_position)?;
        viewport_pos -= viewport_logical;
    }
    camera.viewport_to_world(camera_tfm, viewport_pos).ok()
}
