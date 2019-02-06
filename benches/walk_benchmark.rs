#![allow(dead_code)]
#![allow(unused_imports)]

use criterion::{criterion_group, criterion_main, Criterion};
use fts;
use ignore::WalkBuilder;
use jwalk::WalkDir;
use num_cpus;
use std::cmp;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use walkdir;

fn linux_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/assets/linux_checkout")
}

fn checkout_linux_if_needed() {
  let linux_dir = linux_dir();
  if !linux_dir.exists() {
    println!("will git clone linux...");
    let output = Command::new("git")
      .arg("clone")
      .arg("https://github.com/BurntSushi/linux.git")
      .arg(&linux_dir)
      .output()
      .expect("failed to git clone linux");
    println!("did git clone linux...{:?}", output);
  }
}

fn walk_benches(c: &mut Criterion) {
  checkout_linux_if_needed();

  c.bench_function("fts", move |b| {
    b.iter(|| {
      for p in fts::walkdir::WalkDir::new(
        fts::walkdir::WalkDirConf::new(linux_dir())
          .no_metadata()
          .no_chdir(),
      ) {}
    })
  });

  /*
  c.bench_function("walkdir", move |b| {
    b.iter(|| for _ in walkdir::WalkDir::new(linux_dir()) {})
  });

  c.bench_function("walkdir (sorted, metadata)", move |b| {
    b.iter(|| {
      for each in
        walkdir::WalkDir::new(linux_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name()))
      {
        let _ = each.unwrap().metadata();
      }
    })
  });

  c.bench_function("ignore (parallel, sorted, metadata)", move |b| {
    b.iter(|| {
      let (tx, rx) = mpsc::channel();
      WalkBuilder::new(linux_dir())
        .hidden(false)
        .standard_filters(false)
        .threads(cmp::min(12, num_cpus::get()))
        .build_parallel()
        .run(move || {
          let tx = tx.clone();
          Box::new(move |dir_entry_result| {
            if let Ok(dir_entry) = dir_entry_result {
              let _ = dir_entry.metadata();
              tx.send(dir_entry.metadata().unwrap()).unwrap();
            }
            ignore::WalkState::Continue
          })
        });
      let mut metadatas: Vec<_> = rx.into_iter().collect();
      metadatas.sort_by(|a, b| a.len().cmp(&b.len()))
    })
  });
  */

  c.bench_function("jwalk (parallel)", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()) {})
  });

  c.bench_function("jwalk (parallel, 2 threads)", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()).num_threads(2) {})
  });

  c.bench_function("jwalk (parallel, metadata)", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()).preload_metadata(true) {})
  });

  c.bench_function("jwalk (parallel, take first 1000)", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()).into_iter().take(1000) {})
  });
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = walk_benches
}

criterion_main!(benches);
