use glam::Vec3;
use world::column::WORLD_HEIGHT;
use world::terrain::{Biome, WorldType};

use super::Game;
use crate::daytime::{DayTime, MC_TICKS_PER_DAY};
use crate::weather::Weather;

const LOCATE_RADIUS: i32 = 6400;
const LOCATE_STEP: i32 = 32;
const TP_LIMIT: f64 = 3.0e7;
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.1..=10.0;

const DAY_TICKS: i64 = 1000;
const NOON_TICKS: i64 = 6000;
const NIGHT_TICKS: i64 = 13000;
const MIDNIGHT_TICKS: i64 = 18000;

const TIME_NAMES: [(&str, i64); 4] = [
    ("day", DAY_TICKS),
    ("noon", NOON_TICKS),
    ("night", NIGHT_TICKS),
    ("midnight", MIDNIGHT_TICKS),
];

const WEATHERS: [(&str, Weather); 3] = [
    ("clear", Weather::Clear),
    ("rain", Weather::Rain),
    ("thunder", Weather::Thunder),
];

struct Spec {
    name: &'static str,
    usage: &'static str,
    about: &'static str,
}

const COMMANDS: [Spec; 6] = [
    Spec {
        name: "speed",
        usage: "/speed <walk|fly> [множитель 0.1–10]",
        about: "Меняет скорость ходьбы или полёта; без числа показывает текущую",
    },
    Spec {
        name: "tp",
        usage: "/tp <x> <y> <z> (~ — относительно игрока)",
        about: "Телепортирует игрока; «~» по высоте при абсолютных x и z ставит на поверхность",
    },
    Spec {
        name: "time",
        usage: "/time set <day|noon|night|midnight|тики> | add <тики> | query daytime",
        about: "Меняет время суток (24000 тиков в сутках)",
    },
    Spec {
        name: "weather",
        usage: "/weather <clear|rain|thunder> [секунды]",
        about: "Включает погоду, по желанию на заданное число секунд",
    },
    Spec {
        name: "locate",
        usage: "/locate biome <биом>",
        about: "Находит ближайший биом",
    },
    Spec {
        name: "help",
        usage: "/help [команда]",
        about: "Показывает список команд или справку по одной",
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub ok: bool,
    pub text: String,
    pub suggest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub start: usize,
    pub options: Vec<String>,
}

type Reply = Result<String, String>;

struct Answer {
    text: String,
    suggest: Option<String>,
}

impl From<String> for Answer {
    fn from(text: String) -> Self {
        Self {
            text,
            suggest: None,
        }
    }
}

enum Coord {
    Absolute(f64),
    Relative(f64),
}

fn parse_coord(spec: &Spec, text: &str) -> Result<Coord, String> {
    let bad = || usage_error(spec, &format!("Некорректная координата «{text}»"));
    let number = |t: &str| {
        t.parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(bad)
    };
    match text.strip_prefix('~') {
        Some("") => Ok(Coord::Relative(0.0)),
        Some(offset) => number(offset).map(Coord::Relative),
        None => number(text).map(Coord::Absolute),
    }
}

fn spec(name: &str) -> Option<&'static Spec> {
    COMMANDS.iter().find(|spec| spec.name == name)
}

fn usage_error(spec: &Spec, problem: &str) -> String {
    format!("{problem}. Синтаксис: {}", spec.usage)
}

fn tokens(line: &str) -> Vec<&str> {
    line.trim_start_matches('/').split_whitespace().collect()
}

fn parse_ticks(spec: &Spec, text: &str) -> Result<i64, String> {
    text.parse::<i64>()
        .map_err(|_| usage_error(spec, &format!("Некорректное число тиков «{text}»")))
}

fn biome_entry(name: &str) -> Option<Biome> {
    Biome::ALL
        .iter()
        .copied()
        .find(|biome| biome.name() == name)
}

