use rayon::iter::ParallelIterator;
use rayon::prelude::*;
use std::env;
use std::fs;
use std::path::PathBuf;

pub mod util;

use jwalk::*;
use util::{parallelism_options, test_dir, Dir, ReadOnlyDir};

#[test]
fn empty() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        let wd = WalkDir::new(dir.path()).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        assert_eq!(1, r.ents().len());
        let ent = &r.ents()[0];
        assert!(ent.file_type().is_dir());
        assert!(!ent.path_is_symlink());
        assert_eq!(0, ent.depth());
        assert_eq!(dir.path(), ent.path());
        assert_eq!(dir.path().file_name().unwrap(), ent.file_name());
    }
}

#[test]
fn empty_follow() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        let wd = WalkDir::new(dir.path())
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        assert_eq!(1, r.ents().len());
        let ent = &r.ents()[0];
        assert!(ent.file_type().is_dir());
        assert!(!ent.path_is_symlink());
        assert_eq!(0, ent.depth());
        assert_eq!(dir.path(), ent.path());
        assert_eq!(dir.path().file_name().unwrap(), ent.file_name());
    }
}

#[test]
fn empty_file() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        dir.touch("a");

        let wd = WalkDir::new(dir.path().join("a")).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        assert_eq!(1, r.ents().len());
        let ent = &r.ents()[0];
        assert!(ent.file_type().is_file());
        assert!(!ent.path_is_symlink());
        assert_eq!(0, ent.depth());
        assert_eq!(dir.join("a"), ent.path());
        assert_eq!("a", ent.file_name());
    }
}

#[test]
fn empty_file_follow() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        dir.touch("a");

        let wd = WalkDir::new(dir.path().join("a"))
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        assert_eq!(1, r.ents().len());
        let ent = &r.ents()[0];
        assert!(ent.file_type().is_file());
        assert!(!ent.path_is_symlink());
        assert_eq!(0, ent.depth());
        assert_eq!(dir.join("a"), ent.path());
        assert_eq!("a", ent.file_name());
    }
}

#[test]
fn one_dir() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-one-dir.sh");

        let wd = WalkDir::new(dir.path()).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(2, ents.len());
        let ent = &ents[1];
        assert_eq!(dir.join("a"), ent.path());
        assert_eq!(1, ent.depth());
        assert_eq!("a", ent.file_name());
        assert!(ent.file_type().is_dir());
    }
}

#[test]
fn one_file() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-one-file.sh");

        let wd = WalkDir::new(dir.path()).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(2, ents.len());
        let ent = &ents[1];
        assert_eq!(dir.join("a"), ent.path());
        assert_eq!(1, ent.depth());
        assert_eq!("a", ent.file_name());
        assert!(ent.file_type().is_file());
    }
}

