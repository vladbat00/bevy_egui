use crate::{
    egui_node::{
        DrawCommand, DrawPrimitive, EguiBevyPaintCallback, EguiDraw, EguiNode, EguiPipeline,
        EguiPipelineKey, EguiRenderTargetType, PaintCallbackDraw,
    },
    EguiContext, EguiContextSettings, EguiManagedTextures, EguiRenderOutput, EguiRenderToImage,
    EguiUserTextures, RenderTargetSize,
};
use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::Image;
use bevy_log as log;
use bevy_math::Vec2;
use bevy_render::{
    extract_resource::ExtractResource,
    render_asset::RenderAssets,
    render_graph::{RenderGraph, RenderLabel},
    render_resource::{
        BindGroup, BindGroupEntry, BindingResource, Buffer, BufferDescriptor, BufferId,
        CachedRenderPipelineId, DynamicUniformBuffer, PipelineCache, SpecializedRenderPipelines,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity},
    texture::GpuImage,
    view::ExtractedWindows,
    Extract,
};
use bevy_utils::HashMap;
use bevy_window::Window;
use bytemuck::cast_slice;
use wgpu_types::{BufferAddress, BufferUsages};

/// Extracted Egui settings.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct ExtractedEguiSettings(pub EguiContextSettings);

/// The extracted version of [`EguiManagedTextures`].
#[derive(Debug, Resource)]
pub struct ExtractedEguiManagedTextures(pub HashMap<(Entity, u64), Handle<Image>>);
impl ExtractResource for ExtractedEguiManagedTextures {
    type Source = EguiManagedTextures;

    fn extract_resource(source: &Self::Source) -> Self {
        Self(source.iter().map(|(k, v)| (*k, v.handle.clone())).collect())
    }
}

/// Corresponds to Egui's [`egui::TextureId`].
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum EguiTextureId {
    /// Textures allocated via Egui.
    Managed(MainEntity, u64),
    /// Textures allocated via Bevy.
    User(u64),
}

/// Extracted Egui textures.
#[derive(SystemParam)]
pub struct ExtractedEguiTextures<'w> {
    /// Maps Egui managed texture ids to Bevy image handles.
    pub egui_textures: Res<'w, ExtractedEguiManagedTextures>,
    /// Maps Bevy managed texture handles to Egui user texture ids.
    pub user_textures: Res<'w, EguiUserTextures>,
}

/// [`RenderLabel`] type for the Egui pass.
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct EguiPass {
    /// Index of the window entity.
    pub entity_index: u32,
    /// Generation of the window entity.
    pub entity_generation: u32,
    /// Render target type (e.g. window, image).
    pub render_target_type: EguiRenderTargetType,
}

impl EguiPass {
    /// Creates a pass from a window Egui context.
    pub fn from_window_entity(entity: Entity) -> Self {
        Self {
            entity_index: entity.index(),
            entity_generation: entity.generation(),
            render_target_type: EguiRenderTargetType::Window,
        }
    }

    /// Creates a pass from a "render to image" Egui context.
    pub fn from_render_to_image_entity(entity: Entity) -> Self {
        Self {
            entity_index: entity.index(),
            entity_generation: entity.generation(),
            render_target_type: EguiRenderTargetType::Image,
        }
    }
}

impl ExtractedEguiTextures<'_> {
    /// Returns an iterator over all textures (both Egui and Bevy managed).
    pub fn handles(&self) -> impl Iterator<Item = (EguiTextureId, AssetId<Image>)> + '_ {
        self.egui_textures
            .0
            .iter()
            .map(|(&(window, texture_id), managed_tex)| {
                (
                    EguiTextureId::Managed(MainEntity::from(window), texture_id),
                    managed_tex.id(),
                )
            })
            .chain(
                self.user_textures
                    .textures
                    .iter()
                    .map(|(handle, id)| (EguiTextureId::User(*id), handle.id())),
            )
    }
}

