extern crate jwalk;

use jwalk::{WalkDir, Parallelism};
use rayon::prelude::*;

fn main() {
    let rounds = vec![0, 1];

    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .expect("Failed to initialize worker thread pool");

    let jwalk_pool = std::sync::Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap(),
    );
    
    // Does finish if jwalk uses own pool with 1 thread
    rounds.par_iter().for_each(|round| {
        eprintln!("Round {round}…");
        for _entry in WalkDir::new(".").parallelism(Parallelism::RayonExistingPool(jwalk_pool.clone())) {}
        eprintln!("Round {round} completed");
    });

    // Does not finish if jwalk uses shared pool with 1 thread
    rounds.par_iter().for_each(|round| {
        eprintln!("Round {round}…");
        for _entry in WalkDir::new(".") {}
        eprintln!("Round {round} completed");
    });
}