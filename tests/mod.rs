use jwalk::*;
use std::path::PathBuf;

fn test_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/test_dir")
}

fn local_paths(walk_dir: WalkDir) -> Vec<String> {
  let test_dir = test_dir();
  walk_dir
    .into_iter()
    .map(|each_result| {
      let each_entry = each_result.unwrap();
      let path = each_entry.path().to_path_buf();
      let path = path.strip_prefix(&test_dir).unwrap().to_path_buf();
      let mut path_string = path.to_str().unwrap().to_string();
      path_string.push_str(&format!(" ({})", each_entry.depth()));
      path_string
    })
    .collect()
}

#[test]
fn walk() {
  let paths = local_paths(WalkDir::new(test_dir()));
  assert!(paths.contains(&"b.txt (1)".to_string()));
  assert!(paths.contains(&"group 1 (1)".to_string()));
  assert!(paths.contains(&"group 1/d.txt (2)".to_string()));
}

#[test]
fn sort_by_name_single_thread() {
  let paths = local_paths(WalkDir::new(test_dir()).num_threads(1).sort(true));
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
fn sort_by_name_rayon_pool_global() {
  let paths = local_paths(WalkDir::new(test_dir()).sort(true));
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
fn sort_by_name_rayon_pool_2_threads() {
  let paths = local_paths(WalkDir::new(test_dir()).num_threads(2).sort(true));
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
  let paths = local_paths(WalkDir::new(test_dir()).skip_hidden(false).sort(true));
  assert!(paths.contains(&"group 2/.hidden_file.txt (2)".to_string()));
}

#[test]
fn max_depth() {
  let paths = local_paths(WalkDir::new(test_dir()).max_depth(1).sort(true));
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
  let walk_dir = WalkDir::new(test_dir().join("a.txt"));
  let mut iter = walk_dir.into_iter();
  assert!(iter.next().unwrap().unwrap().file_name().to_str().unwrap() == "a.txt");
  assert!(iter.next().is_none());
}

#[test]
fn walk_root() {
  let mut iter = walkdir::WalkDir::new("/").max_depth(1).into_iter();
  assert!(iter.next().unwrap().unwrap().file_name() == "/");
}
