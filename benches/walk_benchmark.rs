#![allow(dead_code)]
#![allow(unused_imports)]

use criterion::{criterion_group, criterion_main, Criterion};
use ignore::WalkBuilder;
use jwalk::core::walk;
use jwalk::WalkDir;
use num_cpus;
use rayon::prelude::*;
use std::cmp;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
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

  c.bench_function("walkdir::WalkDir", move |b| {
    b.iter(|| for _ in walkdir::WalkDir::new(linux_dir()) {})
  });

  c.bench_function("walkdir::WalkDir_sorted", move |b| {
    b.iter(|| {
      for _ in walkdir::WalkDir::new(linux_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
      }
    })
  });

  c.bench_function("ignore::WalkParallel", move |b| {
    b.iter(|| {
      WalkBuilder::new(linux_dir())
        .hidden(false)
        .standard_filters(false)
        .threads(cmp::min(12, num_cpus::get()))
        .build_parallel()
        .run(move || Box::new(move |_| ignore::WalkState::Continue));
    })
  });

  c.bench_function("jwalk::WalkDir", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()) {})
  });

  c.bench_function("jwalk::WalkDir_1", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()).into_iter().take(1) {})
  });

  c.bench_function("jwalk::WalkDir_1000", |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()).into_iter().take(1000) {})
  });

  c.bench_function("jwalk::core::walk", |b| {
    b.iter(|| {
      let dir_list_iter = walk(
        linux_dir(),
        0,
        |_path, state, mut entries| {
          entries.par_sort_by(|a, b| a.file_name().cmp(b.file_name()));
          (state, entries)
        },
        |_path, _error| true,
      );

      for each_dir_contents in dir_list_iter {
        for _each_entry in each_dir_contents.contents.iter() {}
      }
    })
  });
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = walk_benches
}

criterion_main!(benches);
