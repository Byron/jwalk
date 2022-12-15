use jwalk::WalkDir;
use rayon::prelude::*;

#[test]
fn works() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .expect("Failed to initialize worker thread pool");
    // Does not finish if jwalk uses shared pool with 1 thread, but we can detect this issue and signal this with an error.
    (0..=1)
        .collect::<Vec<usize>>()
        .par_iter()
        .for_each(|_round| {
            for entry in WalkDir::new(".").parallelism(jwalk::Parallelism::RayonDefaultPool {
                busy_timeout: std::time::Duration::from_millis(10),
            }) {
                match entry {
                    Ok(_) => panic!("Must detect deadlock"),
                    Err(err)
                        if err.io_error().expect("is IO").kind() == std::io::ErrorKind::Other => {}
                    Err(err) => panic!("Unexpected error: {:?}", err),
                }
            }
        });
}
