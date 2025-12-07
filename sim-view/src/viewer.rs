//! Interactive 2D space-colonization tree viewer built with eframe/egui.
//!
//! This module defines [`Viewer`], which owns the simulation state
//! (tree, attractors, configuration, etc.) and implements [`eframe::App`]
//! to render and control the simulation through an egui UI.

use eframe::App;
use glam::Vec2;
use rand::rng;
use sim_core::{
    attractor::AttractorSet,
    config::{Config, SpawnTool},
    influence_buffer::InfluenceBuffer,
    phases,
    tree::Tree,
    types::NodeId,
};

/// Main application state for the interactive viewer.
///
/// [`Viewer`] glues together:
/// - The simulation core: [`Tree`], [`AttractorSet`], [`InfluenceBuffer`], [`Config`].
/// - UI configuration (pan/zoom, spawn tool, timing).
/// - eframe/egui callbacks for drawing and user interaction.
///
/// The typical per-frame update is:
/// 1. Handle UI interactions / input.
/// 2. If `running` is `true` and enough time has passed, call [`Viewer::step_once`].
/// 3. Render the tree, attractors, and tool hints.
///
/// ### Fields
/// - `tree` - Current tree structure being grown.
/// - `attractors` - Set of attractor points driving the growth.
/// - `acc` - Per-node influence buffer used between phases.
/// - `cfg` - Global simulation configuration (radii, k-NN, tropism, spawn settings).
///
/// - `rng` - Random number generator used for spawning attractors.
///
/// - `running` - Whether the simulation is currently auto-advancing.
/// - `zoom` - Zoom factor for world-to-screen coordinate mapping.
/// - `pan` - Screen-space pan offset in pixels.
///
/// - `last_new_ids` - Node ids created in the last simulation step (for highlighting).
///
/// - `step_interval` - Target time step between automatic simulation steps (seconds).
/// - `last_step_time` - Time stamp of the last step (egui time).
/// - `last_step_dt` - Actual time delta between the last two steps (for display only).
pub struct Viewer {
    tree: Tree,
    attractors: AttractorSet,
    acc: InfluenceBuffer,
    cfg: Config,

    rng: rand::rngs::ThreadRng,

    running: bool,
    zoom: f32,
    pan: egui::Vec2,

    last_new_ids: Vec<NodeId>,

    step_interval: f64,
    last_step_time: f64,
    last_step_dt: f64,
}

impl Viewer {
    /// Creates a new viewer with a single root node and a random attractor cloud.
    ///
    /// The default setup is:
    /// - A tree with one root at `(0, 0)` and radius `1.0`.
    /// - An oval of attractors centered around `(0, 120)` with radii `(100, 100)`.
    /// - A fresh [`InfluenceBuffer`] sized to the current number of nodes.
    /// - [`Config::default`] for simulation parameters.
    ///
    /// The camera starts with a moderate zoom and no pan.
    ///
    /// ### Returns
    /// A fully-initialized [`Viewer`] ready to be passed to `eframe::run_native`.
    pub fn new() -> Self {
        let mut rng = rng();
        let tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        let attractors = AttractorSet::random_in_oval(
            Vec2::new(0.0, 120.0),
            Vec2::new(100.0, 100.0),
            1000,
            &mut rng,
        );
        let acc = InfluenceBuffer::with_len(tree.nodes.len());
        let cfg = Config::default();

        Self {
            tree,
            attractors,
            acc,
            cfg,
            rng,
            running: false,
            zoom: 3.0,
            pan: egui::vec2(0.0, 0.0),
            last_new_ids: Vec::with_capacity(16),
            step_interval: 0.1,
            last_step_time: 0.0,
            last_step_dt: 0.0,
        }
    }

    /// Resets the simulation to a fresh tree and attractor set.
    ///
    /// This keeps the current configuration (`cfg`) and camera settings,
    /// but:
    /// - Replaces the tree with a single root at `(0, 0)`.
    /// - Generates a new random attractor set in the default oval region.
    /// - Resizes the influence buffer to match the new tree.
    /// - Clears `last_new_ids` and stops auto-running.
    fn reset(&mut self) {
        self.tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        self.attractors = AttractorSet::random_in_oval(
            Vec2::new(0.0, 120.0),
            Vec2::new(100.0, 100.0),
            1000,
            &mut self.rng,
        );
        self.acc = InfluenceBuffer::with_len(self.tree.nodes.len());
        self.last_new_ids.clear();
        self.running = false;
    }

