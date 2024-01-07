//! Collect the amount of directories and files as fast as possible.
mod shared;

use clap::Parser;
use jwalk::WalkDirGeneric;
use walkdir::WalkDir;

#[derive(clap::Parser)]
pub struct OurArgs {
    /// Use the `walkdir` crate for walking instead.
    #[arg(long)]
    use_walkdir: bool,

    #[clap(flatten)]
    inner: shared::Args,
}

fn main() {
    let args = OurArgs::parse();

    let parallelism = args.inner.parallelism();
    let threads = args.inner.threads();
    let path = args.inner.root.unwrap_or_else(|| ".".into());
    let (mut dirs, mut files, mut symlinks) = (0, 0, 0);

    if args.use_walkdir {
        for dir_entry_result in WalkDir::new(&path)
            .follow_links(false)
            .follow_root_links(true)
        {
            match dir_entry_result {
                Ok(dir_entry) => {
                    if dir_entry.file_type().is_dir() {
                        dirs += 1;
                    } else if dir_entry.file_type().is_file() {
                        files += 1;
                    } else if dir_entry.file_type().is_symlink() {
                        symlinks += 1
                    }
                }
                Err(error) => {
                    println!("Read dir_entry error: {}", error);
                }
            }
        }
    } else {
        for dir_entry_result in WalkDirGeneric::<((), Option<u64>)>::new(&path)
            .skip_hidden(false)
            .follow_links(false)
            .parallelism(parallelism)
        {
            match dir_entry_result {
                Ok(dir_entry) => {
                    if dir_entry.file_type.is_dir() {
                        dirs += 1;
                    } else if dir_entry.file_type.is_file() {
                        files += 1;
                    } else if dir_entry.file_type.is_symlink() {
                        symlinks += 1
                    }
                }
                Err(error) => {
                    println!("Read dir_entry error: {}", error);
                }
            }
        }
    }
    println!(
        "dirs: {dirs}, files: {files}, symlinks: {symlinks} ({})",
        if args.use_walkdir {
            "walkdir single-threaded".to_string()
        } else {
            format!("threads: {threads}")
        }
    );
}