#[test]
fn one_dir_one_file() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-basic.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("foo"),
            dir.join("foo").join("a"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn many_files() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-many-files.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("foo"),
            dir.join("foo").join("a"),
            dir.join("foo").join("b"),
            dir.join("foo").join("c"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn many_dirs() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-many-dirs.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("foo"),
            dir.join("foo").join("a"),
            dir.join("foo").join("b"),
            dir.join("foo").join("c"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn many_mixed() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-many-mixed.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("foo"),
            dir.join("foo").join("a"),
            dir.join("foo").join("b"),
            dir.join("foo").join("c"),
            dir.join("foo").join("d"),
            dir.join("foo").join("e"),
            dir.join("foo").join("f"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn nested() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-nested.sh");
        let nested = PathBuf::from("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("a"),
            dir.join("a/b"),
            dir.join("a/b/c"),
            dir.join("a/b/c/d"),
            dir.join("a/b/c/d/e"),
            dir.join("a/b/c/d/e/f"),
            dir.join("a/b/c/d/e/f/g"),
            dir.join("a/b/c/d/e/f/g/h"),
            dir.join("a/b/c/d/e/f/g/h/i"),
            dir.join("a/b/c/d/e/f/g/h/i/j"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y"),
            dir.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z"),
            dir.join(&nested).join("A"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn siblings() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-siblings.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("bar"),
            dir.join("bar").join("a"),
            dir.join("bar").join("b"),
            dir.join("foo"),
            dir.join("foo").join("a"),
            dir.join("foo").join("b"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn sym_root_file_nofollow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-file.sh");

        let wd = WalkDir::new(dir.join("a-link"))
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(1, ents.len());
        let link = &ents[0];

        assert_eq!(dir.join("a-link"), link.path());

        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(0, link.depth());

        assert!(link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());
    }
}

#[test]
fn sym_root_file_follow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-file.sh");

        let wd = WalkDir::new(dir.join("a-link"))
            .sort(true)
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        let link = &ents[0];

        assert_eq!(dir.join("a-link"), link.path());

        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(0, link.depth());

        assert!(!link.file_type().is_symlink());
        assert!(link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(!link.metadata().unwrap().file_type().is_symlink());
        assert!(link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());
    }
}

#[test]
fn sym_root_dir_nofollow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-dir.sh");

        let wd = WalkDir::new(dir.join("a-link"))
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(2, ents.len());
        let link = &ents[0];

        assert_eq!(dir.join("a-link"), link.path());

        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(0, link.depth());

        assert!(link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());

        let link_zzz = &ents[1];
        assert_eq!(dir.join("a-link").join("zzz"), link_zzz.path());
        assert!(!link_zzz.path_is_symlink());
    }
}

#[test]
fn sym_root_dir_follow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-dir.sh");

        let wd = WalkDir::new(dir.join("a-link"))
            .sort(true)
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(2, ents.len());
        let link = &ents[0];

        assert_eq!(dir.join("a-link"), link.path());

        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(0, link.depth());

        assert!(!link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(link.file_type().is_dir());

        assert!(!link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(link.metadata().unwrap().is_dir());

        let link_zzz = &ents[1];
        assert_eq!(dir.join("a-link").join("zzz"), link_zzz.path());
        assert!(!link_zzz.path_is_symlink());
    }
}

#[test]
fn sym_file_nofollow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-file.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(3, ents.len());
        let (src, link) = (&ents[1], &ents[2]);

        assert_eq!(dir.join("a"), src.path());
        assert_eq!(dir.join("a-link"), link.path());

        assert!(!src.path_is_symlink());
        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(1, src.depth());
        assert_eq!(1, link.depth());

        assert!(src.file_type().is_file());
        assert!(link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(src.metadata().unwrap().is_file());
        assert!(link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());
    }
}

#[test]
fn sym_file_follow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-file.sh");

        let wd = WalkDir::new(dir.path())
            .sort(true)
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(3, ents.len());
        let (src, link) = (&ents[1], &ents[2]);

        assert_eq!(dir.join("a"), src.path());
        assert_eq!(dir.join("a-link"), link.path());

        assert!(!src.path_is_symlink());
        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(1, src.depth());
        assert_eq!(1, link.depth());

        assert!(src.file_type().is_file());
        assert!(!link.file_type().is_symlink());
        assert!(link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(src.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().file_type().is_symlink());
        assert!(link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());
    }
}

#[test]
fn sym_dir_nofollow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-dir.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(4, ents.len());
        let (src, link) = (&ents[1], &ents[3]);

        assert_eq!(dir.join("a"), src.path());
        assert_eq!(dir.join("a-link"), link.path());

        assert!(!src.path_is_symlink());
        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(1, src.depth());
        assert_eq!(1, link.depth());

        assert!(src.file_type().is_dir());
        assert!(link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(!link.file_type().is_dir());

        assert!(src.metadata().unwrap().is_dir());
        assert!(link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(!link.metadata().unwrap().is_dir());
    }
}

#[test]
fn sym_dir_follow() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-root-dir.sh");

        let wd = WalkDir::new(dir.path())
            .follow_links(true)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let ents = r.ents();
        assert_eq!(5, ents.len());
        let (src, link) = (&ents[1], &ents[3]);

        assert_eq!(dir.join("a"), src.path());
        assert_eq!(dir.join("a-link"), link.path());

        assert!(!src.path_is_symlink());
        assert!(link.path_is_symlink());

        assert_eq!(PathBuf::from("a"), fs::read_link(link.path()).unwrap());

        assert_eq!(1, src.depth());
        assert_eq!(1, link.depth());

        assert!(src.file_type().is_dir());
        assert!(!link.file_type().is_symlink());
        assert!(!link.file_type().is_file());
        assert!(link.file_type().is_dir());

        assert!(src.metadata().unwrap().is_dir());
        assert!(!link.metadata().unwrap().file_type().is_symlink());
        assert!(!link.metadata().unwrap().is_file());
        assert!(link.metadata().unwrap().is_dir());

        let (src_zzz, link_zzz) = (&ents[2], &ents[4]);
        assert_eq!(dir.join("a").join("zzz"), src_zzz.path());
        assert_eq!(dir.join("a-link").join("zzz"), link_zzz.path());
        assert!(!src_zzz.path_is_symlink());
        assert!(!link_zzz.path_is_symlink());
    }
}