    /// Clears all simulation data.
    ///
    /// After this call:
    /// - The tree has no nodes.
    /// - There are no attractors.
    /// - The influence buffer is empty.
    /// - `last_new_ids` is cleared.
    ///
    /// This is mainly useful as a “blank canvas” for manual spawning.
    fn clear(&mut self) {
        self.tree.nodes.clear();
        self.attractors.points.clear();
        self.acc = InfluenceBuffer::with_len(0);
        self.last_new_ids.clear();
    }

    /// Advances the simulation by a single step.
    ///
    /// The step consists of:
    /// 1. [`phases::attraction_phase`] — accumulate influences into [`InfluenceBuffer`].
    /// 2. [`phases::growth_phase`] — grow new nodes based on the influences.
    /// 3. [`phases::kill_phase`] — mark attractors near the tree as dead.
    ///
    /// The ids of nodes created in this step are stored in `last_new_ids`
    /// so they can be highlighted in the next frame.
    fn step_once(&mut self) {
        phases::attraction_phase(&self.tree, &mut self.attractors, &self.cfg, &mut self.acc);
        let new_ids = phases::growth_phase(&mut self.tree, &self.acc, &self.cfg);
        phases::kill_phase(&self.tree, &mut self.attractors, &self.cfg);

        self.last_new_ids = new_ids;
    }

    /// Converts a world-space position to screen-space.
    ///
    /// World coordinates are scaled by `zoom`, offset by `pan`, and then
    /// centered inside the given `rect`. The y-axis is flipped so that
    /// positive y goes up in world space.
    ///
    /// ### Parameters
    /// - `p` - World-space position.
    /// - `rect` - Screen-space rectangle representing the drawing area.
    ///
    /// ### Returns
    /// The corresponding egui position in screen-space.
    fn world_to_screen(&self, p: Vec2, rect: egui::Rect) -> egui::Pos2 {
        let center = rect.center();
        egui::pos2(
            center.x + p.x * self.zoom + self.pan.x,
            center.y - p.y * self.zoom + self.pan.y,
        )
    }

    /// Converts a screen-space position back to world-space.
    ///
    /// This is the inverse of [`Viewer::world_to_screen`] (up to floating
    /// point rounding), using the same `zoom`, `pan`, and `rect` center.
    ///
    /// ### Parameters
    /// - `p` - Screen-space position in egui coordinates.
    /// - `rect` - Screen-space rectangle representing the drawing area.
    ///
    /// ### Returns
    /// The corresponding position in world-space.
    fn screen_to_world(&self, p: egui::Pos2, rect: egui::Rect) -> Vec2 {
        let center = rect.center();
        let x = (p.x - center.x - self.pan.x) / self.zoom;
        let y = (center.y - p.y + self.pan.y) / self.zoom;
        Vec2::new(x, y)
    }

