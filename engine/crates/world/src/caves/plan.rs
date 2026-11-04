use std::f32::consts::PI;

use glam::DVec3;

use crate::stream::Stream;
use crate::trees::hash;

pub const RANGE: i32 = 8;
const MAX_STEPS: i32 = RANGE * 16 - 16;
const SALT_CAVES: u64 = 140;
const SALT_RAVINES: u64 = 150;
const RAVINE_PROFILE_LEN: usize = 256;

#[derive(Clone, Copy)]
pub struct Segment {
    pub center: DVec3,
    pub width: f64,
    pub height: f64,
    /// Номер комнаты (`addRoom`), 0 — обычный туннель: комната копает один шаг и останавливается.
    pub room: u32,
}

impl Segment {
    pub fn reach(&self) -> f64 {
        16.0 + self.width * 2.0
    }
}

pub struct Ravine {
    pub profile: Box<[f32; RAVINE_PROFILE_LEN]>,
    pub segments: Vec<Segment>,
}

#[derive(Default)]
pub struct Plan {
    pub caves: Vec<Segment>,
    pub ravines: Vec<Ravine>,
    pub bounds: Option<Bounds>,
}

#[derive(Clone, Copy)]
pub struct Bounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
}

struct Tunnel {
    start: DVec3,
    scale: f32,
    yaw: f32,
    pitch: f32,
    from_step: i32,
    to_step: i32,
    height_scale: f64,
}

impl Plan {
    pub fn new(seed: i32, source: (i32, i32)) -> Self {
        let mut plan = Plan::default();
        plan.add_caves(seed, source);
        plan.add_ravines(seed, source);
        plan.bounds = plan.compute_bounds();
        plan
    }

    fn compute_bounds(&self) -> Option<Bounds> {
        self.caves.iter().fold(None, |bounds, s| {
            let reach = s.reach();
            let found = Bounds {
                min_x: s.center.x - reach,
                max_x: s.center.x + reach,
                min_z: s.center.z - reach,
                max_z: s.center.z + reach,
            };
            Some(bounds.map_or(found, |b| Bounds {
                min_x: b.min_x.min(found.min_x),
                max_x: b.max_x.max(found.max_x),
                min_z: b.min_z.min(found.min_z),
                max_z: b.max_z.max(found.max_z),
            }))
        })
    }

    /// `MapGenCaves.recursiveGenerate`.
    fn add_caves(&mut self, seed: i32, (sx, sz): (i32, i32)) {
        let mut rng = source_stream(seed, sx, sz, SALT_CAVES);
        let mut count = {
            let a = rng.below(15) + 1;
            let b = rng.below(a) + 1;
            rng.below(b)
        };
        if rng.below(7) != 0 {
            count = 0;
        }
        let mut rooms = 0;
        for _ in 0..count {
            let center = DVec3::new(
                f64::from(sx * 16 + rng.below(16) as i32),
                {
                    let span = rng.below(120) + 8;
                    f64::from(rng.below(span))
                },
                f64::from(sz * 16 + rng.below(16) as i32),
            );
            let mut tunnels = 1;
            if rng.below(4) == 0 {
                let mut room_rng = rng.fork();
                rooms += 1;
                let scale = 1.0 + rng.float() * 6.0;
                self.cave_tunnel(
                    &mut room_rng,
                    Tunnel {
                        start: center,
                        scale,
                        yaw: 0.0,
                        pitch: 0.0,
                        from_step: -1,
                        to_step: -1,
                        height_scale: 0.5,
                    },
                    rooms,
                );
                tunnels += rng.below(4);
            }
            for _ in 0..tunnels {
                let yaw = rng.float() * 2.0 * PI;
                let pitch = (rng.float() - 0.5) * 2.0 / 8.0;
                let mut scale = rng.float() * 2.0 + rng.float();
                if rng.below(10) == 0 {
                    scale *= rng.float() * rng.float() * 3.0 + 1.0;
                }
                let mut tunnel_rng = rng.fork();
                self.cave_tunnel(
                    &mut tunnel_rng,
                    Tunnel {
                        start: center,
                        scale,
                        yaw,
                        pitch,
                        from_step: 0,
                        to_step: 0,
                        height_scale: 1.0,
                    },
                    0,
                );
            }
        }
    }