fn biome_label(biome: Biome) -> &'static str {
    use Biome::*;
    match biome {
        Ocean => "океан",
        DeepOcean => "глубокий океан",
        FrozenOcean => "замёрзший океан",
        River => "река",
        FrozenRiver => "замёрзшая река",
        Beach => "пляж",
        StoneBeach => "каменистый берег",
        ColdBeach => "холодный пляж",
        Plains => "равнины",
        SunflowerPlains => "подсолнечные равнины",
        Desert => "пустыня",
        DesertHills => "холмы пустыни",
        DesertM => "пустыня M",
        ExtremeHills => "горы",
        ExtremeHillsPlus => "лесистые горы",
        ExtremeHillsM => "горы M",
        ExtremeHillsPlusM => "лесистые горы M",
        ExtremeHillsEdge => "край гор",
        Forest => "лес",
        ForestHills => "лесистые холмы",
        FlowerForest => "цветочный лес",
        BirchForest => "берёзовый лес",
        BirchForestHills => "холмы берёзового леса",
        BirchForestM => "высокий берёзовый лес",
        BirchForestHillsM => "высокие холмы берёзового леса",
        RoofedForest => "тёмный лес",
        RoofedForestM => "тёмный лес M",
        Taiga => "тайга",
        TaigaHills => "холмы тайги",
        TaigaM => "тайга M",
        ColdTaiga => "холодная тайга",
        ColdTaigaHills => "холмы холодной тайги",
        ColdTaigaM => "холодная тайга M",
        MegaTaiga => "мегатайга",
        MegaTaigaHills => "холмы мегатайги",
        MegaSpruceTaiga => "мегаельник",
        MegaSpruceTaigaHills => "холмы мегаельника",
        Swampland => "болото",
        SwamplandM => "болото M",
        IcePlains => "снежные равнины",
        IceSpikes => "ледяные шипы",
        IceMountains => "снежные горы",
        MushroomIsland => "грибной остров",
        MushroomIslandShore => "берег грибного острова",
        Jungle => "джунгли",
        JungleHills => "холмы джунглей",
        JungleEdge => "край джунглей",
        JungleM => "джунгли M",
        JungleEdgeM => "край джунглей M",
        Savanna => "саванна",
        SavannaPlateau => "плато саванны",
        SavannaM => "саванна M",
        SavannaPlateauM => "плато саванны M",
        Mesa => "бесплодные земли",
        MesaPlateauF => "лесистое плато бесплодных земель",
        MesaPlateau => "плато бесплодных земель",
        MesaBryce => "бесплодные земли Брайс",
        MesaPlateauFM => "лесистое плато бесплодных земель M",
        MesaPlateauM => "плато бесплодных земель M",
    }
}

impl Game {
    pub fn run_command(&mut self, line: &str) -> CommandOutput {
        match self.execute(&tokens(line)) {
            Ok(Answer { text, suggest }) => CommandOutput {
                ok: true,
                text,
                suggest,
            },
            Err(text) => CommandOutput {
                ok: false,
                text,
                suggest: None,
            },
        }
    }

    pub fn command_usage(&self, line: &str) -> String {
        let parts = tokens(line);
        match parts.first() {
            None => "/help — список команд".to_owned(),
            Some(name) => match spec(name) {
                Some(spec) => spec.usage.to_owned(),
                None => COMMANDS
                    .iter()
                    .filter(|spec| spec.name.starts_with(name) && parts.len() == 1)
                    .map(|spec| spec.usage)
                    .collect::<Vec<_>>()
                    .join("\n"),
            },
        }
    }

    pub fn complete_command(&self, line: &str) -> Completion {
        let body = line.trim_start_matches('/');
        let offset = line.chars().count() - body.chars().count();
        let parts: Vec<&str> = body.split_whitespace().collect();
        let fresh = body.is_empty() || body.ends_with(char::is_whitespace);
        let (done, current) = if fresh {
            (parts.as_slice(), "")
        } else {
            (&parts[..parts.len() - 1], parts[parts.len() - 1])
        };
        let start = offset + body.chars().count() - current.chars().count();
        let options = candidates(done)
            .into_iter()
            .filter(|option| option.starts_with(&current.to_lowercase()))
            .map(str::to_owned)
            .collect();
        Completion { start, options }
    }

    fn execute(&mut self, parts: &[&str]) -> Result<Answer, String> {
        let Some((name, args)) = parts.split_first() else {
            return Err("Введите команду. /help — список команд".to_owned());
        };
        match *name {
            "time" => self.time_command(args).map(Answer::from),
            "weather" => self.weather_command(args).map(Answer::from),
            "tp" => self.tp_command(args).map(Answer::from),
            "speed" => self.speed_command(args).map(Answer::from),
            "locate" => self.locate_command(args),
            "help" => help(args).map(Answer::from),
            other => Err(format!(
                "Неизвестная команда «{other}». /help — список команд"
            )),
        }
    }

