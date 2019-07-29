use fs_extra;
use jwalk::*;
use std::path::PathBuf;

fn test_dir() -> (PathBuf, tempfile::TempDir) {
    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir");
    let temp_dir = tempfile::tempdir().unwrap();
    let options = fs_extra::dir::CopyOptions::new();
    fs_extra::dir::copy(&template, &temp_dir, &options).unwrap();
    let mut test_dir = temp_dir.path().to_path_buf();
    test_dir.push(template.file_name().unwrap());
    (test_dir, temp_dir)
}

fn local_paths(walk_dir: WalkDir) -> Vec<String> {
    let root = walk_dir.root().to_owned();
    walk_dir
        .into_iter()
        .map(|each_result| {
            let each_entry = each_result.unwrap();
            let path = each_entry.path().to_path_buf();
            let path = path.strip_prefix(&root).unwrap().to_path_buf();
            let mut path_string = path.to_str().unwrap().to_string();
            path_string.push_str(&format!(" ({})", each_entry.depth));
            path_string
        })
        .collect()
}

fn local_sizes(walk_dir: WalkDir) -> Vec<String> {
    let root = walk_dir.root().to_owned();
    walk_dir
        .into_iter()
        .map(|each_result| {
            let each_entry = each_result.unwrap();
            let path = each_entry.path().to_path_buf();
            let path = path.strip_prefix(&root).unwrap().to_path_buf();
            let mut path_string = path.to_str().unwrap().to_string();
            path_string.push_str(&format!(" ({:?})", each_entry.metadata.unwrap().unwrap().len()));
            path_string
        })
        .collect()
}

#[test]
fn walk() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir));
    assert!(paths.contains(&"b.txt (1)".to_string()));
    assert!(paths.contains(&"group 1 (1)".to_string()));
    assert!(paths.contains(&"group 1/d.txt (2)".to_string()));
    assert!(paths.contains(&"group 2/e.txt (2)".to_string()));
}

#[test]
fn sort_by_name_single_thread() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).num_threads(1).sort(true));
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
            "link_to_a (1)",
            "link_to_group_1 (1)",
            "link_to_group_1/d.txt (2)",
        ]
    );
}

#[test]
fn sort_by_name_rayon_pool_global() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).sort(true));
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
            "link_to_a (1)",
            "link_to_group_1 (1)",
            "link_to_group_1/d.txt (2)",
        ]
    );
}

#[test]
fn sort_by_name_rayon_pool_2_threads() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).num_threads(2).sort(true));
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
            "link_to_a (1)",
            "link_to_group_1 (1)",
            "link_to_group_1/d.txt (2)",
        ]
    );
}

#[test]
fn see_hidden_files() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).skip_hidden(false).sort(true));
    assert!(paths.contains(&"group 2/.hidden_file.txt (2)".to_string()));
}

#[test]
fn max_depth_0() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).max_depth(0).sort(true));
    assert_eq!(paths, vec![" (0)",]);
}

#[test]
fn max_depth_1() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).max_depth(1).sort(true));
    assert_eq!(
        paths,
        vec![
            " (0)",
            "a.txt (1)",
            "b.txt (1)",
            "c.txt (1)",
            "group 1 (1)",
            "group 2 (1)",
            "link_to_a (1)",
            "link_to_group_1 (1)"
        ]
    );
}

#[test]
fn walk_file() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(test_dir.join("a.txt"));
    let mut iter = walk_dir.into_iter();
    assert!(iter.next().unwrap().unwrap().file_name.to_str().unwrap() == "a.txt");
    assert!(iter.next().is_none());
}

#[test]
fn error_when_path_does_not_exist() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(test_dir.join("path_does_not_exist"));
    let mut iter = walk_dir.into_iter();
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn error_when_path_removed_during_iteration() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(&test_dir).num_threads(1).sort(true);
    let mut iter = walk_dir.into_iter();

    // Read root. read_dir for root is also called since single thread mode.
    iter.next().unwrap().is_ok(); // " (0)",

    // Remove group 2 dir from disk
    fs_extra::remove_items(&vec![test_dir.join("group 2")]).unwrap();

    iter.next().unwrap().is_ok(); // "a.txt (1)",
    iter.next().unwrap().is_ok(); // "b.txt (1)",
    iter.next().unwrap().is_ok(); // "c.txt (1)",
    iter.next().unwrap().is_ok(); // "group 1 (1)",
    iter.next().unwrap().is_ok(); // "group 1/d.txt (2)",

    // group 2 is read correctly, since it was read before path removed.
    let group_2 = iter.next().unwrap().unwrap();

    // group 2 content error IS set, since path is removed when try read_dir for
    // group 2 path.
    group_2.content_error.is_some();

    iter.next().unwrap().is_ok(); // "link_to_a (1)",
    iter.next().unwrap().is_ok(); // "link_to_group_1 (2)"
    iter.next().unwrap().is_ok(); // "link_to_group_1/d.txt (2)",

    // done!
    assert!(iter.next().is_none());
}

#[test]
fn walk_root() {
    let paths: Vec<_> = WalkDir::new("/")
        .max_depth(1)
        .sort(true)
        .into_iter()
        .filter_map(|each| Some(each.ok()?.path()))
        .collect();
    assert_eq!(paths.first().unwrap().to_str().unwrap(), "/");
}

#[test]
fn sizes_metadata() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_sizes(WalkDir::new(test_dir).sort(true).preload_metadata(true));
    assert_eq!(
        paths,
        vec![
            " (4096)",
            "a.txt (2)",
            "b.txt (9)",
            "c.txt (2)",
            "group 1 (4096)",
            "group 1/d.txt (7)",
            "group 2 (4096)",
            "group 2/e.txt (7)",
            "link_to_a (2)",
            "link_to_group_1 (4096)",
            "link_to_group_1/d.txt (7)",
        ]
    );
}
