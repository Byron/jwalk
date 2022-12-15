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
        .for_each(|round| {
            let generic = WalkDir::new(".").parallelism(jwalk::Parallelism::RayonDefaultPool {
                busy_timeout: std::time::Duration::from_millis(10),
            });
            if *round == 0 {
                for entry in generic {
                    match entry {
                        Ok(_) => panic!("Must detect deadlock"),
                        Err(err) if err.is_busy() => {}
                        Err(err) => panic!("Unexpected error: {:?}", err),
                    }
                }
            } else {
                assert!(matches!(generic.try_into_iter(), Err(err) if err.is_busy()));
            }
        });
}