    fn time_command(&mut self, args: &[&str]) -> Reply {
        let spec = spec("time").unwrap();
        match args {
            ["set", value] => {
                let ticks = match TIME_NAMES.iter().find(|(name, _)| name == value) {
                    Some(&(_, ticks)) => ticks,
                    None => {
                        let ticks = parse_ticks(spec, value)?;
                        if ticks < 0 {
                            return Err(usage_error(spec, "Время не может быть отрицательным"));
                        }
                        ticks
                    }
                };
                self.day = DayTime::from_mc_ticks(ticks);
                Ok(format!("Время установлено: {}", self.day.mc_ticks()))
            }
            ["add", value] => {
                let ticks = parse_ticks(spec, value)?;
                self.day = DayTime::from_mc_ticks(i64::from(self.day.mc_ticks()) + ticks);
                Ok(format!("Время установлено: {}", self.day.mc_ticks()))
            }
            ["query", "daytime"] => Ok(format!(
                "Время суток: {} из {MC_TICKS_PER_DAY}",
                self.day.mc_ticks()
            )),
            ["query", "gametime" | "day"] => Err(usage_error(
                spec,
                "Игровое время и номер дня не отслеживаются",
            )),
            _ => Err(usage_error(spec, "Неверные аргументы")),
        }
    }

    fn weather_command(&mut self, args: &[&str]) -> Reply {
        let spec = spec("weather").unwrap();
        let (kind, seconds) = match args {
            [kind] => (kind, None),
            [kind, seconds] => {
                let seconds = seconds
                    .parse::<u32>()
                    .ok()
                    .filter(|&s| s > 0)
                    .ok_or_else(|| {
                        usage_error(spec, &format!("Некорректная длительность «{seconds}»"))
                    })?;
                (kind, Some(seconds))
            }
            _ => return Err(usage_error(spec, "Неверные аргументы")),
        };
        let Some(&(name, weather)) = WEATHERS.iter().find(|(name, _)| name == kind) else {
            return Err(usage_error(spec, &format!("Неизвестная погода «{kind}»")));
        };
        if !self.weather_enabled {
            return Err("Погода выключена в настройках".to_owned());
        }
        match seconds {
            Some(seconds) => self.weather.set_kind_for(weather, seconds),
            None => self.weather.set_kind(weather),
        }
        let label = match weather {
            Weather::Clear => "ясная",
            Weather::Rain => "дождь",
            Weather::Thunder => "гроза",
        };
        Ok(match seconds {
            Some(seconds) => format!("Погода: {label} ({name}) на {seconds} с"),
            None => format!("Погода: {label} ({name})"),
        })
    }

    fn tp_command(&mut self, args: &[&str]) -> Reply {
        let spec = spec("tp").unwrap();
        let [x, y, z] = args else {
            return Err(usage_error(spec, "Неверные аргументы"));
        };
        let (x, y, z) = (
            parse_coord(spec, x)?,
            parse_coord(spec, y)?,
            parse_coord(spec, z)?,
        );
        let feet = self.player.feet;
        let horizontal = |coord: &Coord, from: f32| match *coord {
            Coord::Absolute(v) if v.fract() == 0.0 => v + 0.5,
            Coord::Absolute(v) => v,
            Coord::Relative(d) => f64::from(from) + d,
        };
        let (tx, tz) = (horizontal(&x, feet.x), horizontal(&z, feet.z));
        let ty = match (&x, &y, &z) {
            (Coord::Absolute(_), Coord::Relative(d), Coord::Absolute(_)) if *d == 0.0 => f64::from(
                self.terrain
                    .surface_height(tx.floor() as i32, tz.floor() as i32)
                    + 1,
            ),
            (_, Coord::Absolute(v), _) => *v,
            (_, Coord::Relative(d), _) => f64::from(feet.y) + d,
        };
        let in_range = tx.abs() <= TP_LIMIT
            && tz.abs() <= TP_LIMIT
            && (0.0..f64::from(WORLD_HEIGHT)).contains(&ty);
        if !in_range {
            return Err(usage_error(spec, "Координаты вне допустимых пределов"));
        }
        self.teleport(Vec3::new(tx as f32, ty as f32, tz as f32));
        Ok(format!(
            "Телепортация на [{}, {}, {}]",
            shown(tx),
            shown(ty),
            shown(tz)
        ))
    }

    fn speed_command(&mut self, args: &[&str]) -> Reply {
        let spec = spec("speed").unwrap();
        let (mode, value) = match args {
            [mode] => (mode, None),
            [mode, value] => (mode, Some(value)),
            _ => return Err(usage_error(spec, "Неверные аргументы")),
        };
        let (label, slot) = match *mode {
            "walk" => ("ходьбы", &mut self.player.walk_multiplier),
            "fly" => ("полёта", &mut self.player.fly_multiplier),
            other => return Err(usage_error(spec, &format!("Неизвестный режим «{other}»"))),
        };
        if let Some(value) = value {
            *slot = value
                .parse::<f32>()
                .ok()
                .filter(|v| SPEED_RANGE.contains(v))
                .ok_or_else(|| usage_error(spec, &format!("Множитель «{value}» вне диапазона")))?;
        }
        Ok(format!("Скорость {label}: ×{}", shown(f64::from(*slot))))
    }

