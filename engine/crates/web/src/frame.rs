use game::Game;
use game::camera::NEAR;
use glam::{Vec2, Vec3};
use render::fog::Fog;
use render::{Effects, FrameParams};
use world::block::FluidKind;
use world::terrain::WATER_LEVEL;

const UNDERWATER_FOG: Fog = Fog {
    start: 0.0,
    end: 24.0,
};
const UNDERWATER_COLOR: [f32; 3] = [0.02, 0.07, 0.2];
const LAVA_FOG: Fog = Fog {
    start: 0.0,
    end: 4.0,
};
const LAVA_COLOR: [f32; 3] = [0.784, 0.314, 0.102];

pub(crate) fn frame_params(
    game: &Game,
    radius: i32,
    aspect: f32,
    clouds: bool,
    effects: Effects,
) -> FrameParams {
    let rain = game.rain();
    let camera = game.camera();
    let fog = Fog::for_radius(radius);
    let far = fog.far_plane(camera.position.y);
    let view_proj = camera.view_proj(aspect, far, game.fov_scale());
    let eye_fluid = game.eye_fluid();
    let sky = game.sky();
    let (fog, background) = match eye_fluid {
        Some(FluidKind::Water) => (
            UNDERWATER_FOG,
            UNDERWATER_COLOR.map(|c| c * sky.underwater_scale),
        ),
        Some(FluidKind::Lava) => (LAVA_FOG, LAVA_COLOR),
        None => (fog.in_rain(rain), sky.color),
    };
    FrameParams {
        view_proj,
        camera_pos: camera.position,
        fog_start: fog.start,
        fog_end: fog.end,
        background: Vec3::from(background),
        eye_fluid,
        darkness: sky.darkness,
        anim_time: game.anim_time(),
        cloud_drift: game.cloud_drift(),
        sun_dir: Vec3::from(game.sun_direction()),
        night: sky.night,
        zenith: Vec3::from(sky.zenith),
        warm: sky.warm,
        horizon_tilt: (camera.position.y - (WATER_LEVEL + 1) as f32).max(0.0) / fog.end.max(1.0),
        star_rotation: Vec2::from(game.star_rotation()),
        clouds,
        effects,
        rain,
        rain_drops: game.rain_drops(),
        flash: game.flash(),
        snow: f32::from(u8::from(game.snowing())),
        near: NEAR,
        far,
    }
}
