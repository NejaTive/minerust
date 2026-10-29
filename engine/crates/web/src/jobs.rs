//! Учёт заданий мешера в воркерах: что в полёте и какие результаты ещё годятся.
//! Без GPU и браузера, чтобы отсев устаревшего проверялся тестами на хосте.
use std::collections::HashMap;

use mesh::mesher::MeshOptions;
use world::column::COLUMN_SECTIONS;
use world::pos::{ColumnPos, SectionPos};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Column(ColumnPos),
    Section(SectionPos),
}

/// Задание в полёте.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ticket {
    pub target: Target,
    /// Опции, с которыми построен меш: после смены AO результат всё равно ставится
    /// (дыра хуже старого AO), но колонку нужно перестроить.
    pub options: MeshOptions,
    column_version: u32,
    section_version: u32,
}

/// Версии нужны, потому что результат возвращается через пару кадров: за это время
/// колонку могли выгрузить, правка игрока — обогнать задание, а перестройка колонки —
/// устареть. Задание годится, только если версии колонки и секции в момент `finish`
/// те же, что при `begin`. Версии берутся из одного счётчика (общего с номерами заданий),
/// поэтому после выгрузки и повторной загрузки колонки старые версии не совпадут.
#[derive(Default)]
pub(crate) struct MeshJobs {
    counter: u32,
    slots: usize,
    column_version: HashMap<ColumnPos, u32>,
    section_version: HashMap<SectionPos, u32>,
    tickets: HashMap<u32, Ticket>,
    /// Номер новейшего задания на колонку целиком, пока оно в полёте.
    column_jobs: HashMap<ColumnPos, u32>,
}

impl MeshJobs {
    fn fresh(&mut self) -> u32 {
        self.counter = self.counter.wrapping_add(1);
        self.counter
    }

    /// Сколько заданий можно держать в полёте; 0 — воркеров нет.
    pub(crate) fn set_slots(&mut self, slots: usize) {
        self.slots = slots;
    }

    pub(crate) fn enabled(&self) -> bool {
        self.slots > 0
    }

    pub(crate) fn free_slots(&self) -> usize {
        self.slots.saturating_sub(self.tickets.len())
    }

    /// Заданий в полёте, считая не принятые результаты.
    pub(crate) fn len(&self) -> usize {
        self.tickets.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tickets.is_empty()
    }

    /// Идёт ли задание на колонку целиком. Задания на её секции в это время не
    /// отдаются: их данные новее, а результат колонки перезаписал бы их.
    pub(crate) fn column_in_flight(&self, key: ColumnPos) -> bool {
        self.column_jobs.contains_key(&key)
    }

    /// Регистрирует задание и возвращает его номер. Задание на колонку снимает все
    /// старые задания на неё, в том числе на секции: колонка строится по свежим данным.
    pub(crate) fn begin(&mut self, target: Target, options: MeshOptions) -> u32 {
        let id = self.fresh();
        let ticket = match target {
            Target::Column(key) => {
                self.column_version.insert(key, id);
                self.column_jobs.insert(key, id);
                Ticket {
                    target,
                    options,
                    column_version: id,
                    section_version: 0,
                }
            }
            Target::Section(key) => {
                let column_version = *self.column_version.entry(key.column()).or_insert(id);
                self.section_version.insert(key, id);
                Ticket {
                    target,
                    options,
                    column_version,
                    section_version: id,
                }
            }
        };
        self.tickets.insert(id, ticket);
        id
    }

    /// Отмена задания, которое не удалось закодировать (колонка уже выгружена).
    pub(crate) fn cancel(&mut self, id: u32) {
        if let Some(ticket) = self.tickets.remove(&id)
            && let Target::Column(key) = ticket.target
            && self.column_jobs.get(&key) == Some(&id)
        {
            self.column_jobs.remove(&key);
        }
    }

    /// Закрывает задание `id`; `Some`, если результат годится и его надо поставить.
    pub(crate) fn finish(&mut self, id: u32) -> Option<Ticket> {
        let ticket = self.tickets.remove(&id)?;
        match ticket.target {
            Target::Column(key) => {
                if self.column_jobs.get(&key) == Some(&id) {
                    self.column_jobs.remove(&key);
                }
                (self.column_version.get(&key) == Some(&ticket.column_version)).then_some(ticket)
            }
            Target::Section(key) => (self.column_version.get(&key.column())
                == Some(&ticket.column_version)
                && self.section_version.get(&key) == Some(&ticket.section_version))
            .then_some(ticket),
        }
    }

    /// Блоки секции изменились (правка игрока): задание на неё устарело.
    pub(crate) fn section_changed(&mut self, key: SectionPos) {
        if self.section_version.contains_key(&key) {
            let version = self.fresh();
            self.section_version.insert(key, version);
        }
    }

    /// Данные колонки изменились: задание на колонку целиком в полёте устарело.
    /// `true`, если такое было — тогда колонку нужно поставить на меш заново.
    pub(crate) fn column_changed(&mut self, key: ColumnPos) -> bool {
        if self.column_jobs.remove(&key).is_none() {
            return false;
        }
        let version = self.fresh();
        self.column_version.insert(key, version);
        true
    }

    /// Колонку выгрузили: все её задания в полёте устаревают.
    pub(crate) fn unload(&mut self, key: ColumnPos) {
        self.column_version.remove(&key);
        self.column_jobs.remove(&key);
        for sy in 0..COLUMN_SECTIONS {
            self.section_version.remove(&key.section(sy));
        }
    }

    /// Воркеров больше нет: забывает всё в полёте и отдаёт задания, чтобы их построили
    /// заново. Опоздавшие результаты отсеются в `finish`: номеров уже нет в таблице.
    pub(crate) fn clear(&mut self) -> Vec<Ticket> {
        self.column_jobs.clear();
        self.tickets.drain().map(|(_, ticket)| ticket).collect()
    }
}

#[cfg(test)]
mod tests;