/// Sets up render nodes for newly created Egui contexts.
pub fn setup_new_egui_nodes_system(
    windows: Extract<
        Query<(Entity, &RenderEntity, AnyOf<(&Window, &EguiRenderToImage)>), Added<EguiContext>>,
    >,
    mut render_graph: ResMut<RenderGraph>,
) {
    for (main_entity, render_entity, (window, render_to_image)) in windows.iter() {
        let egui_pass = EguiPass::from_window_entity(main_entity);
        let new_node = EguiNode::new(
            MainEntity::from(main_entity),
            *render_entity,
            match (window.is_some(), render_to_image.is_some()) {
                (true, false) => EguiRenderTargetType::Window,
                (false, true) => EguiRenderTargetType::Image,
                (true, true) => {
                    log::error!(
                        "Failed to set up an Egui node: can't render both to a window and an image"
                    );
                    continue;
                }
                (false, false) => unreachable!(),
            },
        );

        render_graph.add_node(egui_pass.clone(), new_node);

        render_graph.add_node_edge(bevy_render::graph::CameraDriverLabel, egui_pass);
    }
}

/// Tears render nodes down for deleted window Egui contexts.
pub fn teardown_window_nodes_system(
    mut removed_windows: Extract<RemovedComponents<Window>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    for window_entity in removed_windows.read() {
        if let Err(err) = render_graph.remove_node(EguiPass::from_window_entity(window_entity)) {
            log::error!("Failed to remove a render graph node: {err:?}");
        }
    }
}

/// Tears render nodes down for deleted "render to texture" Egui contexts.
pub fn teardown_render_to_image_nodes_system(
    mut removed_windows: Extract<RemovedComponents<EguiRenderToImage>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    for window_entity in removed_windows.read() {
        if let Err(err) =
            render_graph.remove_node(EguiPass::from_render_to_image_entity(window_entity))
        {
            log::error!("Failed to remove a render graph node: {err:?}");
        }
    }
}

/// Describes the transform buffer.
#[derive(Resource, Default)]
pub struct EguiTransforms {
    /// Uniform buffer.
    pub buffer: DynamicUniformBuffer<EguiTransform>,
    /// The Entity is from the main world.
    pub offsets: HashMap<MainEntity, u32>,
    /// Bind group.
    pub bind_group: Option<(BufferId, BindGroup)>,
}

/// Scale and translation for rendering Egui shapes. Is needed to transform Egui coordinates from
/// the screen space with the center at (0, 0) to the normalised viewport space.
#[derive(encase::ShaderType, Default)]
pub struct EguiTransform {
    /// Is affected by window size and [`EguiContextSettings::scale_factor`].
    pub scale: Vec2,
    /// Normally equals `Vec2::new(-1.0, 1.0)`.
    pub translation: Vec2,
}

impl EguiTransform {
    /// Calculates the transform from window size and scale factor.
    pub fn from_render_target_size(
        render_target_size: RenderTargetSize,
        scale_factor: f32,
    ) -> Self {
        EguiTransform {
            scale: Vec2::new(
                2.0 / (render_target_size.width() / scale_factor),
                -2.0 / (render_target_size.height() / scale_factor),
            ),
            translation: Vec2::new(-1.0, 1.0),
        }
    }
}

/// Prepares Egui transforms.
pub fn prepare_egui_transforms_system(
    mut egui_transforms: ResMut<EguiTransforms>,
    render_targets: Query<(Option<&MainEntity>, &EguiContextSettings, &RenderTargetSize)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    egui_pipeline: Res<EguiPipeline>,
) {
    egui_transforms.buffer.clear();
    egui_transforms.offsets.clear();

    for (window_main, egui_settings, size) in render_targets.iter() {
        let offset = egui_transforms
            .buffer
            .push(&EguiTransform::from_render_target_size(
                *size,
                egui_settings.scale_factor,
            ));
        if let Some(window_main) = window_main {
            egui_transforms.offsets.insert(*window_main, offset);
        }
    }

    egui_transforms
        .buffer
        .write_buffer(&render_device, &render_queue);

    if let Some(buffer) = egui_transforms.buffer.buffer() {
        match egui_transforms.bind_group {
            Some((id, _)) if buffer.id() == id => {}
            _ => {
                let transform_bind_group = render_device.create_bind_group(
                    Some("egui transform bind group"),
                    &egui_pipeline.transform_bind_group_layout,
                    &[BindGroupEntry {
                        binding: 0,
                        resource: egui_transforms.buffer.binding().unwrap(),
                    }],
                );
                egui_transforms.bind_group = Some((buffer.id(), transform_bind_group));
            }
        };
    }
}

/// Maps Egui textures to bind groups.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct EguiTextureBindGroups(pub HashMap<EguiTextureId, BindGroup>);

