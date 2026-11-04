use std::fs;
use std::path::PathBuf;

use super::super::template::Template;
use super::super::*;
use super::dump::{dump, dump_below};
use super::snapshots::{basement, desert_temple, hut, igloo_top};

fn template_files() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/structure/templates");
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(path).unwrap())
        })
        .collect();
    files.sort();
    files
}

fn cells(template: &Template) -> Vec<String> {
    let size = template.size();
    let mut out = Vec::new();
    for y in 0..size.y {
        for z in 0..size.z {
            for x in 0..size.x {
                out.push(format!("{:?}", template.entry(x, y, z)));
            }
        }
    }
    out
}

#[test]
fn every_template_in_the_catalog_parses() {
    let files = template_files();
    assert!(files.len() >= 2);
    for (name, text) in files {
        Template::parse(&name, &text);
    }
}

#[test]
fn a_dump_of_a_template_part_is_the_template_itself() {
    let part = igloo_top(Facing::South);
    let again = Template::parse("dump", &dump(&part));
    assert_eq!(cells(&again), cells(&super::super::igloo::TOP));
    let part = hut(Facing::South);
    let again = Template::parse("dump", &dump(&part));
    assert_eq!(cells(&again), cells(&super::super::swamp_hut::TEMPLATE));
}

#[test]
fn rotated_parts_dump_the_same_in_local_axes() {
    for facing in Facing::ALL {
        assert_eq!(dump(&hut(facing)), dump(&hut(Facing::South)), "{facing:?}");
        assert_eq!(
            dump(&igloo_top(facing)),
            dump(&igloo_top(Facing::South)),
            "{facing:?}"
        );
        assert_eq!(
            dump(&basement(5, facing)),
            dump(&basement(5, Facing::South)),
            "{facing:?}"
        );
        assert_eq!(
            dump_below(&desert_temple(facing), 14),
            dump_below(&desert_temple(Facing::South), 14),
            "{facing:?}"
        );
    }
}

#[test]
fn template_options_set_state() {
    let text =
        "size 2 1 1\nlegend\nB bed front=-z color=14 data=4\nR random 3:stone 1:dirt\ny 0\nBR\n";
    let template = Template::parse("opts.txt", text);
    let [bed, random] = [template.entry(0, 0, 0), template.entry(1, 0, 0)];
    assert!(format!("{bed:?}").contains("data: 116"), "{bed:?}");
    assert!(format!("{random:?}").starts_with("Random"), "{random:?}");
}

#[test]
#[should_panic(expected = "bad.txt:5: unknown symbol `Q`")]
fn an_unknown_symbol_names_file_and_line() {
    Template::parse("bad.txt", "size 2 1 1\nlegend\nA stone\ny 0\nAQ\n");
}

#[test]
#[should_panic(expected = "bad.txt:3: unknown block `nope`")]
fn an_unknown_block_names_file_and_line() {
    Template::parse("bad.txt", "size 1 1 1\nlegend\nA nope\ny 0\nA\n");
}

#[test]
#[should_panic(expected = "bad.txt:5: row must be 2 symbols wide")]
fn an_uneven_row_is_rejected() {
    Template::parse("bad.txt", "size 2 1 1\nlegend\nA stone\ny 0\nAAA\n");
}

#[test]
#[should_panic(expected = "template ends before every layer is complete")]
fn a_missing_layer_is_rejected() {
    Template::parse("bad.txt", "size 1 2 1\nlegend\nA stone\ny 0\nA\n");
}
