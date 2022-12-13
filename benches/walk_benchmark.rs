#![allow(dead_code)]
#![allow(unused_imports)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ignore::WalkBuilder;
use jwalk::{Error, Parallelism, WalkDir, WalkDirGeneric};
use num_cpus;
use rayon::prelude::*;
use std::cmp;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use walkdir;

fn big_dir() -> PathBuf {
    std::env::var_os("JWALK_BENCHMARK_DIR")
        .expect(
            "the JWALK_BENCHMARK_DIR must be set to the directory to traverse for the benchmark",
        )
        .into()
}

fn checkout_linux_if_needed() {
    let linux_dir = big_dir();
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

    c.bench_function("rayon (unsorted, n threads)", |b| {
        b.iter(|| black_box(rayon_recursive_descent(big_dir(), None, false)))
    });

    c.bench_function("rayon (unsorted, metadata, n threads)", |b| {
        b.iter(|| black_box(rayon_recursive_descent(big_dir(), None, true)))
    });

    c.bench_function("jwalk (unsorted, n threads)", |b| {
        b.iter(|| for _ in WalkDir::new(big_dir()) {})
    });

    c.bench_function("jwalk (sorted, n threads)", |b| {
        b.iter(|| for _ in WalkDir::new(big_dir()).sort(true) {})
    });

    c.bench_function("jwalk (sorted, metadata, n threads)", |b| {
        b.iter(|| {
            for _ in WalkDirGeneric::<((), Option<Result<Metadata, Error>>)>::new(big_dir())
                .sort(true)
                .process_read_dir(|_, _, _, dir_entry_results| {
                    dir_entry_results.iter_mut().for_each(|dir_entry_result| {
                        if let Ok(dir_entry) = dir_entry_result {
                            dir_entry.client_state = Some(dir_entry.metadata());
                        }
                    })
                })
            {}
        })
    });

    c.bench_function("jwalk (sorted, n threads, first 100)", |b| {
        b.iter(
            || {
                for _ in WalkDir::new(big_dir()).sort(true).into_iter().take(100) {}
            },
        )
    });

    c.bench_function("jwalk (unsorted, 2 threads)", |b| {
        b.iter(
            || {
                for _ in WalkDir::new(big_dir()).parallelism(Parallelism::RayonNewPool(2)) {}
            },
        )
    });

    c.bench_function("jwalk (unsorted, 1 thread)", |b| {
        b.iter(
            || {
                for _ in WalkDir::new(big_dir()).parallelism(Parallelism::Serial) {}
            },
        )
    });

    c.bench_function("jwalk (sorted, 1 thread)", |b| {
        b.iter(|| {
            for _ in WalkDir::new(big_dir())
                .sort(true)
                .parallelism(Parallelism::Serial)
            {}
        })
    });

    c.bench_function("jwalk (sorted, metadata, 1 thread)", |b| {
        b.iter(|| {
            for _ in WalkDirGeneric::<((), Option<Result<Metadata, Error>>)>::new(big_dir())
                .sort(true)
                .parallelism(Parallelism::Serial)
                .process_read_dir(|_, _, _, dir_entry_results| {
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
            WalkBuilder::new(big_dir())
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
            WalkBuilder::new(big_dir())
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
            WalkBuilder::new(big_dir())
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
            WalkBuilder::new(big_dir())
                .hidden(false)
                .standard_filters(false)
                .threads(cmp::min(2, num_cpus::get()))
                .build_parallel()
                .run(move || Box::new(move |_| ignore::WalkState::Continue));
        })
    });

    c.bench_function("walkdir (unsorted, 1 thread)", move |b| {
        b.iter(|| for _ in walkdir::WalkDir::new(big_dir()) {})
    });

    c.bench_function("walkdir (sorted, 1 thread)", move |b| {
        b.iter(|| {
            for _ in
                walkdir::WalkDir::new(big_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name()))
            {
            }
        })
    });

    c.bench_function("walkdir (sorted, metadata, 1 thread)", move |b| {
        b.iter(|| {
            for each in
                walkdir::WalkDir::new(big_dir()).sort_by(|a, b| a.file_name().cmp(b.file_name()))
            {
                let _ = each.unwrap().metadata();
            }
        })
    });
}

fn rayon_recursive_descent(
    root: impl AsRef<Path>,
    file_type: Option<std::fs::FileType>,
    get_file_metadata: bool,
) {
    let root = root.as_ref();
    let (_metadata, is_dir) = file_type
        .map(|ft| {
            (
                if !ft.is_dir() && get_file_metadata {
                    std::fs::symlink_metadata(root).ok()
                } else {
                    None
                },
                ft.is_dir(),
            )
        })
        .or_else(|| {
            std::fs::symlink_metadata(root)
                .map(|m| {
                    let is_dir = m.file_type().is_dir();
                    (Some(m), is_dir)
                })
                .ok()
        })
        .unwrap_or((None, false));

    if is_dir {
        std::fs::read_dir(root)
            .map(|iter| {
                iter.filter_map(Result::ok)
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .map(|entry| {
                        rayon_recursive_descent(
                            entry.path(),
                            entry.file_type().ok(),
                            get_file_metadata,
                        )
                    })
                    .for_each(|_| {})
            })
            .unwrap_or_default()
    };
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets = walk_benches
}

criterion_main!(benches);
