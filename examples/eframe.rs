use eframe::{NativeOptions, Result};
use egui_spine::{SkeletonKind, Spine, WgpuContexOptions, init_wgpu_spine_context};

fn main() -> Result<()> {
    let native_options = NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "My egui App",
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
        init_wgpu_spine_context(render_state, WgpuContexOptions {});

        Self {
            spine: Spine::new(
                "assets/spineboy/export/spineboy.atlas",
                SkeletonKind::Json("assets/spineboy/export/spineboy-ess.json"),
                Default::default(),
            )
            .unwrap(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Default::default())
            .show(ctx, |ui| ui.add(&mut self.spine));
    }
}
