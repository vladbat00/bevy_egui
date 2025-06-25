use crate::{render::graph::NodeEgui, EguiContext, EguiRenderOutput, RenderTargetViewport};
use bevy_app::SubApp;
use bevy_core_pipeline::{core_2d::Camera2d, prelude::Camera3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Or, With},
    system::{Commands, Local, Query, ResMut},
    world::World,
};
use bevy_math::{Mat4, UVec4};
use bevy_platform::collections::HashSet;
use bevy_render::{
    camera::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext},
    renderer::RenderContext,
    sync_world::{RenderEntity, TemporaryRenderEntity},
    view::{ExtractedView, RetainedViewEntity},
    Extract, MainWorld,
};

mod render_pass;
use crate::render::graph::SubGraphEgui;
pub use render_pass::*;

pub mod graph {
    use bevy_render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct SubGraphEgui;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeEgui {
        EguiPass,
    }
}

/// A render-world component that lives on the main render target view and
/// specifies the corresponding Egui view.
///
/// For example, if Egui is being rendered to a 3D camera, this component lives on
/// the 3D camera and contains the entity corresponding to the Egui view.
///
/// Entity id of the temporary render entity with the corresponding extracted Egui view.
#[derive(Component, Debug)]
pub struct EguiCameraView(pub Entity);

/// A render-world component that lives on the Egui view and specifies the
/// corresponding main render target view.
///
/// For example, if Egui is being rendered to a 3D camera, this component
/// lives on the Egui view and contains the entity corresponding to the 3D camera.
///
/// This is the inverse of [`EguiCameraView`].
#[derive(Component, Debug)]
pub struct EguiViewTarget(pub Entity);

pub fn get_egui_graph(render_app: &mut SubApp) -> RenderGraph {
    let pass_node = EguiPassNode::new(render_app.world_mut());
    let mut graph = RenderGraph::default();
    graph.add_node(NodeEgui::EguiPass, pass_node);
    graph
}

/// A [`RenderGraphNode`] that executes the Egui rendering subgraph on the Egui
/// view.
pub struct RunEguiSubgraphOnEguiViewNode;

impl Node for RunEguiSubgraphOnEguiViewNode {
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        _: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Fetch the UI view.
        let Some(mut render_views) = world.try_query::<&EguiCameraView>() else {
            return Ok(());
        };
        let Ok(default_camera_view) = render_views.get(world, graph.view_entity()) else {
            return Ok(());
        };

        // Run the subgraph on the Egui view.
        graph.run_sub_graph(SubGraphEgui, vec![], Some(default_camera_view.0))?;
        Ok(())
    }
}

/// Extracts all Egui contexts associated with a camera into the render world.
pub fn extract_egui_camera_view(
    mut commands: Commands,
    mut world: ResMut<MainWorld>,
    // query: Extract<
    //     Query<
    //         (Entity, RenderEntity, &Camera),
    //         (With<EguiContext>, Or<(With<Camera2d>, With<Camera3d>)>),
    //     >,
    // >,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();
    let mut q = world.query::<(Entity, RenderEntity, &Camera, &mut EguiRenderOutput, &RenderTargetViewport)>();

    for (main_entity, render_entity, camera, mut egui_render_output, render_target_viewport) in &mut q.iter_mut(&mut world)
    {
        // Move Egui shapes and textures out of the main world into the render one.
        let egui_render_output = std::mem::take(egui_render_output.as_mut());

        // Ignore inactive cameras.
        if !camera.is_active {
            commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.")
                .remove::<EguiCameraView>();
            continue;
        }

        const UI_CAMERA_FAR: f32 = 1000.0;
        const EGUI_CAMERA_SUBVIEW: u32 = 2095931312;
        const UI_CAMERA_TRANSFORM_OFFSET: f32 = -0.1;

        if let Some(physical_viewport_rect) = camera.physical_viewport_rect() {
            // Use a projection matrix with the origin in the top left instead of the bottom left that comes with OrthographicProjection.
            let projection_matrix = Mat4::orthographic_rh(
                0.0,
                physical_viewport_rect.width() as f32,
                physical_viewport_rect.height() as f32,
                0.0,
                0.0,
                UI_CAMERA_FAR,
            );
            // We use `EGUI_CAMERA_SUBVIEW` here so as not to conflict with the
            // main 3D or 2D camera or UI view, which will have subview index 0 or 1.
            let retained_view_entity =
                RetainedViewEntity::new(main_entity.into(), None, EGUI_CAMERA_SUBVIEW);
            // Creates the UI view.
            let ui_camera_view = commands
                .spawn((
                    ExtractedView {
                        retained_view_entity,
                        clip_from_view: projection_matrix,
                        world_from_view: bevy_transform::components::GlobalTransform::from_xyz(
                            0.0,
                            0.0,
                            UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                        ),
                        clip_from_world: None,
                        hdr: camera.hdr,
                        viewport: UVec4::from((
                            physical_viewport_rect.min,
                            physical_viewport_rect.size(),
                        )),
                        color_grading: Default::default(),
                    },
                    // Link to the main camera view.
                    EguiViewTarget(render_entity),
                    egui_render_output,
                    render_target_viewport.clone(),
                    TemporaryRenderEntity,
                ))
                .id();

            let mut entity_commands = commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.");
            // Link from the main 2D/3D camera view to the UI view.
            entity_commands.insert(EguiCameraView(ui_camera_view));
            live_entities.insert(retained_view_entity);
        }
    }
}
