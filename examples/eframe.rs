use eframe::{NativeOptions, Result};
use egui_spine::{
    Animation, AnimationId, Scene, SkeletonKind, Spine, SpineOptions, init_wgpu_spine_context,
};
use glam::vec2;

fn main() -> Result<()> {
    let native_options = NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Spine egui",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

pub struct App {
    spine: Spine,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        init_wgpu_spine_context(render_state, Default::default());

        let options = SpineOptions {
            scene: Scene {
                position: vec2(100., -360.),
                scale: 0.70,
                ..Default::default()
            },
            animation: Animation {
                id: AnimationId::Index(2),
                ..Default::default()
            },
        };
        Self {
            spine: Spine::new(
                "assets/spineboy/export/spineboy.atlas",
                SkeletonKind::Json("assets/spineboy/export/spineboy-ess.json"),
                options,
            )
            .unwrap(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Default::default())
            .show(ctx, |ui| {
                let _ = ui.add(&mut self.spine);
            });
    }
}
