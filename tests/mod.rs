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

#[test]
fn walk_serial() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(
        WalkDir::new(test_dir)
            .parallelism(Parallelism::Serial)
            .sort(true),
    );
    assert!(
        paths
            == vec![
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
}

#[test]
fn sort_by_name_rayon_custom_2_threads() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(
        WalkDir::new(test_dir)
            .parallelism(Parallelism::RayonNewPool(2))
            .sort(true),
    );
    assert!(
        paths
            == vec![
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
}

#[test]
fn walk_rayon_global() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).sort(true));
    assert!(
        paths
            == vec![
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
    assert!(paths == vec![" (0)",]);
}

#[test]
fn max_depth_1() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(WalkDir::new(test_dir).max_depth(1).sort(true));
    assert!(
        paths
            == vec![
                " (0)",
                "a.txt (1)",
                "b.txt (1)",
                "c.txt (1)",
                "group 1 (1)",
                "group 2 (1)",
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
fn walk_file_serial() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(test_dir.join("a.txt")).parallelism(Parallelism::Serial);
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
fn error_when_path_removed_durring_iteration() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(&test_dir)
        .parallelism(Parallelism::Serial)
        .sort(true);
    let mut iter = walk_dir.into_iter();

    // Read root. read_dir for root is also called since single thread mode.
    let _ = iter.next().unwrap().is_ok(); // " (0)",

    // Remove group 2 dir from disk
    fs_extra::remove_items(&vec![test_dir.join("group 2")]).unwrap();

    let _ = iter.next().unwrap().is_ok(); // "a.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "b.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "c.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "group 1 (1)",
    let _ = iter.next().unwrap().is_ok(); // "group 1/d.txt (2)",

    // group 2 is read correctly, since it was read before path removed.
    let group_2 = iter.next().unwrap().unwrap();

    // group 2 content error IS set, since path is removed when try read_dir for
    // group 2 path.
    let _ = group_2.read_children_error.is_some();

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
    assert!(paths.first().unwrap().to_str().unwrap() == "/");
}

#[test]
fn filter_groups_with_process_entries() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(
        WalkDir::new(test_dir)
            .sort(true)
            // Filter groups out manually
            .process_entries(|_parent, children| {
                children.retain(|each_result| {
                    each_result
                        .as_ref()
                        .map(|dir_entry| {
                            !dir_entry.file_name.to_string_lossy().starts_with("group")
                        })
                        .unwrap_or(true)
                });
            }),
    );
    assert!(paths == vec![" (0)", "a.txt (1)", "b.txt (1)", "c.txt (1)",]);
}

#[test]
fn filter_group_children_with_process_entries() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(
        WalkDir::new(test_dir)
            .sort(true)
            // Filter group children
            .process_entries(|_parent, children| {
                children.iter_mut().for_each(|each_result| {
                    if let Ok(each) = each_result {
                        if each.file_name.to_string_lossy().starts_with("group") {
                            each.read_children_path = None;
                        }
                    }
                });
            }),
    );
    assert!(
        paths
            == vec![
                " (0)",
                "a.txt (1)",
                "b.txt (1)",
                "c.txt (1)",
                "group 1 (1)",
                "group 2 (1)",
            ]
    );
}

#[test]
fn save_state_with_process_entries() {
    let (test_dir, _temp_dir) = test_dir();

    // Test that both parent client state and children client state can be set
    // from process_entries callback.
    let mut iter = WalkDirGeneric::<usize>::new(test_dir)
        .sort(true)
        .process_entries(|parent_client_state, children| {
            *parent_client_state += children.len();
            children.iter_mut().for_each(|each_result| {
                if let Ok(each) = each_result {
                    each.client_state = 1;
                }
            });
        })
        .into_iter();

    assert!(iter.next().unwrap().unwrap().client_state == 6); // " (0)"
    assert!(iter.next().unwrap().unwrap().client_state == 1); // "a.txt (1)"
    assert!(iter.next().unwrap().unwrap().client_state == 1); // "b.txt (1)"
    assert!(iter.next().unwrap().unwrap().client_state == 1); // "c.txt (1)"
    assert!(iter.next().unwrap().unwrap().client_state == 2); // "group 1 (1)",
    assert!(iter.next().unwrap().unwrap().client_state == 1); // "group 1/d.txt (2)",
    assert!(iter.next().unwrap().unwrap().client_state == 2); // "group 2 (1)",
    assert!(iter.next().unwrap().unwrap().client_state == 1); // "group 2/e.txt (2)",
    assert!(iter.next().is_none());
}
