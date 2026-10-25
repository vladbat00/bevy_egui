//! Regression test: colors of bevy textures aren't premultiplied by their alpha, unlike
//! egui-managed textures, so fully transparent pixels must not be visible whatever their
//! color is.

use bevy::{
    app::ScheduleRunnerPlugin,
    asset::RenderAssetUsages,
    camera::RenderTarget,
    image::ImageSampler,
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use bevy_egui::{
    EguiContexts, EguiGlobalSettings, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle,
    PrimaryEguiContext, egui,
};

const SIZE: u32 = 64;
const MAX_FRAMES: u32 = 300;

/// A 2x1 texture: a fully transparent white texel and an opaque red one.
#[derive(Resource)]
struct UserTexture(Handle<Image>);

fn setup_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;

    let mut user_texture = Image::new(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 0, 255, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    user_texture.sampler = ImageSampler::nearest();
    commands.insert_resource(UserTexture(images.add(user_texture)));

    let mut target = Image::new_fill(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let target = images.add(target);

    commands.spawn((
        PrimaryEguiContext,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
    ));
    commands
        .spawn(Readback::texture(target))
        .observe(check_pixels_system);
}

fn draw_texture_system(mut contexts: EguiContexts, user_texture: Res<UserTexture>) -> Result {
    let texture_id = contexts.add_image(EguiTextureHandle::Weak(user_texture.0.id()));
    let ctx = contexts.ctx_mut()?;
    ctx.layer_painter(egui::LayerId::background()).image(
        texture_id,
        ctx.content_rect(),
        egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.)),
        egui::Color32::WHITE,
    );
    Ok(())
}

fn check_pixels_system(
    readback: On<ReadbackComplete>,
    mut app_exit_writer: MessageWriter<AppExit>,
) {
    let pixel = |x: u32| {
        let index = ((SIZE / 2 * SIZE + x) * 4) as usize;
        &readback.data[index..index + 4]
    };
    let transparent_half = pixel(SIZE / 4);
    let opaque_half = pixel(SIZE * 3 / 4);

    // The texture isn't drawn yet.
    if opaque_half != [255, 0, 0, 255] {
        return;
    }
    assert_eq!(
        transparent_half,
        [0, 0, 0, 255],
        "the color of the transparent texel is visible",
    );
    app_exit_writer.write(AppExit::Success);
}

fn timeout_system(mut frame_count: Local<u32>) {
    *frame_count += 1;
    if *frame_count > MAX_FRAMES {
        panic!("test timeout: the user texture was never drawn");
    }
}

fn main() {
    // Bindless mode uses a different code path to switch between textures.
    for bindless_mode_array_size in [std::num::NonZero::new(16), None] {
        run(bindless_mode_array_size);
    }
}

// `EguiPlugin` has more fields with some features enabled.
#[allow(clippy::needless_update)]
fn run(bindless_mode_array_size: Option<std::num::NonZero<u32>>) {
    let exit = App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<WinitPlugin>(),
            ScheduleRunnerPlugin::default(),
            EguiPlugin {
                bindless_mode_array_size,
                ..default()
            },
        ))
        .add_systems(Startup, setup_system)
        .add_systems(Update, timeout_system)
        .add_systems(EguiPrimaryContextPass, draw_texture_system)
        .run();
    assert_eq!(exit, AppExit::Success);
}