    /// Helper to draw a labeled `usize` [`egui::DragValue`].
    fn labeled_drag_usize(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut usize,
        range: std::ops::RangeInclusive<usize>,
        speed: f64,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).range(range).speed(speed));
        });
    }

    /// Helper to draw a labeled `f32` [`egui::DragValue`].
    fn labeled_drag_f32(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        speed: f64,
    ) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).range(range).speed(speed));
        });
    }

    /// Builds the top panel UI (run controls, stepping, zoom).
    fn ui_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(if self.running { "⏸ Pause" } else { "▶ Run" })
                    .clicked()
                {
                    self.running = !self.running;
                }

                ui.add(
                    egui::DragValue::new(&mut self.step_interval)
                        .prefix("dt target = ")
                        .range(0.01..=1.0)
                        .speed(0.01),
                );

                if ui.button("Step").clicked() {
                    let now = ctx.input(|i| i.time);
                    if self.last_step_time > 0.0 {
                        self.last_step_dt = now - self.last_step_time;
                    }
                    self.step_once();
                    self.last_step_time = now;
                }

                if ui.button("Reset").clicked() {
                    self.reset();
                }

                if ui.button("Clear").clicked() {
                    self.clear();
                }

                ui.separator();
                ui.add(egui::Slider::new(&mut self.zoom, 0.1..=10.0).text("Zoom"));
            });
        });
    }

    /// Builds the bottom status bar (time step, node count, alive attractors).
    fn ui_status_bar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("dt target = {:.3} s", self.step_interval));
                ui.label(format!("dt last = {:.3} s", self.last_step_dt));
                ui.separator();
                ui.label(format!("nodes = {}", self.tree.nodes.len()));
                ui.label(format!(
                    "alive attractors = {}",
                    self.attractors.points.iter().filter(|a| a.alive).count()
                ));
            });
        });
    }

    /// Builds the right-hand configuration panel for simulation parameters.
    fn ui_config_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("config_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Config");

                ui.separator();
                ui.label("K-nearest settings");
                Self::labeled_drag_usize(
                    ui,
                    "attract_from_kn:",
                    &mut self.cfg.attract_from_kn,
                    0..=10,
                    1.0,
                );
                Self::labeled_drag_usize(
                    ui,
                    "kill_from_kn:",
                    &mut self.cfg.kill_from_kn,
                    0..=10,
                    1.0,
                );

                ui.separator();
                ui.label("Radii");
                Self::labeled_drag_f32(
                    ui,
                    "influence_radius:",
                    &mut self.cfg.influence_radius,
                    0.0..=200.0,
                    0.5,
                );
                Self::labeled_drag_f32(
                    ui,
                    "kill_radius:",
                    &mut self.cfg.kill_radius,
                    0.0..=200.0,
                    0.5,
                );

                ui.separator();
                ui.label("Growth");
                Self::labeled_drag_f32(ui, "step_len:", &mut self.cfg.step_len, 0.0..=20.0, 0.2);

                ui.separator();
                ui.label("Tropism (gravity-like)");
                Self::labeled_drag_f32(ui, "tropism.x:", &mut self.cfg.tropism.x, -2.0..=2.0, 0.05);
                Self::labeled_drag_f32(ui, "tropism.y:", &mut self.cfg.tropism.y, -2.0..=2.0, 0.05);

                ui.separator();
                ui.label("Spawning");
                Self::labeled_drag_usize(
                    ui,
                    "spawn_attractors:",
                    &mut self.cfg.spawn_attractors,
                    1..=1000,
                    1.0,
                );

                ui.label("Rect half extents");
                Self::labeled_drag_f32(
                    ui,
                    "hx:",
                    &mut self.cfg.spawn_rect_half_extents.x,
                    0.0..=1000.0,
                    1.0,
                );
                Self::labeled_drag_f32(
                    ui,
                    "hy:",
                    &mut self.cfg.spawn_rect_half_extents.y,
                    0.0..=1000.0,
                    1.0,
                );

                ui.label("Oval radii");
                Self::labeled_drag_f32(
                    ui,
                    "rx:",
                    &mut self.cfg.spawn_oval_radii.x,
                    0.0..=1000.0,
                    1.0,
                );
                Self::labeled_drag_f32(
                    ui,
                    "ry:",
                    &mut self.cfg.spawn_oval_radii.y,
                    0.0..=1000.0,
                    1.0,
                );

                ui.separator();
                if ui.button("Reset cfg to default").clicked() {
                    self.cfg = Config::default();
                }
            });
    }

    /// Builds the small floating toolbar for choosing the spawn tool.
    fn ui_toolbar(&mut self, ctx: &egui::Context) {
        egui::Area::new("toolbar".into())
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 100.0))
            .movable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 32))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            if ui
                                .selectable_label(
                                    matches!(self.cfg.spawn_tool, SpawnTool::RootNode),
                                    "◎ Root",
                                )
                                .clicked()
                            {
                                self.cfg.spawn_tool = SpawnTool::RootNode;
                            }

                            if ui
                                .selectable_label(
                                    matches!(self.cfg.spawn_tool, SpawnTool::RectAttractors),
                                    "■ Rect",
                                )
                                .clicked()
                            {
                                self.cfg.spawn_tool = SpawnTool::RectAttractors;
                            }

                            if ui
                                .selectable_label(
                                    matches!(self.cfg.spawn_tool, SpawnTool::OvalAttractors),
                                    "○ Oval",
                                )
                                .clicked()
                            {
                                self.cfg.spawn_tool = SpawnTool::OvalAttractors;
                            }
                        });
                    });
            });
    }

    /// Draws a visual hint for the current spawn tool at the hovered world position.
    fn ui_tool_hint(&self, painter: &egui::Painter, rect: egui::Rect, hover_world: Option<Vec2>) {
        let Some(center) = hover_world else {
            return;
        };

        let stroke = egui::Stroke::new(1.5, egui::Color32::YELLOW);

        match self.cfg.spawn_tool {
            SpawnTool::RootNode => {
                let p_screen = self.world_to_screen(center, rect);
                let r = self.cfg.step_len * self.zoom * 0.5;
                painter.circle_filled(p_screen, r, egui::Color32::GREEN);
            }

            SpawnTool::RectAttractors => {
                let half_extents = self.cfg.spawn_rect_half_extents;
                let corners = [
                    Vec2::new(-half_extents.x, -half_extents.y),
                    Vec2::new(half_extents.x, -half_extents.y),
                    Vec2::new(half_extents.x, half_extents.y),
                    Vec2::new(-half_extents.x, half_extents.y),
                ];
                let points: Vec<egui::Pos2> = corners
                    .iter()
                    .map(|&off| self.world_to_screen(center + off, rect))
                    .collect();
                painter.add(egui::Shape::closed_line(points, stroke));
            }

            SpawnTool::OvalAttractors => {
                let radii = self.cfg.spawn_oval_radii;
                let segments = 64;
                let mut pts = Vec::with_capacity(segments);
                use std::f32::consts::TAU;
                for i in 0..segments {
                    let t = (i as f32) / (segments as f32) * TAU;
                    let local = Vec2::new(t.cos() * radii.x, t.sin() * radii.y);
                    pts.push(self.world_to_screen(center + local, rect));
                }
                painter.add(egui::Shape::closed_line(pts, stroke));
            }
        }
    }

    /// Builds the central panel where tree and attractors are drawn and interacted with.
    fn ui_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
            let rect = response.rect;
            let painter = ui.painter_at(rect);

            // Pan with drag.
            if response.dragged() {
                let delta = response.drag_delta();
                self.pan += delta;
            }

            let hover_world = response.hover_pos().map(|p| self.screen_to_world(p, rect));

            // Handle click-based spawning.
            if response.clicked()
                && let Some(center) = hover_world
            {
                match self.cfg.spawn_tool {
                    SpawnTool::RootNode => {
                        let id = self.tree.add_free_node(center, 1.0);
                        self.last_new_ids.clear();
                        self.last_new_ids.push(id);
                    }

                    SpawnTool::RectAttractors => {
                        let new_set = AttractorSet::random_in_rect(
                            center,
                            self.cfg.spawn_rect_half_extents,
                            self.cfg.spawn_attractors,
                            &mut self.rng,
                        );
                        self.attractors.points.extend(new_set.points);
                    }

                    SpawnTool::OvalAttractors => {
                        let new_set = AttractorSet::random_in_oval(
                            center,
                            self.cfg.spawn_oval_radii,
                            self.cfg.spawn_attractors,
                            &mut self.rng,
                        );
                        self.attractors.points.extend(new_set.points);
                    }
                }
            }

            // Zoom around the mouse cursor.
            if ui.ctx().input(|i| i.raw_scroll_delta.y != 0.0) {
                let scroll = ui.ctx().input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    let pointer_screen = response.hover_pos().unwrap_or(rect.center());

                    let world_before = self.screen_to_world(pointer_screen, rect);

                    let factor = (1.0 + scroll * 0.001).clamp(0.5, 2.0);
                    let new_zoom = (self.zoom * factor).clamp(0.1, 10.0);
                    self.zoom = new_zoom;

                    let screen_after = self.world_to_screen(world_before, rect);

                    let delta = pointer_screen - screen_after;
                    self.pan += delta;
                }
            }

            // Draw tree edges.
            for node in self.tree.nodes.iter() {
                for &child in &node.children {
                    let a = self.world_to_screen(node.pos, rect);
                    let b = self.world_to_screen(self.tree.nodes[child].pos, rect);
                    painter
                        .line_segment([a, b], egui::Stroke::new(1.0, egui::Color32::LIGHT_GREEN));
                }
            }

            // Draw tree nodes (highlighting newly added nodes in red).
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

            // Draw alive attractors.
            for a in &self.attractors.points {
                if !a.alive {
                    continue;
                }
                let p = self.world_to_screen(a.pos, rect);
                painter.circle_filled(p, 2.0, egui::Color32::LIGHT_RED);
            }

            // Tool hint overlay.
            self.ui_tool_hint(&painter, rect, hover_world);

            // Auto-run simulation if requested.
            if self.running {
                let now = ctx.input(|i| i.time);
                let elapsed = now - self.last_step_time;
                if elapsed >= self.step_interval {
                    if self.last_step_time > 0.0 {
                        self.last_step_dt = elapsed;
                    }
                    self.step_once();
                    self.last_step_time = now;
                }

                ctx.request_repaint();
            }
        });
    }
}

