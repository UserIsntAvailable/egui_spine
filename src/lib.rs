use egui::{Response, Ui, Widget};
use glam::{Mat4, Vec2, vec3};
use renderer::{Meshes, RendererCallback};
use rusty_spine::{
    AnimationStateData, Atlas, Physics, SkeletonBinary, SkeletonData, SkeletonJson, SpineError,
    controller::{SkeletonController, SkeletonControllerSettings},
    draw::{ColorSpace, CullDirection},
};
use std::{borrow::Cow, path::Path, sync::Arc};

mod renderer;

pub use renderer::Face;
pub use renderer::wgpu::{WgpuContextOptions, init_wgpu_spine_context};

// TODO(Unavailable): Feature gate non strictly necessary dependencies.

#[derive(Debug)]
pub struct Spine {
    options: SpineOptions,
    controller: Arc<SkeletonController>,
}

impl Spine {
    pub fn new<A, S>(
        atlas: A,
        skel: SkeletonKind<S>,
        options: SpineOptions,
    ) -> Result<Self, SpineError>
    where
        A: AsRef<Path>,
        S: AsRef<Path>,
    {
        Self::__new(atlas.as_ref(), skel.as_ref(), options)
    }

    fn __new(
        atlas: &Path,
        skel: SkeletonKind<&Path>,
        options: SpineOptions,
    ) -> Result<Self, SpineError> {
        let atlas = Arc::new(Atlas::new_from_file(atlas)?);
        let premultiplied_alpha = atlas.pages().any(|page| page.pma());
        let skel = Arc::new(skel.read(atlas)?);

        // TODO(Unavailable): Set any crossfades.
        let animation_state = Arc::new(AnimationStateData::new(skel.clone()));
        let controller = SkeletonController::new(skel.clone(), animation_state);
        let settings = SkeletonControllerSettings {
            color_space: ColorSpace::SRGB,
            cull_direction: CullDirection::CounterClockwise,
            premultiplied_alpha,
        };
        let mut controller = controller.with_settings(settings);

        // TODO(Unavailable): Allow users to inspect animation events.

        // TODO(Unavailable): Configuration
        let should_loop = true;
        let animation_state = &mut controller.animation_state;
        match &options.animation.id {
            AnimationId::Index(index) => {
                match controller.skeleton.data().animations().nth(*index) {
                    Some(animation) => animation_state.set_animation(0, &animation, should_loop),
                    None => {
                        return Err(SpineError::NotFound {
                            what: "Animation".to_owned(),
                            name: index.to_string(),
                        });
                    }
                }
            }
            AnimationId::Name(name) => {
                animation_state.set_animation_by_name(0, &name, should_loop)?
            }
        };

        // TODO(Unvailable): `Skin` handling

        Ok(Self {
            options,
            controller: Arc::new(controller),
        })
    }
}

impl Spine {
    pub fn options(&self) -> &SpineOptions {
        &self.options
    }

    // TODO(Unavailable): Iterator that returns all the available animations.

    // TODO(Unavailable): Individual `set_animation_*` methods.

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.options.scene
    }
}

impl Widget for &mut Spine {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();

        let Some(controller) = Arc::get_mut(&mut self.controller) else {
            panic!("Tried to render the same Spine model multiple times in the same render pass");
        };

        let dt = ui.input(|i| i.stable_dt).max(0.001);
        controller.update(dt, Physics::Update);

        let renderables = controller.combined_renderables();
        let controller = Arc::clone(&self.controller);
        let meshes = Meshes::new(controller, renderables);

        let rect = ui.available_rect_before_wrap();
        let scene_view = self.options.scene.create_scene_view(rect.size());
        let cull_mode = self.options.animation.cull_mode;

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {
                meshes,
                scene_view,
                cull_mode,
            },
        ));

        ui.response()
    }
}

pub enum SkeletonKind<P>
where
    P: AsRef<Path>,
{
    Json(P),
    Binary(P),
}

impl<P> SkeletonKind<P>
where
    P: AsRef<Path>,
{
    #[inline]
    fn as_ref(&self) -> SkeletonKind<&Path> {
        match self {
            Self::Json(path) => SkeletonKind::Json(path.as_ref()),
            Self::Binary(path) => SkeletonKind::Binary(path.as_ref()),
        }
    }

    #[inline]
    fn read(self, atlas: Arc<Atlas>) -> Result<SkeletonData, SpineError> {
        match self {
            SkeletonKind::Json(path) => SkeletonJson::new(atlas).read_skeleton_data_file(path),
            SkeletonKind::Binary(path) => SkeletonBinary::new(atlas).read_skeleton_data_file(path),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpineOptions {
    pub scene: Scene,
    pub animation: Animation,
    // TODO(Unavailable): event_cb: Box<dyn Fn()>
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub position: Vec2,
    pub angle: f32,
    pub scale: f32,
    pub reflect: Reflect,
}

impl Scene {
    pub(crate) fn create_scene_view(&self, size: egui::Vec2) -> Mat4 {
        let position = self.position.extend(0.);
        let scale = vec3(self.scale, self.scale, 1.);

        let world = Mat4::from_translation(position)
            * Mat4::from_rotation_z(self.angle)
            * Mat4::from_scale(scale);

        let (mut xl, mut xr) = (size.x * -0.5, size.x * 0.5);
        let (mut yl, mut yr) = (size.y * -0.5, size.y * 0.5);

        if self.reflect.contains(Reflect::XAxis) {
            std::mem::swap(&mut yl, &mut yr);
        }
        if self.reflect.contains(Reflect::YAxis) {
            std::mem::swap(&mut xl, &mut xr);
        }

        let proj = Mat4::orthographic_rh(xl, xr, yl, yr, 0., 1.);

        proj * world
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            angle: 0.0,
            scale: 1.0,
            reflect: Reflect::empty(),
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct Reflect: u8 {
        const XAxis = 0b01;
        const YAxis = 0b10;
    }
}

#[derive(Clone, Debug)]
pub struct Animation {
    // TODO(Unavailable): Option<>, to allow not showing anything, until a
    // user changes it.
    pub id: AnimationId,
    pub cull_mode: Option<Face>,
    // TODO(Unavailable): Extra fields:
    // ```
    // playback_speed: f32,
    // loop: bool,
    // skin: Option<Skin>,
    // ```
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            id: AnimationId::Index(0),
            cull_mode: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AnimationId {
    Index(usize),
    Name(Cow<'static, str>),
}

impl Default for AnimationId {
    fn default() -> Self {
        Self::Index(0)
    }
}