#[test]
fn sym_noloop() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        dir.mkdirp("a/b/c");
        dir.symlink_dir("a", "a/b/c/a-link");

        let wd = WalkDir::new(dir.path()).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        // There's no loop if we aren't following symlinks.
        r.assert_no_errors();

        assert_eq!(5, r.ents().len());
    }
}

#[test]
fn sym_loop_detect() {
    for parallelism in parallelism_options() {
        let dir = Dir::tmp();
        dir.mkdirp("a/b/c");
        dir.symlink_dir("a", "a/b/c/a-link");

        let wd = WalkDir::new(dir.path())
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);

        let (ents, errs) = (r.ents(), r.errs());
        assert_eq!(4, ents.len());
        assert_eq!(1, errs.len());

        let err = &errs[0];

        let expected = dir.join("a/b/c/a-link");
        assert_eq!(Some(&*expected), err.path());

        let expected = dir.join("a");
        assert_eq!(Some(&*expected), err.loop_ancestor());

        assert_eq!(4, err.depth());
        assert!(err.io_error().is_none());
    }
}

#[test]
fn sym_self_loop_no_error() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-self-loop.sh");

        let wd = WalkDir::new(dir.path()).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        // No errors occur because even though the symlink points to nowhere, it
        // is never followed, and thus no error occurs.
        r.assert_no_errors();
        assert_eq!(2, r.ents().len());

        let ent = &r.ents()[1];
        assert_eq!(dir.join("a"), ent.path());
        assert!(ent.path_is_symlink());

        assert!(ent.file_type().is_symlink());
        assert!(!ent.file_type().is_file());
        assert!(!ent.file_type().is_dir());

        assert!(ent.metadata().unwrap().file_type().is_symlink());
        assert!(!ent.metadata().unwrap().file_type().is_file());
        assert!(!ent.metadata().unwrap().file_type().is_dir());
    }
}

#[test]
fn sym_file_self_loop_io_error() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-self-loop.sh");

        let wd = WalkDir::new(dir.path())
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);

        let (ents, errs) = (r.ents(), r.errs());
        assert_eq!(1, ents.len());
        assert_eq!(1, errs.len());

        let err = &errs[0];

        let expected = dir.join("a");
        assert_eq!(Some(&*expected), err.path());
        assert_eq!(1, err.depth());
        assert!(err.loop_ancestor().is_none());
        assert!(err.io_error().is_some());
    }
}

#[test]
fn sym_dir_self_loop_io_error() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-symlink-self-loop.sh");

        let wd = WalkDir::new(dir.path())
            .follow_links(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);

        let (ents, errs) = (r.ents(), r.errs());
        assert_eq!(1, ents.len());
        assert_eq!(1, errs.len());

        let err = &errs[0];

        let expected = dir.join("a");
        assert_eq!(Some(&*expected), err.path());
        assert_eq!(1, err.depth());
        assert!(err.loop_ancestor().is_none());
        assert!(err.io_error().is_some());
    }
}

#[test]
fn min_depth_1() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test.sh");

        let wd = WalkDir::new(dir.path())
            .min_depth(1)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.join("a"), dir.join("a").join("b")];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn min_depth_2() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test.sh");

        let wd = WalkDir::new(dir.path())
            .min_depth(2)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.join("a").join("b")];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn max_depth_0() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test.sh");

        let wd = WalkDir::new(dir.path())
            .max_depth(0)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.path().to_path_buf()];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn max_depth_1() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test.sh");

        let wd = WalkDir::new(dir.path())
            .max_depth(1)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.path().to_path_buf(), dir.join("a")];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn max_depth_2() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test.sh");

        let wd = WalkDir::new(dir.path())
            .max_depth(2)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("a"),
            dir.join("a").join("b"),
        ];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn min_max_depth_diff_0() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test-3.sh");

        let wd = WalkDir::new(dir.path())
            .min_depth(2)
            .max_depth(2)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.join("a").join("b")];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn min_max_depth_diff_1() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-depth-test-3.sh");

        let wd = WalkDir::new(dir.path())
            .min_depth(1)
            .max_depth(2)
            .sort(true)
            .parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![dir.join("a"), dir.join("a").join("b")];
        assert_eq!(expected, r.paths());
    }
}