impl App for Viewer {
    /// eframe callback that builds all UI panels for each frame.
    ///
    /// This method:
    /// - Renders the top control bar and status bar.
    /// - Renders the config side panel and toolbar.
    /// - Draws the central simulation view and handles interactions.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_top_panel(ctx);
        self.ui_status_bar(ctx);
        self.ui_config_panel(ctx);
        self.ui_central_panel(ctx);
        self.ui_toolbar(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui;
    use glam::Vec2;

    fn test_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(800.0, 600.0))
    }

    #[test]
    fn world_to_screen_and_back_is_roundtrip() {
        let mut viewer = Viewer::new();
        // Use non-trivial zoom and pan to exercise the math.
        viewer.zoom = 2.0;
        viewer.pan = egui::vec2(15.0, -7.0);
        let rect = test_rect();

        let world_points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, -5.0),
            Vec2::new(-3.5, 8.25),
        ];

        let eps = 1e-5;

        for p in world_points {
            let screen = viewer.world_to_screen(p, rect);
            let back = viewer.screen_to_world(screen, rect);

            assert!(
                (back.x - p.x).abs() < eps && (back.y - p.y).abs() < eps,
                "roundtrip mismatch: p={:?}, back={:?}",
                p,
                back
            );
        }
    }

    #[test]
    fn reset_restores_basic_state() {
        let mut viewer = Viewer::new();

        // Mutate state to make sure reset actually changes things.
        viewer.tree.add_free_node(Vec2::new(10.0, 0.0), 1.0);
        viewer.attractors.points.clear();
        viewer.acc = InfluenceBuffer::with_len(0);
        viewer.last_new_ids.push(42);
        viewer.running = true;

        viewer.reset();

        // Tree should have exactly one root node again.
        assert_eq!(viewer.tree.nodes.len(), 1);
        assert!(viewer.tree.nodes[0].parent.is_none());

        // Attractors are regenerated; the exact positions don't matter,
        // but the count should match the hard-coded value in reset.
        assert_eq!(viewer.attractors.points.len(), 1000);

        // Influence buffer should be sized to the number of nodes.
        assert_eq!(viewer.acc.count.len(), viewer.tree.nodes.len());

        // No "last new" nodes after reset.
        assert!(viewer.last_new_ids.is_empty());

        // Simulation should not be running after reset.
        assert!(!viewer.running);
    }

    #[test]
    fn clear_removes_all_content() {
        let mut viewer = Viewer::new();

        // Populate the viewer so that `clear` actually removes content.
        assert!(!viewer.tree.nodes.is_empty());
        assert!(!viewer.attractors.points.is_empty());
        assert!(viewer.acc.count.len() > 0);

        viewer.last_new_ids.push(0);

        viewer.clear();

        assert!(viewer.tree.nodes.is_empty());
        assert!(viewer.attractors.points.is_empty());
        assert_eq!(viewer.acc.count.len(), 0);
        assert!(viewer.last_new_ids.is_empty());
    }

    #[test]
    fn step_once_creates_child_and_updates_last_new_ids() {
        let mut viewer = Viewer::new();

        // Override the random setup with a deterministic scenario:
        // - one root at (0, 0)
        // - a single attractor at (10, 0)
        // - influence radius large enough to see the attractor
        viewer.tree = Tree::new(Vec2::new(0.0, 0.0), 1.0);
        viewer.attractors = AttractorSet::from_positions(vec![Vec2::new(10.0, 0.0)]);
        viewer.acc = InfluenceBuffer::with_len(viewer.tree.nodes.len());

        viewer.cfg = Config::default();
        viewer.cfg.influence_radius = 20.0;
        viewer.cfg.kill_radius = 1.0; // small: do not kill the attractor
        viewer.cfg.attract_from_kn = 0;
        viewer.cfg.kill_from_kn = 0;
        viewer.cfg.tropism = Vec2::new(0.0, 0.0);
        viewer.cfg.step_len = 2.0;

        viewer.step_once();

        // Exactly one new node should be created.
        assert_eq!(viewer.last_new_ids.len(), 1);
        let new_id = viewer.last_new_ids[0];
        assert_eq!(new_id, 1);
        assert_eq!(viewer.tree.nodes.len(), 2);

        let new_node = &viewer.tree.nodes[new_id];

        // Direction from (0, 0) to (10, 0) is (1, 0); step_len = 2.0 -> new pos (2, 0).
        assert_eq!(new_node.pos, Vec2::new(2.0, 0.0));

        // Radius should be inherited from the parent.
        assert_eq!(new_node.radius, viewer.tree.nodes[0].radius);

        // The attractor should still be alive (kill radius is too small).
        assert!(viewer.attractors.points[0].alive);
    }
}
