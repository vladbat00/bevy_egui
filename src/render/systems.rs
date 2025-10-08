use crate::{
    EguiContextSettings, EguiManagedTextures, EguiRenderOutput, EguiUserTextures,
    RenderComputedScaleFactor,
    render::{
        DrawCommand, DrawPrimitive, EguiBevyPaintCallback, EguiCameraView, EguiDraw, EguiPipeline,
        EguiPipelineKey, EguiViewTarget, PaintCallbackDraw,
    },
};
use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_image::Image;
use bevy_log as log;
use bevy_math::{URect, UVec2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedCamera,
    extract_resource::ExtractResource,
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupEntry, BindingResource, Buffer, BufferDescriptor, BufferId,
        CachedRenderPipelineId, DynamicUniformBuffer, PipelineCache, SpecializedRenderPipelines,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity},
    texture::GpuImage,
    view::ExtractedView,
};
use bytemuck::cast_slice;
use itertools::Itertools;
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
                    .map(|(handle, (_, id))| (EguiTextureId::User(*id), *handle)),
            )
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
    /// Is affected by render target size, scale factor and [`EguiContextSettings::scale_factor`].
    pub scale: Vec2,
    /// Normally equals `Vec2::new(-1.0, 1.0)`.
    pub translation: Vec2,
}

impl EguiTransform {
    /// Calculates the transform from target size and target scale factor multiplied by [`EguiContextSettings::scale_factor`].
    pub fn new(target_size: Vec2, scale_factor: f32) -> Self {
        EguiTransform {
            scale: Vec2::new(
                2.0 / (target_size.x / scale_factor),
                -2.0 / (target_size.y / scale_factor),
            ),
            translation: Vec2::new(-1.0, 1.0),
        }
    }
}

/// Prepares Egui transforms.
pub fn prepare_egui_transforms_system(
    mut egui_transforms: ResMut<EguiTransforms>,
    views: Query<&RenderComputedScaleFactor>,
    render_targets: Query<(&ExtractedView, &ExtractedCamera, &EguiCameraView)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    egui_pipeline: Res<EguiPipeline>,
) -> Result {
    egui_transforms.buffer.clear();
    egui_transforms.offsets.clear();

    for (view, camera, egui_camera_view) in render_targets.iter() {
        let Some(target_size) = camera.physical_target_size else {
            continue;
        };

        let &RenderComputedScaleFactor { scale_factor } = views.get(egui_camera_view.0)?;
        let offset = egui_transforms
            .buffer
            .push(&EguiTransform::new(target_size.as_vec2(), scale_factor));
        egui_transforms
            .offsets
            .insert(view.retained_view_entity.main_entity, offset);
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

    Ok(())
}

/// Maps Egui textures to bind groups.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct EguiTextureBindGroups(pub HashMap<EguiTextureId, (BindGroup, Option<u32>)>);

/// Queues bind groups.
pub fn queue_bind_groups_system(
    mut commands: Commands,
    egui_textures: ExtractedEguiTextures,
    render_device: Res<RenderDevice>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    egui_pipeline: Res<EguiPipeline>,
) {
    let egui_texture_iterator = egui_textures.handles().filter_map(|(texture, handle_id)| {
        let gpu_image = gpu_images.get(handle_id)?;
        Some((texture, gpu_image))
    });

    let bind_groups = if let Some(bindless) = egui_pipeline.bindless {
        let bindless = u32::from(bindless) as usize;
        let mut bind_groups = HashMap::new();

        let mut texture_array = Vec::new();
        let mut sampler_array = Vec::new();
        let mut egui_texture_ids = Vec::new();

        for textures in egui_texture_iterator.chunks(bindless).into_iter() {
            texture_array.clear();
            sampler_array.clear();
            egui_texture_ids.clear();

            for (egui_texture_id, gpu_image) in textures {
                egui_texture_ids.push(egui_texture_id);
                // Dereference needed to convert from bevy to wgpu type
                texture_array.push(&*gpu_image.texture_view);
                sampler_array.push(&*gpu_image.sampler);
            }

            let bind_group = render_device.create_bind_group(
                None,
                &egui_pipeline.texture_bind_group_layout,
                &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureViewArray(texture_array.as_slice()),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::SamplerArray(sampler_array.as_slice()),
                    },
                ],
            );

            // Simply assign bind group to egui texture
            // Additional code is not needed because bevy RenderPass set_bind_group
            // removes redundant switching between bind groups
            for (offset, egui_texture_id) in egui_texture_ids.drain(..).enumerate() {
                bind_groups.insert(egui_texture_id, (bind_group.clone(), Some(offset as u32)));
            }
        }
        bind_groups
    } else {
        egui_texture_iterator
            .map(|(texture, gpu_image)| {
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
                (texture, (bind_group, None::<u32>))
            })
            .collect()
    };
    commands.insert_resource(EguiTextureBindGroups(bind_groups))
}

