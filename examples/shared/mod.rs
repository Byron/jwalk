use jwalk::Parallelism;
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Args {
    /// The amount of threads to use. Default to 4 on MacOS and `available_parallelism` on other platforms.
    /// Set to 1 for single-threaded operation.
    #[arg(long, short = 't')]
    pub threads: Option<NonZeroUsize>,
    /// The path from which to start the operation, or `.` if unset
    pub root: Option<PathBuf>,
}

impl Args {
    pub fn threads(&self) -> usize {
        self.threads
            .unwrap_or_else(|| {
                if cfg!(target_vendor = "apple") {
                    NonZeroUsize::new(4).unwrap()
                } else {
                    std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap())
                }
            })
            .get()
    }

    pub fn parallelism(&self) -> Parallelism {
        match self.threads() {
            1 => Parallelism::Serial,
            n => Parallelism::RayonNewPool(n),
        }
    }
}
