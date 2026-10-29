use super::*;

const OPTIONS: MeshOptions = MeshOptions {
    ambient_occlusion: true,
};
const KEY: ColumnPos = ColumnPos(glam::IVec2::new(3, -2));

fn jobs() -> MeshJobs {
    let mut jobs = MeshJobs::default();
    jobs.set_slots(2);
    jobs
}

#[test]
fn fresh_result_is_accepted() {
    let mut jobs = jobs();
    let id = jobs.begin(Target::Column(KEY), OPTIONS);
    assert!(jobs.column_in_flight(KEY));
    assert!(jobs.finish(id).is_some());
    assert!(!jobs.column_in_flight(KEY));
    assert!(jobs.is_empty());
}

#[test]
fn result_for_unloaded_column_is_dropped() {
    let mut jobs = jobs();
    let column = jobs.begin(Target::Column(KEY), OPTIONS);
    let section = jobs.begin(Target::Section(KEY.section(2)), OPTIONS);
    jobs.unload(KEY);
    assert!(jobs.finish(column).is_none());
    assert!(jobs.finish(section).is_none());
}

#[test]
fn reload_does_not_revive_old_job() {
    let mut jobs = jobs();
    let old = jobs.begin(Target::Column(KEY), OPTIONS);
    jobs.unload(KEY);
    let new = jobs.begin(Target::Column(KEY), OPTIONS);
    assert!(jobs.finish(old).is_none());
    assert!(jobs.finish(new).is_some());
}

#[test]
fn edit_makes_section_job_stale() {
    let mut jobs = jobs();
    let section = KEY.section(1);
    let id = jobs.begin(Target::Section(section), OPTIONS);
    jobs.section_changed(section);
    assert!(jobs.finish(id).is_none());
}

#[test]
fn edit_in_another_section_keeps_section_job() {
    let mut jobs = jobs();
    let id = jobs.begin(Target::Section(KEY.section(1)), OPTIONS);
    jobs.section_changed(KEY.section(4));
    assert!(jobs.finish(id).is_some());
}

#[test]
fn edit_during_column_job_drops_it_and_asks_for_rebuild() {
    let mut jobs = jobs();
    let id = jobs.begin(Target::Column(KEY), OPTIONS);
    assert!(jobs.column_changed(KEY));
    assert!(!jobs.column_in_flight(KEY));
    assert!(jobs.finish(id).is_none());
    assert!(!jobs.column_changed(KEY));
}

#[test]
fn newer_column_job_supersedes_older_jobs() {
    let mut jobs = jobs();
    let section = jobs.begin(Target::Section(KEY.section(0)), OPTIONS);
    let old = jobs.begin(Target::Column(KEY), OPTIONS);
    let new = jobs.begin(Target::Column(KEY), OPTIONS);
    assert!(jobs.finish(section).is_none());
    assert!(jobs.finish(old).is_none());
    assert!(jobs.column_in_flight(KEY));
    assert!(jobs.finish(new).is_some());
}

#[test]
fn newer_section_job_supersedes_older() {
    let mut jobs = jobs();
    let section = KEY.section(5);
    let old = jobs.begin(Target::Section(section), OPTIONS);
    let new = jobs.begin(Target::Section(section), OPTIONS);
    assert!(jobs.finish(old).is_none());
    assert!(jobs.finish(new).is_some());
}

#[test]
fn slots_count_unfinished_jobs() {
    let mut jobs = jobs();
    assert_eq!(jobs.free_slots(), 2);
    let a = jobs.begin(Target::Column(KEY), OPTIONS);
    jobs.begin(Target::Section(KEY.section(0)), OPTIONS);
    assert_eq!(jobs.free_slots(), 0);
    jobs.finish(a);
    assert_eq!(jobs.free_slots(), 1);
}

#[test]
fn clear_returns_tickets_and_ignores_late_results() {
    let mut jobs = jobs();
    let id = jobs.begin(Target::Column(KEY), OPTIONS);
    assert_eq!(jobs.clear().len(), 1);
    assert!(jobs.finish(id).is_none());
    assert!(!jobs.column_in_flight(KEY));
}