/// Cached Pipeline IDs for the specialized instances of `EguiPipeline`.
#[derive(Resource)]
pub struct EguiPipelines(pub HashMap<MainEntity, CachedRenderPipelineId>);

/// Queue [`EguiPipeline`] instances.
pub fn queue_pipelines_system(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<EguiPipeline>>,
    egui_pipeline: Res<EguiPipeline>,
    egui_views: Query<&EguiViewTarget, With<ExtractedView>>,
    camera_views: Query<(&MainEntity, &ExtractedCamera)>,
) {
    let pipelines: HashMap<MainEntity, CachedRenderPipelineId> = egui_views
        .iter()
        .filter_map(|egui_camera_view| {
            let (main_entity, extracted_camera) = camera_views.get(egui_camera_view.0).ok()?;

            let pipeline_id = specialized_pipelines.specialize(
                &pipeline_cache,
                &egui_pipeline,
                EguiPipelineKey {
                    hdr: extracted_camera.hdr,
                },
            );
            Some((*main_entity, pipeline_id))
        })
        .collect();

    commands.insert_resource(EguiPipelines(pipelines));
}

/// Cached Pipeline IDs for the specialized instances of `EguiPipeline`.
#[derive(Default, Resource)]
pub struct EguiRenderData(pub(crate) HashMap<MainEntity, EguiRenderTargetData>);

pub(crate) struct EguiRenderTargetData {
    keep: bool,
    pub(crate) render_entity: RenderEntity,
    pub(crate) vertex_data: Vec<u8>,
    pub(crate) vertex_buffer_capacity: usize,
    pub(crate) vertex_buffer: Option<Buffer>,
    pub(crate) index_data: Vec<u32>,
    pub(crate) index_buffer_capacity: usize,
    pub(crate) index_buffer: Option<Buffer>,
    pub(crate) draw_commands: Vec<DrawCommand>,
    pub(crate) postponed_updates: Vec<(egui::Rect, PaintCallbackDraw)>,
    pub(crate) pixels_per_point: f32,
    pub(crate) target_size: UVec2,
    pub(crate) key: Option<EguiPipelineKey>,
}

impl Default for EguiRenderTargetData {
    fn default() -> Self {
        Self {
            keep: false,
            render_entity: RenderEntity::from(Entity::PLACEHOLDER),
            vertex_data: Vec::new(),
            vertex_buffer_capacity: 0,
            vertex_buffer: None,
            index_data: Vec::new(),
            index_buffer_capacity: 0,
            index_buffer: None,
            draw_commands: Vec::new(),
            postponed_updates: Vec::new(),
            pixels_per_point: 0.0,
            target_size: UVec2::ZERO,
            key: None,
        }
    }
}

/// Prepares Egui transforms.
pub fn prepare_egui_render_target_data_system(
    mut render_data: ResMut<EguiRenderData>,
    render_targets: Query<(
        Entity,
        &ExtractedView,
        &RenderComputedScaleFactor,
        &EguiViewTarget,
        &EguiRenderOutput,
    )>,
    extracted_cameras: Query<&ExtractedCamera>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let render_data = &mut render_data.0;
    render_data.retain(|_, data| {
        let keep = data.keep;
        data.keep = false;
        keep
    });

    for (render_entity, view, computed_scale_factor, egui_view_target, render_output) in
        render_targets.iter()
    {
        let data = render_data
            .entry(view.retained_view_entity.main_entity)
            .or_default();

        data.keep = true;
        data.render_entity = render_entity.into();

        // Construct a pipeline key based on a render target.
        let Ok(extracted_camera) = extracted_cameras.get(egui_view_target.0) else {
            // This is ok when a window is minimized.
            log::trace!("ExtractedCamera entity doesn't exist for the Egui view");
            continue;
        };
        data.key = Some(EguiPipelineKey {
            hdr: extracted_camera.hdr,
        });

        data.pixels_per_point = computed_scale_factor.scale_factor;
        if extracted_camera
            .physical_viewport_size
            .is_none_or(|size| size.x < 1 || size.y < 1)
        {
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

            let clip_urect = URect {
                min: UVec2 {
                    x: (clip_rect.min.x * data.pixels_per_point).round() as u32,
                    y: (clip_rect.min.y * data.pixels_per_point).round() as u32,
                },
                max: UVec2 {
                    x: (clip_rect.max.x * data.pixels_per_point).round() as u32,
                    y: (clip_rect.max.y * data.pixels_per_point).round() as u32,
                },
            };

            if clip_urect
                .intersect(URect::new(
                    view.viewport.x,
                    view.viewport.y,
                    view.viewport.x + view.viewport.z,
                    view.viewport.y + view.viewport.w,
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
                egui::TextureId::Managed(id) => {
                    EguiTextureId::Managed(view.retained_view_entity.main_entity, id)
                }
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
