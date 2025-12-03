use serial_test::serial;
use std::env;

pub mod util;

use jwalk::*;
use util::{parallelism_options, test_dir};

fn local_paths(walk_dir: WalkDir) -> Vec<String> {
    let root = walk_dir.root().to_owned();
    walk_dir
        .into_iter()
        .map(|each_result| {
            let each_entry = each_result.unwrap();
            if let Some(err) = each_entry.read_children.as_ref().and_then(|c| c.error()) {
                panic!("should not encounter any child errors :{:?}", err);
            }
            let path = each_entry.path();
            let path = path.strip_prefix(&root).unwrap().to_path_buf();
            let mut path_string = path.to_str().unwrap().to_string();
            path_string.push_str(&format!(" ({})", each_entry.depth));
            path_string
        })
        .collect()
}

#[test]
#[serial]
fn walk_relative_1() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();

        env::set_current_dir(test_dir).unwrap();

        let paths = local_paths(
            WalkDir::new(".")
                .sort(true)
                .parallelism(parallelism.clone()),
        );

        assert_eq!(
            paths,
            vec![
                " (0)",
                "a.txt (1)",
                "b.txt (1)",
                "c.txt (1)",
                "group 1 (1)",
                "group 1/d.txt (2)",
                "group 2 (1)",
                "group 2/e.txt (2)",
            ]
        );

        let root_dir_entry = WalkDir::new("..")
            .parallelism(parallelism)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(&root_dir_entry.file_name, "..");
    }
}

#[test]
#[serial]
fn walk_relative_2() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();

        env::set_current_dir(test_dir.join("group 1")).unwrap();

        let paths = local_paths(
            WalkDir::new("..")
                .sort(true)
                .parallelism(parallelism.clone()),
        );

        assert_eq!(
            paths,
            vec![
                " (0)",
                "a.txt (1)",
                "b.txt (1)",
                "c.txt (1)",
                "group 1 (1)",
                "group 1/d.txt (2)",
                "group 2 (1)",
                "group 2/e.txt (2)",
            ]
        );

        let root_dir_entry = WalkDir::new(".")
            .parallelism(parallelism)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(&root_dir_entry.file_name, ".");
    }
}