#[test]
fn sort() {
    for parallelism in parallelism_options() {
        let dir = ReadOnlyDir::from_fixture("make-sort-test.sh");

        let wd = WalkDir::new(dir.path()).sort(true).parallelism(parallelism);
        let r = dir.run_recursive(wd);
        r.assert_no_errors();

        let expected = vec![
            dir.path().to_path_buf(),
            dir.join("foo"),
            dir.join("foo").join("bar"),
            dir.join("foo").join("bar").join("baz"),
            dir.join("foo").join("bar").join("baz").join("abc"),
            dir.join("quux"),
        ];
        assert_eq!(expected, r.paths());
    }
}

fn local_paths(walk_dir: WalkDir) -> Vec<String> {
    let root = walk_dir.root().to_owned();
    walk_dir
        .into_iter()
        .map(|each_result| {
            let each_entry = each_result.unwrap();
            if let Some(err) = each_entry.read_children.as_ref().and_then(|rc| rc.error()) {
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
fn walk_serial() {
    let (test_dir, _temp_dir) = test_dir();

    let paths = local_paths(
        WalkDir::new(test_dir)
            .parallelism(Parallelism::Serial)
            .sort(true),
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
}

#[test]
fn sort_by_name_rayon_custom_2_threads() {
    let (test_dir, _temp_dir) = test_dir();
    let paths = local_paths(
        WalkDir::new(test_dir)
            .parallelism(Parallelism::RayonNewPool(2))
            .sort(true),
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
}

#[test]
fn walk_rayon_global() {
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
        ]
    );
}

#[test]
fn walk_rayon_no_lockup() {
    // Without jwalk_par_bridge this locks (pre rayon 1.6.1)
    // This test now passes without needing jwalk_par_bridge
    // and that code has been removed from jwalk.
    let pool = std::sync::Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap(),
    );
    let _: Vec<_> = WalkDir::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .parallelism(Parallelism::RayonExistingPool {
            pool,
            busy_timeout: std::time::Duration::from_millis(500).into(),
        })
        .process_read_dir(|_, _, _, dir_entry_results| {
            for dir_entry_result in dir_entry_results {
                let _ = dir_entry_result
                    .as_ref()
                    .map(|dir_entry| dir_entry.metadata());
            }
        })
        .sort(true)
        .into_iter()
        .collect();
}

#[test]
fn combine_with_rayon_no_lockup_1() {
    // only run this test if linux_checkout present
    let linux_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/assets/linux_checkout");
    if linux_dir.exists() {
        rayon::scope(|_| {
            eprintln!("WalkDir…");
            for _entry in WalkDir::new(linux_dir) {}
            eprintln!("WalkDir completed");
        });
    }
}

#[test]
fn combine_with_rayon_no_lockup_2() {
    WalkDir::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .sort(true)
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            let dir_entry = dir_entry_result.ok()?;
            if dir_entry.file_type().is_file() {
                let path = dir_entry.path();
                let text = std::fs::read_to_string(path).ok()?;
                if text.contains("hello world") {
                    return Some(true);
                }
            }
            None
        })
        .count();
}

#[test]
fn see_hidden_files() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();
        let paths = local_paths(
            WalkDir::new(test_dir)
                .skip_hidden(false)
                .sort(true)
                .parallelism(parallelism),
        );
        assert!(paths.contains(&"group 2/.hidden_file.txt (2)".to_string()));
    }
}

#[test]
fn walk_file() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();
        let walk_dir = WalkDir::new(test_dir.join("a.txt")).parallelism(parallelism);
        let mut iter = walk_dir.into_iter();
        assert_eq!(
            iter.next().unwrap().unwrap().file_name.to_str().unwrap(),
            "a.txt"
        );
        assert!(iter.next().is_none());
    }
}

#[test]
fn walk_file_serial() {
    let (test_dir, _temp_dir) = test_dir();
    let walk_dir = WalkDir::new(test_dir.join("a.txt")).parallelism(Parallelism::Serial);
    let mut iter = walk_dir.into_iter();
    assert_eq!(
        iter.next().unwrap().unwrap().file_name.to_str().unwrap(),
        "a.txt"
    );
    assert!(iter.next().is_none());
}