/// Queues bind groups.
pub fn queue_bind_groups_system(
    mut commands: Commands,
    egui_textures: ExtractedEguiTextures,
    render_device: Res<RenderDevice>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    egui_pipeline: Res<EguiPipeline>,
) {
    let bind_groups = egui_textures
        .handles()
        .filter_map(|(texture, handle_id)| {
            let gpu_image = gpu_images.get(&Handle::Weak(handle_id))?;
            let bind_group = render_device.create_bind_group(
                None,
                &egui_pipeline.texture_bind_group_layout,
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&gpu_image.texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&gpu_image.sampler),
                    },
                ],
            );
            Some((texture, bind_group))
        })
        .collect();

    commands.insert_resource(EguiTextureBindGroups(bind_groups))
}

/// Cached Pipeline IDs for the specialized instances of `EguiPipeline`.
#[derive(Resource)]
pub struct EguiPipelines(pub HashMap<MainEntity, CachedRenderPipelineId>);

/// Queue [`EguiPipeline`] instances specialized on each window's swap chain texture format.
pub fn queue_pipelines_system(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<EguiPipeline>>,
    egui_pipeline: Res<EguiPipeline>,
    windows: Res<ExtractedWindows>,
    render_to_image: Query<(&MainEntity, &EguiRenderToImage)>,
    images: Res<RenderAssets<GpuImage>>,
) {
    let mut pipelines: HashMap<MainEntity, CachedRenderPipelineId> = windows
        .iter()
        .filter_map(|(window_id, window)| {
            let key = EguiPipelineKey::from_extracted_window(window)?;
            let pipeline_id =
                specialized_pipelines.specialize(&pipeline_cache, &egui_pipeline, key);
            Some((MainEntity::from(*window_id), pipeline_id))
        })
        .collect();

    pipelines.extend(
        render_to_image
            .iter()
            .filter_map(|(main_entity, render_to_image)| {
                let img = images.get(&render_to_image.handle)?;
                let key = EguiPipelineKey::from_gpu_image(img);
                let pipeline_id =
                    specialized_pipelines.specialize(&pipeline_cache, &egui_pipeline, key);

                Some((*main_entity, pipeline_id))
            }),
    );

    commands.insert_resource(EguiPipelines(pipelines));
}

/// Cached Pipeline IDs for the specialized instances of `EguiPipeline`.
#[derive(Default, Resource)]
pub struct EguiRenderData(pub(crate) HashMap<MainEntity, EguiRenderTargetData>);

#[derive(Default)]
pub(crate) struct EguiRenderTargetData {
    keep: bool,
    pub(crate) vertex_data: Vec<u8>,
    pub(crate) vertex_buffer_capacity: usize,
    pub(crate) vertex_buffer: Option<Buffer>,
    pub(crate) index_data: Vec<u32>,
    pub(crate) index_buffer_capacity: usize,
    pub(crate) index_buffer: Option<Buffer>,
    pub(crate) draw_commands: Vec<DrawCommand>,
    pub(crate) postponed_updates: Vec<(egui::Rect, PaintCallbackDraw)>,
    pub(crate) pixels_per_point: f32,
    pub(crate) key: Option<EguiPipelineKey>,
    pub(crate) render_target_size: Option<RenderTargetSize>,
}