    /// `MapGenCaves.addTunnel` без отсечения по целевой колонке: у отсечения нет побочных эффектов.
    fn cave_tunnel(&mut self, rng: &mut Stream, tunnel: Tunnel, room: u32) {
        let Tunnel {
            start,
            scale,
            mut yaw,
            mut pitch,
            from_step,
            mut to_step,
            height_scale,
        } = tunnel;
        let mut center = start;
        let (mut yaw_drift, mut pitch_drift) = (0.0f32, 0.0f32);
        if to_step <= 0 {
            to_step = MAX_STEPS - rng.below(MAX_STEPS as u32 / 4) as i32;
        }
        let mut step = from_step;
        let is_room = step == -1;
        if is_room {
            step = to_step / 2;
        }
        let branch_at = (rng.below(to_step as u32 / 2) + to_step as u32 / 4) as i32;
        let steep = rng.below(6) == 0;

        while step < to_step {
            let width = 1.5 + f64::from((step as f32 * PI / to_step as f32).sin() * scale);
            let (sin_pitch, cos_pitch) = pitch.sin_cos();
            center.x += f64::from(yaw.cos() * cos_pitch);
            center.y += f64::from(sin_pitch);
            center.z += f64::from(yaw.sin() * cos_pitch);
            pitch *= if steep { 0.92 } else { 0.7 };
            pitch += pitch_drift * 0.1;
            yaw += yaw_drift * 0.1;
            pitch_drift *= 0.9;
            yaw_drift *= 0.75;
            pitch_drift += (rng.float() - rng.float()) * rng.float() * 2.0;
            yaw_drift += (rng.float() - rng.float()) * rng.float() * 4.0;

            if !is_room && step == branch_at && scale > 1.0 {
                for side in [-1.0, 1.0] {
                    let mut branch_rng = rng.fork();
                    let branch_scale = rng.float() * 0.5 + 0.5;
                    self.cave_tunnel(
                        &mut branch_rng,
                        Tunnel {
                            start: center,
                            scale: branch_scale,
                            yaw: yaw + side * PI / 2.0,
                            pitch: pitch / 3.0,
                            from_step: step,
                            to_step,
                            height_scale: 1.0,
                        },
                        0,
                    );
                }
                return;
            }

            if is_room || rng.below(4) != 0 {
                self.caves.push(Segment {
                    center,
                    width,
                    height: width * height_scale,
                    room,
                });
            }
            step += 1;
        }
    }

    /// `MapGenRavine.recursiveGenerate`.
    fn add_ravines(&mut self, seed: i32, (sx, sz): (i32, i32)) {
        let mut rng = source_stream(seed, sx, sz, SALT_RAVINES);
        if rng.below(50) != 0 {
            return;
        }
        let center = DVec3::new(
            f64::from(sx * 16 + rng.below(16) as i32),
            {
                let span = rng.below(40) + 8;
                f64::from(rng.below(span) + 20)
            },
            f64::from(sz * 16 + rng.below(16) as i32),
        );
        let yaw = rng.float() * 2.0 * PI;
        let pitch = (rng.float() - 0.5) * 2.0 / 8.0;
        let scale = (rng.float() * 2.0 + rng.float()) * 2.0;
        let mut tunnel_rng = rng.fork();
        let ravine = ravine_tunnel(
            &mut tunnel_rng,
            Tunnel {
                start: center,
                scale,
                yaw,
                pitch,
                from_step: 0,
                to_step: 0,
                height_scale: 3.0,
            },
        );
        self.ravines.push(ravine);
    }
}

/// `MapGenRavine.addTunnel`.
fn ravine_tunnel(rng: &mut Stream, tunnel: Tunnel) -> Ravine {
    let Tunnel {
        start,
        scale,
        mut yaw,
        mut pitch,
        from_step,
        mut to_step,
        height_scale,
    } = tunnel;
    let mut center = start;
    let (mut yaw_drift, mut pitch_drift) = (0.0f32, 0.0f32);
    if to_step <= 0 {
        to_step = MAX_STEPS - rng.below(MAX_STEPS as u32 / 4) as i32;
    }
    let mut profile = Box::new([0.0f32; RAVINE_PROFILE_LEN]);
    let mut wobble = 1.0f32;
    for (i, slot) in profile.iter_mut().enumerate() {
        if i == 0 || rng.below(3) == 0 {
            wobble = 1.0 + rng.float() * rng.float();
        }
        *slot = wobble * wobble;
    }
    let mut segments = Vec::new();
    for step in from_step..to_step {
        let mut width = 1.5 + f64::from((step as f32 * PI / to_step as f32).sin() * scale);
        let mut height = width * height_scale;
        width *= f64::from(rng.float()) * 0.25 + 0.75;
        height *= f64::from(rng.float()) * 0.25 + 0.75;
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        center.x += f64::from(yaw.cos() * cos_pitch);
        center.y += f64::from(sin_pitch);
        center.z += f64::from(yaw.sin() * cos_pitch);
        pitch *= 0.7;
        pitch += pitch_drift * 0.05;
        yaw += yaw_drift * 0.05;
        pitch_drift *= 0.8;
        yaw_drift *= 0.5;
        pitch_drift += (rng.float() - rng.float()) * rng.float() * 2.0;
        yaw_drift += (rng.float() - rng.float()) * rng.float() * 4.0;
        if rng.below(4) != 0 {
            segments.push(Segment {
                center,
                width,
                height,
                room: 0,
            });
        }
    }
    Ravine { profile, segments }
}

fn source_stream(seed: i32, sx: i32, sz: i32, salt: u64) -> Stream {
    Stream::new(u64::from(hash(seed, sx, sz, salt)) | u64::from(hash(seed, sx, sz, salt + 1)) << 32)
}
