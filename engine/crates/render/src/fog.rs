use world::column::WORLD_HEIGHT;
use world::section::SECTION_SIZE;

const FOG_START_FRACTION: f32 = 0.65;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub start: f32,
    pub end: f32,
}

impl Fog {
    pub fn for_radius(radius: i32) -> Self {
        let r = radius as f32;
        let end = SECTION_SIZE as f32 * ((r * r + 1.0).sqrt() - std::f32::consts::SQRT_2);
        Self {
            start: end * FOG_START_FRACTION,
            end,
        }
    }

    pub fn far_plane(&self, camera_y: f32) -> f32 {
        let height = WORLD_HEIGHT as f32;
        let dy = camera_y.max(height - camera_y);
        (self.end * self.end + dy * dy).sqrt() + 1.0
    }
}

#[cfg(test)]
mod tests;
