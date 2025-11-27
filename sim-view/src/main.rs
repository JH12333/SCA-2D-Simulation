// Jiehe Chen
// jiehchen@pdx.edu
// Space Colonization Algorithm 2D Simulation

use eframe::App;
use egui;
use glam::Vec2;
use rand::rng;
use sim_core::{
    attractor::AttractorSet, config::Config, influence_buffer::InfluenceBuffer, phases, tree::Tree,
    types::NodeId,
};

struct Viewer {
    tree: Tree,
    attractors: AttractorSet,
    acc: InfluenceBuffer,
    cfg: Config,

    running: bool,
    zoom: f32,
    pan: egui::Vec2,

    last_new_ids: Vec<NodeId>,
}

impl Viewer {
    fn new() -> Self {
        let tree = Tree::new(Vec2::new(0.0, -220.0), 1.0);

        let mut rng: rand::prelude::ThreadRng = rng();
        let attractors = AttractorSet::random_in_square(10000, 200.0, &mut rng);
        let acc = InfluenceBuffer::with_len(tree.nodes.len());

        let cfg = Config {
            influence_radius: 60.0,
            kill_radius: 30.0,
            step_len: 5.0,
            tropism: Vec2::new(0.0, -0.7),
        };

        Self {
            tree,
            attractors,
            acc,
            cfg,
            running: false,
            zoom: 3.0,
            pan: egui::vec2(0.0, 0.0),
            last_new_ids: Vec::with_capacity(16),
        }
    }

    fn step_once(&mut self) {
        phases::attraction_phase(&self.tree, &mut self.attractors, &self.cfg, &mut self.acc);
        let new_ids = phases::growth_phase(&mut self.tree, &self.acc, &self.cfg);
        phases::kill_phase(&self.tree, &mut self.attractors, &self.cfg);

        self.last_new_ids = new_ids;
    }

    fn world_to_screen(&self, p: Vec2, rect: egui::Rect) -> egui::Pos2 {
        let center = rect.center();
        egui::pos2(
            center.x + p.x * self.zoom + self.pan.x,
            center.y - p.y * self.zoom + self.pan.y,
        )
    }
}

impl App for Viewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if self.running { "⏸ Pause" } else { "▶ Run" })
                    .clicked()
                {
                    self.running = !self.running;
                }

                if ui.button("Step").clicked() {
                    self.step_once();
                }

                ui.add(egui::Slider::new(&mut self.zoom, 0.5..=10.0).text("Zoom"));

                ui.label(format!("Nodes: {}", self.tree.nodes.len()));
                ui.label(format!(
                    "Attractors alive: {}",
                    self.attractors.points.iter().filter(|a| a.alive).count()
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());

            if response.dragged() {
                let delta = response.drag_delta();
                self.pan += delta;
            }

            if ui.ctx().input(|i| i.raw_scroll_delta.y != 0.0) {
                let scroll = ui.ctx().input(|i| i.raw_scroll_delta.y);
                let factor = (1.0 + scroll * 0.001).clamp(0.5, 10.0);
                self.zoom *= factor;
            }

            let painter = ui.painter_at(response.rect);
            let rect = response.rect;

            for (_i, node) in self.tree.nodes.iter().enumerate() {
                for &child in &node.children {
                    let a = self.world_to_screen(node.pos, rect);
                    let b = self.world_to_screen(self.tree.nodes[child].pos, rect);
                    painter
                        .line_segment([a, b], egui::Stroke::new(1.0, egui::Color32::LIGHT_GREEN));
                }
            }

            for (i, node) in self.tree.nodes.iter().enumerate() {
                let p = self.world_to_screen(node.pos, rect);
                let r = (node.radius * self.zoom).max(2.0);

                let color = if self.last_new_ids.contains(&i) {
                    egui::Color32::RED
                } else {
                    egui::Color32::LIGHT_BLUE
                };

                painter.circle_filled(p, r, color);
            }

            for a in &self.attractors.points {
                if !a.alive {
                    continue;
                }
                let p = self.world_to_screen(a.pos, rect);
                painter.circle_filled(p, 2.0, egui::Color32::LIGHT_RED);
            }

            if self.running {
                self.step_once();
                ctx.request_repaint();
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "2D SCA Tree",
        options,
        Box::new(|_cc| Ok(Box::new(Viewer::new()))),
    )
}
