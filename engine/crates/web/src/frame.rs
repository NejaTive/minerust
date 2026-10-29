use game::Game;
use glam::{Vec2, Vec3};
use render::FrameParams;
use render::fog::Fog;
use world::terrain::WATER_LEVEL;

const UNDERWATER_FOG: Fog = Fog {
    start: 0.0,
    end: 24.0,
};
const UNDERWATER_COLOR: [f32; 3] = [0.02, 0.07, 0.2];

pub(crate) fn frame_params(game: &Game, radius: i32, aspect: f32) -> FrameParams {
    let camera = game.camera();
    let fog = Fog::for_radius(radius);
    let view_proj = camera.view_proj(aspect, fog.far_plane(camera.position.y), game.fov_scale());
    let underwater = game.underwater();
    let sky = game.sky();
    let (fog, background) = if underwater {
        (
            UNDERWATER_FOG,
            UNDERWATER_COLOR.map(|c| c * sky.underwater_scale),
        )
    } else {
        (fog, sky.color)
    };
    FrameParams {
        view_proj,
        camera_pos: camera.position,
        fog_start: fog.start,
        fog_end: fog.end,
        background: Vec3::from(background),
        underwater,
        darkness: sky.darkness,
        sun_dir: Vec3::from(game.sun_direction()),
        night: sky.night,
        zenith: Vec3::from(sky.zenith),
        warm: sky.warm,
        horizon_tilt: (camera.position.y - (WATER_LEVEL + 1) as f32).max(0.0) / fog.end.max(1.0),
        star_rotation: Vec2::from(game.star_rotation()),
    }
}