    fn teleport(&mut self, feet: Vec3) {
        self.player.feet = feet;
        self.player.velocity = Vec3::ZERO;
        self.player.on_ground = false;
        self.player.sprinting = false;
        self.camera.position = self.player.eye();
        self.sync_prev();
        self.clock.reset();
        self.target = None;
    }

    fn locate_command(&self, args: &[&str]) -> Result<Answer, String> {
        let spec = spec("locate").unwrap();
        let ["biome", name] = args else {
            return Err(usage_error(spec, "Неверные аргументы"));
        };
        let Some(biome) = biome_entry(name) else {
            return Err(usage_error(spec, &format!("Неизвестный биом «{name}»")));
        };
        let label = biome_label(biome);
        let origin = self.player.eye().floor().as_ivec3();
        match self.nearest_biome(origin.x, origin.z, biome) {
            Some((x, z, distance)) => Ok(Answer {
                text: format!(
                    "Ближайший биом «{label}» находится на [{x}, ~, {z}] ({distance} блоков)"
                ),
                suggest: Some(format!("/tp {x} ~ {z}")),
            }),
            None => Err(format!(
                "Биом «{label}» не найден в радиусе {LOCATE_RADIUS} блоков"
            )),
        }
    }

    fn nearest_biome(&self, ox: i32, oz: i32, biome: Biome) -> Option<(i32, i32, i32)> {
        if self.terrain.sample_biome(ox, oz) == biome {
            return Some((ox, oz, 0));
        }
        if self.terrain.world_type() != WorldType::Standard {
            return None;
        }
        let mut best: Option<(i64, i32, i32)> = None;
        for ring in 1..=LOCATE_RADIUS / LOCATE_STEP {
            let reach = i64::from(ring * LOCATE_STEP);
            if best.is_some_and(|(d2, _, _)| d2 <= reach * reach) {
                break;
            }
            for (dx, dz) in ring_cells(ring) {
                let (x, z) = (ox + dx * LOCATE_STEP, oz + dz * LOCATE_STEP);
                if self.terrain.sample_biome(x, z) != biome {
                    continue;
                }
                let d2 = i64::from(x - ox).pow(2) + i64::from(z - oz).pow(2);
                if best.is_none_or(|(b, _, _)| d2 < b) {
                    best = Some((d2, x, z));
                }
            }
        }
        best.map(|(d2, x, z)| (x, z, (d2 as f64).sqrt().round() as i32))
    }
}

fn shown(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{rounded}")
}

fn ring_cells(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    (-ring..=ring).flat_map(move |i| {
        let edge = [(i, -ring), (i, ring)];
        let side = (i.abs() < ring).then_some([(-ring, i), (ring, i)]);
        edge.into_iter().chain(side.into_iter().flatten())
    })
}

fn help(args: &[&str]) -> Reply {
    match args {
        [] => Ok(COMMANDS
            .iter()
            .map(|spec| format!("/{} — {}", spec.name, spec.about))
            .collect::<Vec<_>>()
            .join("\n")),
        [name] => {
            let name = name.trim_start_matches('/');
            spec(name)
                .map(|spec| format!("{}\n{}", spec.about, spec.usage))
                .ok_or_else(|| format!("Неизвестная команда «{name}». /help — список команд"))
        }
        _ => Err(usage_error(spec("help").unwrap(), "Неверные аргументы")),
    }
}

fn candidates(done: &[&str]) -> Vec<&'static str> {
    match done {
        [] | ["help"] => COMMANDS.iter().map(|spec| spec.name).collect(),
        ["time"] => vec!["set", "add", "query"],
        ["time", "set"] => TIME_NAMES.iter().map(|(name, _)| *name).collect(),
        ["time", "query"] => vec!["daytime"],
        ["speed"] => vec!["walk", "fly"],
        ["tp"] | ["tp", _] | ["tp", _, _] => vec!["~"],
        ["weather"] => WEATHERS.iter().map(|(name, _)| *name).collect(),
        ["locate"] => vec!["biome"],
        ["locate", "biome"] => Biome::ALL.iter().map(|biome| biome.name()).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
