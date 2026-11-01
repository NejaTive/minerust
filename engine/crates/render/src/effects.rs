use crate::shadow::ShadowQuality;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effects {
    pub colored_light: bool,
    pub water_reflections: bool,
    pub ground_fog: bool,
    pub plant_sway: bool,
    pub hdr: bool,
    pub bloom: bool,
    pub water_depth: bool,
    pub sun_rays: bool,
    pub shadows: bool,
    pub cloud_shadows: bool,
    pub ssr: bool,
    pub shadow_quality: ShadowQuality,
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
            sun_rays: true,
            shadows: true,
            cloud_shadows: true,
            ssr: true,
            shadow_quality: ShadowQuality::default(),
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
            | u32::from(self.sun_rays && self.hdr) << 7
            | u32::from(self.shadows) << 8
            | u32::from(self.cloud_shadows) << 9
            | u32::from(self.ssr && self.hdr && self.water_reflections) << 10
    }
}

#[cfg(test)]
mod tests;
