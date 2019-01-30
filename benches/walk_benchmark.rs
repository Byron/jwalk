#![allow(dead_code)]
#![allow(unused_imports)]

use criterion::{criterion_group, criterion_main, Criterion};
use ignore::WalkBuilder;
use num_cpus;
use std::cmp;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use walk::walk;
use walkdir::WalkDir;

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

  /*
  c.bench_function("walk_dir", move |b| {
    b.iter(|| for _ in WalkDir::new(linux_dir()) {})
  });

  c.bench_function("ignore_walk", move |b| {
    b.iter(|| {
      for _ in WalkBuilder::new(linux_dir())
        .standard_filters(false)
        .hidden(false)
        .build()
      {}
    })
  });*/

  c.bench_function("par_ignore_walk", move |b| {
    b.iter(|| {
      WalkBuilder::new(linux_dir())
        .hidden(false)
        .standard_filters(false)
        .threads(cmp::min(12, num_cpus::get()))
        .build_parallel()
        .run(move || Box::new(move |_| ignore::WalkState::Continue));
    })
  });

  c.bench_function("walk", |b| {
    b.iter(|| {
      for each_dir_contents in walk(linux_dir()).into_iter() {
        for _each_entry in each_dir_contents.contents.iter() {}
      }
    })
  });

  c.bench_function("wall_first_10", |b| {
    b.iter(|| {
      for each_dir_contents in walk(linux_dir()).into_iter().take(2) {
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
