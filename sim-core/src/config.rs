use glam::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub influence_radius: f32,
    pub kill_radius: f32,
    pub step_len: f32,
    pub tropism: Vec2,
}
