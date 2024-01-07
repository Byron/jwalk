mod shared;

use bytesize::ByteSize;
use clap::Parser;
use jwalk::WalkDirGeneric;

fn main() {
    let args = shared::Args::parse();
    let mut total: u64 = 0;

    let parallelism = args.parallelism();
    let path = args.root.unwrap_or_else(|| ".".into());
    for dir_entry_result in WalkDirGeneric::<((), Option<u64>)>::new(&path)
        .skip_hidden(false)
        .parallelism(parallelism)
        .process_read_dir(|_, _, _, dir_entry_results| {
            dir_entry_results.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if !dir_entry.file_type.is_dir() {
                        dir_entry.client_state =
                            Some(dir_entry.metadata().map(|m| m.len()).unwrap_or_default());
                    }
                }
            })
        })
    {
        match dir_entry_result {
            Ok(dir_entry) => {
                if let Some(len) = &dir_entry.client_state {
                    eprintln!("counting {:?}", dir_entry.path());
                    total += len;
                }
            }
            Err(error) => {
                println!("Read dir_entry error: {}", error);
            }
        }
    }

    println!("path: {:?} total bytes: {}", path, ByteSize(total));
}
