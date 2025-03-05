use egui::{Response, Ui, Widget};
use glam::{Mat4, Vec2, vec3};
use renderer::{Meshes, RendererCallback};
use rusty_spine::{
    AnimationStateData, Atlas, Physics, SkeletonBinary, SkeletonData, SkeletonJson, SpineError,
    controller::{SkeletonController, SkeletonControllerSettings},
    draw::{ColorSpace, CullDirection},
};
use std::{path::Path, sync::Arc};

mod renderer;
pub use renderer::wgpu::{WgpuContexOptions, init_wgpu_spine_context};

#[derive(Debug)]
pub struct Spine {
    scene: Scene,
    controller: SkeletonController,
}

impl Spine {
    pub fn new<A, S>(atlas: A, skel: SkeletonKind<S>, scene: Scene) -> Result<Self, SpineError>
    where
        A: AsRef<Path>,
        S: AsRef<Path>,
    {
        Self::__new(atlas.as_ref(), skel.as_ref(), scene)
    }

    fn __new(atlas: &Path, skel: SkeletonKind<&Path>, scene: Scene) -> Result<Self, SpineError> {
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
        match &scene.animation {
            Animation::Index(index) => match controller.skeleton.data().animations().nth(*index) {
                Some(animation) => animation_state.set_animation(0, &animation, should_loop),
                None => {
                    return Err(SpineError::NotFound {
                        what: "Animation".to_owned(),
                        name: index.to_string(),
                    });
                }
            },
            Animation::Name(name) => {
                animation_state.set_animation_by_name(0, &name, should_loop)?
            }
        };

        // TODO(Unvailable): `Skin` handling

        Ok(Self { scene, controller })
    }
}

impl Spine {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    // TODO(Unavailable): Iterator that returns all the available animations.

    // TODO(Unavailable): Individual `set_scene_*` methods.

    // NOTE: We can't just give a `&mut Scene` to the user, since we need to
    // update the `controller` state depending on what the user changes in the
    // scene.
    pub fn update_scene(&mut self, _new_scene: Scene) {
        todo!()
    }
}

impl Widget for &mut Spine {
    // FIXME(Unavailable): I need to find a way to prevent people for calling
    // `ui.add(&mut spine)` twice in a single render pass.
    fn ui(self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();

        let dt = ui.input(|i| i.stable_dt).max(0.001);
        self.controller.update(dt, Physics::Update);

        let meshes = Meshes(self.controller.combined_renderables());
        let premultiplied_alpha = self.controller.settings.premultiplied_alpha;
        let rect = ui.available_rect_before_wrap();

        let scene_view = self.scene.create_scene_view(rect.size());
        // TODO(Unavailable): Pass down a `Face`

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {
                meshes,
                scene_view,
                premultiplied_alpha,
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

/// Configuration options on how the spine animation would look.
// FIXME(Unavailable): Constructor
// TODO(Unavailable): This struct should be split into (SpineOptions):
//
// scene: Scene {
//   pos: Vec2,
//   angle: f32,
//   scale: f32,
//   flipped: Flipped,
// },
//
// animation: Animation {
//   id: AnimationId,
//   playback_speed: f32,
//   loop: bool,
//   cull_mode: Option<Face>,
//   skin: Option<Skin>,
// }
//
// event_cb: Box<dyn Fn()>
#[derive(Clone, Debug)]
pub struct Scene {
    pos: Vec2,
    angle: f32,
    scale: f32,
    animation: Animation,
    // TODO(Unavailable): Horizontal and Vertical Flips
}

impl Scene {
    pub fn create_scene_view(&self, size: egui::Vec2) -> Mat4 {
        let pos = self.pos.extend(0.);
        let scale = vec3(self.scale, self.scale, 1.);

        let world = Mat4::from_translation(pos)
            * Mat4::from_rotation_z(self.angle)
            * Mat4::from_scale(scale);

        #[rustfmt::skip]
        let proj = Mat4::orthographic_rh(
            size.x * -0.5, size.x * 0.5,
            size.y * -0.5, size.y * 0.5,
            0.           , 1.,
        );

        proj * world
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            angle: 0.0,
            scale: 1.0,
            animation: Default::default(),
        }
    }
}

// TODO(Unavailable): Rename to `AnimationId`
#[derive(Clone, Debug)]
pub enum Animation {
    Index(usize),
    // TODO(Unavailable): Cow<'static, str>
    Name(String),
}

impl Default for Animation {
    fn default() -> Self {
        Self::Index(0)
    }
}
