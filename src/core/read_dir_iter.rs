use std::sync::Arc;

use super::*;
use crate::Result;

/// Client's read dir function.
pub(crate) type ReadDirCallback<C> =
    dyn Fn(ReadDirSpec<C>) -> Result<ReadDir<C>> + Send + Sync + 'static;

/// Result<ReadDir> Iterator.
///
/// Yields ReadDirs (results of fs::read_dir) in order required for recursive
/// directory traversal. Depending on Walk/ParWalk state these reads might be
/// computed in parallel.
pub enum ReadDirIter<C: ClientState> {
    Walk {
        read_dir_spec_stack: Vec<ReadDirSpec<C>>,
        core_read_dir_callback: Arc<ReadDirCallback<C>>,
    },
    ParWalk {
        read_dir_result_iter: OrderedQueueIter<Result<ReadDir<C>>>,
    },
}

impl<C: ClientState> ReadDirIter<C> {
    pub(crate) fn try_new(
        read_dir_specs: Vec<ReadDirSpec<C>>,
        parallelism: Parallelism,
        core_read_dir_callback: Arc<ReadDirCallback<C>>,
    ) -> Option<Self> {
        if let Parallelism::Serial = parallelism {
            ReadDirIter::Walk {
                read_dir_spec_stack: read_dir_specs,
                core_read_dir_callback,
            }
        } else {
            let stop = Arc::new(AtomicBool::new(false));
            let read_dir_result_queue = new_ordered_queue(stop.clone(), Ordering::Strict);
            let (read_dir_result_queue, read_dir_result_iter) = read_dir_result_queue;
            let read_dir_spec_queue = new_ordered_queue(stop.clone(), Ordering::Relaxed);
            let (read_dir_spec_queue, read_dir_spec_iter) = read_dir_spec_queue;

            for (i, read_dir_spec) in read_dir_specs.into_iter().enumerate() {
                read_dir_spec_queue
                    .push(Ordered::new(read_dir_spec, IndexPath::new(vec![0]), i))
                    .unwrap();
            }

            let run_context = RunContext {
                stop,
                read_dir_spec_queue,
                read_dir_result_queue,
                core_read_dir_callback,
            };

            let (startup_tx, startup_rx) = parallelism
                .timeout()
                .map(|duration| {
                    let (tx, rx) = crossbeam::channel::unbounded();
                    (Some(tx), Some((rx, duration)))
                })
                .unwrap_or((None, None));
            parallelism.spawn(move || {
                if let Some(tx) = startup_tx {
                    if tx.send(()).is_err() {
                        // rayon didn't install this function in time so the listener exited. Do the same.
                        return;
                    }
                }
                read_dir_spec_iter.par_bridge().for_each_with(
                    run_context,
                    |run_context, ordered_read_dir_spec| {
                        multi_threaded_walk_dir(ordered_read_dir_spec, run_context);
                    },
                );
            });
            if startup_rx.map_or(false, |(rx, duration)| rx.recv_timeout(duration).is_err()) {
                return None;
            }
            ReadDirIter::ParWalk {
                read_dir_result_iter,
            }
        }
        .into()
    }
}

impl<C: ClientState> Iterator for ReadDirIter<C> {
    type Item = Result<ReadDir<C>>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ReadDirIter::Walk {
                read_dir_spec_stack,
                core_read_dir_callback,
            } => {
                let read_dir_spec = read_dir_spec_stack.pop()?;
                let read_dir_result = core_read_dir_callback(read_dir_spec);

                if let Ok(read_dir) = read_dir_result.as_ref() {
                    for each_spec in read_dir
                        .read_children_specs()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                    {
                        read_dir_spec_stack.push(each_spec);
                    }
                }

                Some(read_dir_result)
            }

            ReadDirIter::ParWalk {
                read_dir_result_iter,
            } => read_dir_result_iter
                .next()
                .map(|read_dir_result| read_dir_result.value),
        }
    }
}

fn multi_threaded_walk_dir<C: ClientState>(
    ordered_read_dir_spec: Ordered<ReadDirSpec<C>>,
    run_context: &mut RunContext<C>,
) {
    let Ordered {
        value: read_dir_spec,
        index_path,
        ..
    } = ordered_read_dir_spec;

    let read_dir_result = (run_context.core_read_dir_callback)(read_dir_spec);
    let ordered_read_children_specs = read_dir_result
        .as_ref()
        .ok()
        .map(|read_dir| read_dir.ordered_read_children_specs(&index_path));

    let ordered_read_dir_result = Ordered::new(
        read_dir_result,
        index_path,
        ordered_read_children_specs.as_ref().map_or(0, Vec::len),
    );

    if !run_context.send_read_dir_result(ordered_read_dir_result) {
        run_context.stop();
        return;
    }

    if let Some(ordered_read_children_specs) = ordered_read_children_specs {
        for each in ordered_read_children_specs {
            if !run_context.schedule_read_dir_spec(each) {
                run_context.stop();
                return;
            }
        }
    }

    run_context.complete_item();
}
