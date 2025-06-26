use crate::{
    egui_node::DrawPrimitive,
    render::{EguiCameraView, EguiViewTarget},
    render_systems::{EguiPipelines, EguiRenderData, EguiTextureBindGroups, EguiTransforms},
};
use bevy_ecs::{entity::Entity, query::QueryState, world::World};
use bevy_ecs::world::Mut;
use bevy_math::UVec2;
use bevy_render::{
    camera::{ExtractedCamera, Viewport},
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::TrackedRenderPass,
    render_resource::{
        CommandEncoderDescriptor, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    sync_world::{MainEntity, RenderEntity},
    view::{ExtractedView, ViewTarget},
};
use wgpu_types::{IndexFormat, Operations, StoreOp};

pub struct EguiPassNode {
    egui_view_query: QueryState<(&'static ExtractedView, &'static EguiViewTarget)>,
    egui_view_target_query: QueryState<(&'static ViewTarget, &'static ExtractedCamera)>,
}

impl EguiPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            egui_view_query: world.query_filtered(),
            egui_view_target_query: world.query(),
        }
    }
}

impl Node for EguiPassNode {
    fn update(&mut self, world: &mut World) {
        self.egui_view_query.update_archetypes(world);
        self.egui_view_target_query.update_archetypes(world);

        world.resource_scope(|world, mut render_data: Mut<EguiRenderData>| {
            for (_main_entity, data) in &mut render_data.0 {
                let (Some(render_target_size), Some(key)) = (data.render_target_size, data.key) else {
                    bevy_log::warn!("Failed to retrieve egui node data!");
                    return;
                };

                for (clip_rect, command) in data.postponed_updates.drain(..) {
                    let info = egui::PaintCallbackInfo {
                        viewport: command.rect,
                        clip_rect,
                        pixels_per_point: data.pixels_per_point,
                        screen_size_px: [
                            render_target_size.target_size.x as u32,
                            render_target_size.target_size.y as u32,
                        ],
                    };
                    command
                        .callback
                        .cb()
                        .update(info, data.render_entity, key, world);
                }
            }
        });
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let egui_pipelines = &world.resource::<EguiPipelines>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_data = world.resource::<EguiRenderData>();

        // Extract the UI view.
        let input_view_entity = graph.view_entity();

        // Query the UI view components.
        let Ok((view, view_target)) = self.egui_view_query.get_manual(world, input_view_entity)
        else {
            return Ok(());
        };

        let Ok((target, camera)) = self.egui_view_target_query.get_manual(world, view_target.0)
        else {
            return Ok(());
        };

        let Some(data) = render_data.0.get(&view.retained_view_entity.main_entity) else {
            bevy_log::warn!("Failed to retrieve render data for egui node rendering!");
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("egui_pass"),
            color_attachments: &[Some(target.get_unsampled_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let Some(viewport) = camera.viewport.clone().or_else(|| {
            camera.physical_viewport_size.map(|size| Viewport {
                physical_position: UVec2::ZERO,
                physical_size: size,
                ..Default::default()
            })
        }) else {
            return Ok(());
        };
        render_pass.set_camera_viewport(&viewport);
        render_pass.set_camera_viewport(&Viewport {
            physical_position: UVec2::ZERO,
            physical_size: camera.physical_target_size.unwrap(),
            ..Default::default()
        });

        let mut requires_reset = true;
        let mut last_scissor_rect = None;

        let pipeline_id = egui_pipelines
            .get(&view.retained_view_entity.main_entity)
            .expect("Expected a queued pipeline");
        let Some(pipeline) = pipeline_cache.get_render_pipeline(*pipeline_id) else {
            return Ok(());
        };

        let bind_groups = world.resource::<EguiTextureBindGroups>();
        let egui_transforms = world.resource::<EguiTransforms>();
        let transform_buffer_offset =
            egui_transforms.offsets[&view.retained_view_entity.main_entity];
        let transform_buffer_bind_group = &egui_transforms
            .bind_group
            .as_ref()
            .expect("Expected a prepared bind group")
            .1;

        let (vertex_buffer, index_buffer) = match (&data.vertex_buffer, &data.index_buffer) {
            (Some(vertex), Some(index)) => (vertex, index),
            _ => {
                return Ok(());
            }
        };

        let mut vertex_offset: u32 = 0;
        for draw_command in &data.draw_commands {
            if requires_reset {
                render_pass.set_render_pipeline(pipeline);
                render_pass.set_bind_group(
                    0,
                    transform_buffer_bind_group,
                    &[transform_buffer_offset],
                );
                requires_reset = false;
            }

            let clip_urect = bevy_math::URect {
                min: bevy_math::UVec2 {
                    x: (draw_command.clip_rect.min.x * data.pixels_per_point).round() as u32,
                    y: (draw_command.clip_rect.min.y * data.pixels_per_point).round() as u32,
                },
                max: bevy_math::UVec2 {
                    x: (draw_command.clip_rect.max.x * data.pixels_per_point).round() as u32,
                    y: (draw_command.clip_rect.max.y * data.pixels_per_point).round() as u32,
                },
            };

            // TODO!
            let scissor_rect = clip_urect.intersect(bevy_math::URect::new(
                0,
                0,
                viewport.physical_size.x,
                viewport.physical_size.y,
            ));
            // if scissor_rect.is_empty() {
            //     continue;
            // }

            if Some(scissor_rect) != last_scissor_rect {
                last_scissor_rect = Some(scissor_rect);

                // Bevy TrackedRenderPass doesn't track set_scissor_rect calls,
                // so set_scissor_rect is updated only when it is needed.
                // render_pass.set_scissor_rect(
                //     scissor_rect.min.x,
                //     scissor_rect.min.y,
                //     scissor_rect.width(),
                //     scissor_rect.height(),
                // );
            }

            let Some(pipeline_key) = data.key else {
                continue;
            };
            match &draw_command.primitive {
                DrawPrimitive::Egui(command) => {
                    let texture_bind_group = match bind_groups.get(&command.egui_texture) {
                        Some(texture_resource) => texture_resource,
                        None => {
                            vertex_offset += command.vertices_count as u32;
                            continue;
                        }
                    };

                    render_pass.set_bind_group(1, texture_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), 0, IndexFormat::Uint32);

                    render_pass.draw_indexed(
                        vertex_offset..(vertex_offset + command.vertices_count as u32),
                        0,
                        0..1,
                    );

                    vertex_offset += command.vertices_count as u32;
                }
                DrawPrimitive::PaintCallback(command) => {
                    let info = egui::PaintCallbackInfo {
                        viewport: command.rect,
                        clip_rect: draw_command.clip_rect,
                        pixels_per_point: data.pixels_per_point,
                        screen_size_px: [viewport.physical_size.x, viewport.physical_size.y],
                    };

                    let viewport = info.viewport_in_pixels();
                    if viewport.width_px > 0 && viewport.height_px > 0 {
                        requires_reset = true;
                        render_pass.set_viewport(
                            viewport.left_px as f32,
                            viewport.top_px as f32,
                            viewport.width_px as f32,
                            viewport.height_px as f32,
                            0.,
                            1.,
                        );

                        command.callback.cb().render(
                            info,
                            &mut render_pass,
                            RenderEntity::from(view_target.0),
                            pipeline_key,
                            world,
                        );
                    }
                }
            }
        }

        // drop(render_pass);
        // command_encoder.finish();

        // render_context.add_command_buffer_generation_task(move |device| {
        //     let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        //         label: Some("egui_node_command_encoder"),
        //     });
        //
        //     let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
        //         label: Some("egui_pass"),
        //         color_attachments: &[Some(RenderPassColorAttachment {
        //             view: swap_chain_texture_view,
        //             resolve_target: None,
        //             ops: Operations {
        //                 load: load_op,
        //                 store: StoreOp::Store,
        //             },
        //         })],
        //         depth_stencil_attachment: None,
        //         timestamp_writes: None,
        //         occlusion_query_set: None,
        //     });
        //     let mut render_pass = TrackedRenderPass::new(&device, render_pass);
        // });

        Ok(())
    }
}
