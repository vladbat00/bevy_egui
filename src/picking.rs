use crate::{
    EguiContext, helpers,
    input::{EguiContextPointerPosition, HoveredNonWindowEguiContext},
};
use bevy_camera::{Camera, NormalizedRenderTarget, RenderTarget};
use bevy_ecs::{
    change_detection::Res,
    component::Component,
    entity::Entity,
    error::Result,
    observer::On,
    prelude::{Commands, Query, With},
};
use bevy_math::Ray3d;
use bevy_picking::{
    Pickable,
    events::{Move, Out, Over, Pointer},
    mesh_picking::ray_cast::RayMeshHit,
    prelude::{MeshRayCast, MeshRayCastSettings, RayCastVisibility},
};
use bevy_transform::components::GlobalTransform;
use bevy_window::PrimaryWindow;

/// This component marks an Entity that displays Egui as an image for [`bevy_picking`] integration
/// (currently, only [`bevy_mesh::Mesh2d`] or [`bevy_mesh::Mesh3d`] are supported for picking).
#[derive(Component)]
#[require(Pickable)]
pub struct PickableEguiContext(pub Entity);

/// Ray-casts a mesh rendering a pickable Egui context and updates its [`EguiContextPointerPosition`] component.
pub fn handle_move_system(
    event: On<Pointer<Move>>,
    mut mesh_ray_cast: MeshRayCast,
    mut egui_pointers: Query<&mut EguiContextPointerPosition>,
    egui_contexts: Query<(&Camera, &GlobalTransform, &RenderTarget), With<EguiContext>>,
    pickable_egui_context_query: Query<&PickableEguiContext>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
) -> Result {
    let NormalizedRenderTarget::Window(_) = event.pointer_location.target else {
        return Ok(());
    };

    // Ray-cast attempting to find the context again.
    // TODO: track https://github.com/bevyengine/bevy/issues/19883 - once it's fixed, we can avoid the double-work with ray-casting again.
    let Ok((context_camera, global_transform, render_target)) = egui_contexts.get(event.hit.camera)
    else {
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
        render_target,
        &bevy_picking::pointer::PointerLocation {
            location: Some(event.pointer_location.clone()),
        },
    ) else {
        return Ok(());
    };
    let &[(hit_entity, RayMeshHit { uv: Some(uv), .. })] = mesh_ray_cast.cast_ray(ray, &settings)
    else {
        return Ok(());
    };

    // At this point, we expect that the context exists, since we checked that with the ray cast filter.
    let &PickableEguiContext(context) = pickable_egui_context_query.get(hit_entity)?;
    let (egui_mesh_camera, _, _) = egui_contexts.get(context)?;

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
    event: On<Pointer<Over>>,
    pickable_egui_context_query: Query<&PickableEguiContext>,
    mut commands: Commands,
) {
    if let Ok(&PickableEguiContext(context)) = pickable_egui_context_query.get(event.entity) {
        commands.insert_resource(HoveredNonWindowEguiContext(context));
    }
}

/// Removes the [`HoveredNonWindowEguiContext`] resource if it contains the Egui context that the pointer has left.
pub fn handle_out_system(
    event: On<Pointer<Out>>,
    pickable_egui_context_query: Query<&PickableEguiContext>,
    mut commands: Commands,
    hovered_non_window_egui_context: Option<Res<HoveredNonWindowEguiContext>>,
) {
    if let Ok(&PickableEguiContext(context)) = pickable_egui_context_query.get(event.entity)
        && hovered_non_window_egui_context
            .as_deref()
            .is_some_and(|&HoveredNonWindowEguiContext(hovered_context)| hovered_context == context)
    {
        commands.remove_resource::<HoveredNonWindowEguiContext>();
    }
}

fn make_ray(
    primary_window_entity: &Query<Entity, With<PrimaryWindow>>,
    camera: &Camera,
    camera_tfm: &GlobalTransform,
    render_target: &RenderTarget,
    pointer_loc: &bevy_picking::pointer::PointerLocation,
) -> Option<Ray3d> {
    let pointer_loc = pointer_loc.location()?;
    if !pointer_loc.is_in_viewport(camera, render_target, primary_window_entity) {
        return None;
    }
    let mut viewport_pos = pointer_loc.position;
    if let Some(viewport) = &camera.viewport {
        let viewport_logical = camera.to_logical(viewport.physical_position)?;
        viewport_pos -= viewport_logical;
    }
    camera.viewport_to_world(camera_tfm, viewport_pos).ok()
}