#[test]
fn error_when_path_does_not_exist() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();
        let walk_dir = WalkDir::new(test_dir.join("path_does_not_exist")).parallelism(parallelism);
        let mut iter = walk_dir.into_iter();
        assert!(iter.next().unwrap().is_err());
        assert!(iter.next().is_none());
    }
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
    fs_extra::remove_items(&[test_dir.join("group 2")]).unwrap();

    let _ = iter.next().unwrap().is_ok(); // "a.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "b.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "c.txt (1)",
    let _ = iter.next().unwrap().is_ok(); // "group 1 (1)",
    let _ = iter.next().unwrap().is_ok(); // "group 1/d.txt (2)",

    // group 2 is read correctly, since it was read before path removed.
    let group_2 = iter.next().unwrap().unwrap();

    // group 2 content error IS set, since path is removed when try read_dir for
    // group 2 path.
    assert!(group_2
        .read_children
        .as_ref()
        .and_then(|rc| rc.error())
        .is_some());

    // done!
    assert!(iter.next().is_none());
}

#[test]
fn walk_root() {
    for parallelism in parallelism_options() {
        let paths: Vec<_> = WalkDir::new("/")
            .max_depth(1)
            .sort(true)
            .parallelism(parallelism)
            .into_iter()
            .filter_map(|each| Some(each.ok()?.path()))
            .collect();
        assert_eq!(paths.first().unwrap().to_str().unwrap(), "/");
    }
}

#[test]
fn filter_groups_with_process_read_dir() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();
        let paths = local_paths(
            WalkDir::new(test_dir)
                .sort(true)
                .parallelism(parallelism)
                // Filter groups out manually
                .process_read_dir(|_depth, _path, _parent, children| {
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
        assert_eq!(paths, vec![" (0)", "a.txt (1)", "b.txt (1)", "c.txt (1)",]);
    }
}

#[test]
fn filter_group_children_with_process_read_dir() {
    for parallelism in parallelism_options() {
        let (test_dir, _temp_dir) = test_dir();
        let paths = local_paths(
            WalkDir::new(test_dir)
                .sort(true)
                .parallelism(parallelism)
                // Filter group children
                .process_read_dir(|_depth, _path, _parent, children| {
                    children.iter_mut().for_each(|each_result| {
                        if let Ok(each) = each_result {
                            if each.file_name.to_string_lossy().starts_with("group") {
                                each.read_children = None;
                            }
                        }
                    });
                }),
        );
        assert_eq!(
            paths,
            vec![
                " (0)",
                "a.txt (1)",
                "b.txt (1)",
                "c.txt (1)",
                "group 1 (1)",
                "group 2 (1)",
            ]
        );
    }
}

#[test]
fn pass_readdir_state_with_process_read_dir() {
    let (test_dir, _temp_dir) = test_dir();
    let iter = WalkDirGeneric::<(Option<String>, ())>::new(&test_dir)
        .sort(true)
        .process_read_dir(|_depth, path, state, children| {
            if let Some(state) = state.as_deref() {
                let fname = path.file_name().and_then(|fname| fname.to_str()).unwrap();
                assert_eq!(state, fname);
            }
            children.retain_mut(|each_result| {
                if let Ok(each) = each_result {
                    let fname = each.file_name().to_str().unwrap().to_owned();
                    if let Some(ref mut rc) = each.read_children {
                        rc.client_read_state = Some(Some(fname));
                        return true;
                    }
                }
                false
            });
        })
        .into_iter();
    assert_eq!(
        iter.map(|e| {
            let e = e.unwrap();
            format!(
                "{} ({})",
                e.path().strip_prefix(&test_dir).unwrap().to_str().unwrap(),
                e.depth()
            )
        })
        .collect::<Vec<_>>(),
        vec![" (0)", "group 1 (1)", "group 2 (1)",]
    );
}

#[test]
fn test_read_linux() {
    for parallelism in parallelism_options() {
        // only run this test if linux_checkout present
        let linux_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benches/assets/linux_checkout");
        if linux_dir.exists() {
            for each in WalkDir::new(&linux_dir).parallelism(parallelism) {
                let path = each.unwrap().path();
                assert!(path.exists(), "{:?}", path);
            }
        }
    }
}
