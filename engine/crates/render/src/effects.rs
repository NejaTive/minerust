#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effects {
    pub colored_light: bool,
    pub water_reflections: bool,
    pub ground_fog: bool,
    pub plant_sway: bool,
    pub hdr: bool,
    pub bloom: bool,
    pub water_depth: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            colored_light: true,
            water_reflections: true,
            ground_fog: true,
            plant_sway: true,
            hdr: true,
            bloom: true,
            water_depth: true,
        }
    }
}

impl Effects {
    /// Биты читает `shader.wgsl` (константы `FX_*`); менять вместе с ним.
    pub fn bits(self) -> u32 {
        u32::from(self.colored_light)
            | u32::from(self.water_reflections) << 1
            | u32::from(self.ground_fog) << 2
            | u32::from(self.plant_sway) << 3
            | u32::from(self.hdr) << 4
            | u32::from(self.bloom && self.hdr) << 5
            | u32::from(self.water_depth && self.hdr) << 6
    }
}
