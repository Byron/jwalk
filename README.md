jwalk
=======
Fast recursive directory walk.

- Walk is performed in parallel using rayon
- Results are streamed in sorted order

This crate is inspired by both [`walkdir`](https://crates.io/crates/walkdir)
and [`ignore`](https://crates.io/crates/ignore). It attempts to combine the
parallelism of `ignore` with the streaming iterator based api of `walkdir`.

# Example

Recursively iterate over the "foo" directory sorting by name:

```
use jwalk::{Sort, WalkDir};

fn main() {
  for entry in WalkDir::new("foo").sort(Some(Sort::Name)) {
    println!("{}", entry?.path().display());
  }
}
```

# Why would you use this crate?

Performance is the main reason. The following benchmarks walk linux's source
code under various conditions. You can run these benchmarks yourself using
`cargo bench`.

Note in particular that this crate is fast when you want streamed sorted
results. Also note that even when used in single thread mode this crate is
very close to `walkdir` in performance.

This crate's parallelism happens at `fs::read_dir` granularity. If you are
walking many files in a single directory it won't help. On the other hand if
you are walking a hierarchy with many folders and many files then it can
help a lot.

Also note that even though the `ignore` crate has similar performance to
this crate is has much worse latency when you want sorted results. This
crate will start streaming sorted results right away, while with `ignore`
you'll need to wait until the entire walk finishes before you can sort and
start processing the results in sorted order.

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

# Why wouldn't you use this crate?

Directory traversal is already pretty fast with existing more popular
crates. `walkdir` in particular is very good if you need a strait forward
single threaded solution.

This crate processes each `fs::read_dir` as a single unit. Reading all
entries and converting them into its own `DirEntry` representation. This
representation is fairly lightweight, but if you have an extremely wide or
deep directory structure it might cause problems holding too many
`DirEntry`s in memory at once. The concern here is memory, not open file
descriptors. This crate only keeps one open file descriptor per rayon
thread.