/// Prepares Egui transforms.
pub fn prepare_egui_render_target_data(
    mut render_data: ResMut<EguiRenderData>,
    render_targets: Query<(
        &MainEntity,
        &EguiContextSettings,
        &RenderTargetSize,
        &EguiRenderOutput,
        Option<&EguiRenderToImage>,
    )>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_windows: Res<ExtractedWindows>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let render_data = &mut render_data.0;
    render_data.retain(|_, data| {
        let keep = data.keep;
        data.keep = false;
        keep
    });

    for (main_entity, egui_settings, render_target_size, render_output, render_to_image) in
        render_targets.iter()
    {
        let data = render_data.entry(*main_entity).or_default();

        data.keep = true;

        let render_target_size = *render_target_size;
        let egui_settings = egui_settings.clone();
        let image_handle =
            render_to_image.map(|render_to_image| render_to_image.handle.clone_weak());

        data.render_target_size = Some(render_target_size);

        let render_target_type = if render_to_image.is_some() {
            EguiRenderTargetType::Image
        } else {
            EguiRenderTargetType::Window
        };

        // Construct a pipeline key based on a render target.
        let key = match render_target_type {
            EguiRenderTargetType::Window => {
                let Some(key) = extracted_windows
                    .windows
                    .get(&main_entity.id())
                    .and_then(EguiPipelineKey::from_extracted_window)
                else {
                    continue;
                };
                key
            }
            EguiRenderTargetType::Image => {
                let image_handle = image_handle
                    .expect("Expected an image handle for a render to image node")
                    .clone();
                let Some(key) = gpu_images
                    .get(&image_handle)
                    .map(EguiPipelineKey::from_gpu_image)
                else {
                    continue;
                };
                key
            }
        };
        data.key = Some(key);

        data.pixels_per_point = render_target_size.scale_factor * egui_settings.scale_factor;
        if render_target_size.physical_width == 0.0 || render_target_size.physical_height == 0.0 {
            continue;
        }

        let mut index_offset = 0;

        data.draw_commands.clear();
        data.vertex_data.clear();
        data.index_data.clear();
        data.postponed_updates.clear();

        for egui::epaint::ClippedPrimitive {
            clip_rect,
            primitive,
        } in render_output.paint_jobs.as_slice()
        {
            let clip_rect = *clip_rect;

            let clip_urect = bevy_math::URect {
                min: bevy_math::UVec2 {
                    x: (clip_rect.min.x * data.pixels_per_point).round() as u32,
                    y: (clip_rect.min.y * data.pixels_per_point).round() as u32,
                },
                max: bevy_math::UVec2 {
                    x: (clip_rect.max.x * data.pixels_per_point).round() as u32,
                    y: (clip_rect.max.y * data.pixels_per_point).round() as u32,
                },
            };

            if clip_urect
                .intersect(bevy_math::URect::new(
                    0,
                    0,
                    render_target_size.physical_width as u32,
                    render_target_size.physical_height as u32,
                ))
                .is_empty()
            {
                continue;
            }

            let mesh = match primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh,
                egui::epaint::Primitive::Callback(paint_callback) => {
                    let callback = match paint_callback
                        .callback
                        .clone()
                        .downcast::<EguiBevyPaintCallback>()
                    {
                        Ok(callback) => callback,
                        Err(err) => {
                            log::error!("Unsupported Egui paint callback type: {err:?}");
                            continue;
                        }
                    };

                    data.postponed_updates.push((
                        clip_rect,
                        PaintCallbackDraw {
                            callback: callback.clone(),
                            rect: paint_callback.rect,
                        },
                    ));

                    data.draw_commands.push(DrawCommand {
                        primitive: DrawPrimitive::PaintCallback(PaintCallbackDraw {
                            callback,
                            rect: paint_callback.rect,
                        }),
                        clip_rect,
                    });
                    continue;
                }
            };

            data.vertex_data
                .extend_from_slice(cast_slice::<_, u8>(mesh.vertices.as_slice()));
            data.index_data
                .extend(mesh.indices.iter().map(|i| i + index_offset));
            index_offset += mesh.vertices.len() as u32;

            let texture_handle = match mesh.texture_id {
                egui::TextureId::Managed(id) => EguiTextureId::Managed(*main_entity, id),
                egui::TextureId::User(id) => EguiTextureId::User(id),
            };

            data.draw_commands.push(DrawCommand {
                primitive: DrawPrimitive::Egui(EguiDraw {
                    vertices_count: mesh.indices.len(),
                    egui_texture: texture_handle,
                }),
                clip_rect,
            });
        }

        if data.vertex_data.len() > data.vertex_buffer_capacity {
            data.vertex_buffer_capacity = data.vertex_data.len().next_power_of_two();
            data.vertex_buffer = Some(render_device.create_buffer(&BufferDescriptor {
                label: Some("egui vertex buffer"),
                size: data.vertex_buffer_capacity as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::VERTEX,
                mapped_at_creation: false,
            }));
        }

        let index_data_size = data.index_data.len() * std::mem::size_of::<u32>();
        if index_data_size > data.index_buffer_capacity {
            data.index_buffer_capacity = index_data_size.next_power_of_two();
            data.index_buffer = Some(render_device.create_buffer(&BufferDescriptor {
                label: Some("egui index buffer"),
                size: data.index_buffer_capacity as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::INDEX,
                mapped_at_creation: false,
            }));
        }

        let (vertex_buffer, index_buffer) = match (&data.vertex_buffer, &data.index_buffer) {
            (Some(vertex), Some(index)) => (vertex, index),
            _ => {
                continue;
            }
        };

        render_queue.write_buffer(vertex_buffer, 0, &data.vertex_data);
        render_queue.write_buffer(index_buffer, 0, cast_slice(&data.index_data));
    }
}
