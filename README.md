jwalk
=======
Fast recursive directory walk.

- Walk is performed in parallel using rayon
- Results are streamed in sorted order
- Custom sort/filter/skip if needed

This crate is inspired by both [`walkdir`](https://crates.io/crates/walkdir)
and [`ignore`](https://crates.io/crates/ignore). It attempts to combine the
parallelism of `ignore` with `walkdir`s streaming iterator API.

### Example

Recursively iterate over the "foo" directory sorting by name:

```no_run
# use std::io::Error;
use jwalk::{Sort, WalkDir};
# fn try_main() -> Result<(), Error> {
for entry in WalkDir::new("foo").sort(Some(Sort::Name)) {
  println!("{}", entry?.path().display());
}
# Ok(())
# }
```

### Why use this crate?

Speed and flexibility.

This crate is particularly fast when you want streamed sorted results. In
this case it's much faster then `walkdir` and has much better latency then
`ignore`.

This crate's `process_entries` callback allows you to arbitrarily
sort/filter/skip each directories entries before they are yielded. This
processing happens in the thread pool and effects the directory traversal.
It can be much faster then post processing the yielded entries.

### Why not use this crate?

Directory traversal is already pretty fast. If you don't need this crate's
speed then `walkdir` provides a smaller single threaded implementation.

### Benchmarks

Time to walk linux's source code:

| Crate   | Options                        | Time      |
|---------|--------------------------------|-----------|
| jwalk   | unsorted, parallel             | 60.811 ms |
| jwalk   | sorted, parallel               | 61.445 ms |
| jwalk   | sorted, parallel, metadata     | 100.95 ms |
| jwalk   | unsorted, parallel (2 threads) | 99.998 ms |
| jwalk   | unsorted, serial               | 168.68 ms |
| jwalk   | sorted, parallel, first 100    | 9.9794 ms |
| ignore  | unsorted, parallel             | 74.251 ms |
| ignore  | sorted, parallel               | 99.336 ms |
| ignore  | sorted, parallel, metadata     | 134.26 ms |
| walkdir | unsorted                       | 162.09 ms |
| walkdir | sorted                         | 200.09 ms |
| walkdir | sorted, metadata               | 422.74 ms |