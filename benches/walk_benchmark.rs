#![allow(dead_code)]
#![allow(unused_imports)]

use criterion::{criterion_group, criterion_main, Criterion};
use ignore::WalkBuilder;
use jwalk::{Parallelism, WalkDir, WalkDirGeneric};
use num_cpus;
use std::cmp;
use std::fs::Metadata;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use walkdir;
use std::io::Error;

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

    c.bench_function("jwalk (unsorted, n threads)", |b| {
        b.iter(|| for _ in WalkDir::new(linux_dir()) {})
    });

    c.bench_function("jwalk (sorted, n threads)", |b| {
        b.iter(|| for _ in WalkDir::new(linux_dir()).sort(true) {})
    });

    c.bench_function("jwalk (sorted, metadata, n threads)", |b| {
        b.iter(
            || {
                for _ in WalkDirGeneric::<((), (Option<Result<Metadata, Error>>))>::new(linux_dir())
                    .sort(true)
                    .process_read_dir(|_, dir_entry_results| {
                        dir_entry_results.iter_mut().for_each(|dir_entry_result| {
                            if let Ok(dir_entry) = dir_entry_result {
                                dir_entry.client_state = Some(dir_entry.metadata());                            
                            }
                        })
                    })
                {}
            },
        )
    });

    c.bench_function("jwalk (sorted, n threads, first 100)", |b| {
        b.iter(
            || {
                for _ in WalkDir::new(linux_dir()).sort(true).into_iter().take(100) {}
            },
        )
    });

    c.bench_function("jwalk (unsorted, 2 threads)", |b| {
        b.iter(|| for _ in WalkDir::new(linux_dir()).parallelism(Parallelism::RayonNewPool(2)) {})
    });

    c.bench_function("jwalk (unsorted, 1 thread)", |b| {
        b.iter(|| for _ in WalkDir::new(linux_dir()).parallelism(Parallelism::Serial) {})
    });

    c.bench_function("jwalk (sorted, 1 thread)", |b| {
        b.iter(|| {
            for _ in WalkDir::new(linux_dir())
                .sort(true)
                .parallelism(Parallelism::Serial)
            {}
        })
    });

    c.bench_function("jwalk (sorted, metadata, 1 thread)", |b| {
        b.iter(|| {
            for _ in WalkDirGeneric::<((), (Option<Result<Metadata, Error>>))>::new(linux_dir())
                .sort(true)
                .parallelism(Parallelism::Serial)
                .process_read_dir(|_, dir_entry_results| {
                    dir_entry_results.iter_mut().for_each(|dir_entry_result| {
                        if let Ok(dir_entry) = dir_entry_result {
                            dir_entry.client_state = Some(dir_entry.metadata());                            
                        }
                    })
                })
            {}
        })
    });

    c.bench_function("ignore (unsorted, n threads)", move |b| {
        b.iter(|| {
            WalkBuilder::new(linux_dir())
                .hidden(false)
                .standard_filters(false)
                .threads(cmp::min(12, num_cpus::get()))
                .build_parallel()
                .run(move || Box::new(move |_| ignore::WalkState::Continue));
        })
    });

    c.bench_function("ignore (sorted, n threads)", move |b| {
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
                            tx.send(dir_entry.file_name().to_owned()).unwrap();
                        }
                        ignore::WalkState::Continue
                    })
                });
            let mut metadatas: Vec<_> = rx.into_iter().collect();
            metadatas.sort_by(|a, b| a.len().cmp(&b.len()))
        })
    });

    c.bench_function("ignore (sorted, metadata, n threads)", move |b| {
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
                            tx.send(dir_entry.file_name().to_owned()).unwrap();
                        }
                        ignore::WalkState::Continue
                    })
                });
            let mut metadatas: Vec<_> = rx.into_iter().collect();
            metadatas.sort_by(|a, b| a.len().cmp(&b.len()))
        })
    });

    c.bench_function("ignore (unsorted, 2 threads)", move |b| {
        b.iter(|| {
            WalkBuilder::new(linux_dir())
                .hidden(false)
                .standard_filters(false)
                .threads(cmp::min(2, num_cpus::get()))
                .build_parallel()
                .run(move || Box::new(move |_| ignore::WalkState::Continue));
        })
    });

    c.bench_function("walkdir (unsorted, 1 thread)", move |b| {
        b.iter(|| for _ in walkdir::WalkDir::new(linux_dir()) {})
    });

    c.bench_function("walkdir (sorted, 1 thread)", move |b| {
        b.iter(|| {
            for _ in
                walkdir::WalkDir::new(linux_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name()))
            {
            }
        })
    });

    c.bench_function("walkdir (sorted, metadata, 1 thread)", move |b| {
        b.iter(|| {
            for each in
                walkdir::WalkDir::new(linux_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name()))
            {
                let _ = each.unwrap().metadata();
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
