jwalk
=======

Fast recursive directory walk.

- Performed in parallel using rayon
- Results are streamed in sorted order
- Custom sort/filter/skip

[![Build Status](https://travis-ci.org/jessegrosjean/jwalk.svg?branch=master)](https://travis-ci.org/jessegrosjean/jwalk)
[![Latest version](http://meritbadge.herokuapp.com/jwalk)](https://crates.io/crates/jwalk)
[![License](https://img.shields.io/crates/l/jwalk.svg)](https://github.com/rust-lang-nursery/jwalk.rs#license)

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
jwalk = "0.1.0"
```

Lean More: [docs.rs/jwalk](https://docs.rs/jwalk)

### Example

Recursively iterate over the "foo" directory sorting by name:

```rust
use jwalk::{Sort, WalkDir};

for entry in WalkDir::new("foo").sort(Some(Sort::Name)) {
  println!("{}", entry?.path().display());
}
```

### Inspiration

This crate is inspired by both [`walkdir`](https://crates.io/crates/walkdir) and
[`ignore`](https://crates.io/crates/ignore). It attempts to combine the
parallelism of `ignore` with `walkdir`'s streaming iterator API.

### Why use this crate?

This crate is particularly fast when you want streamed sorted results. In my
tests its about 4x `walkdir` speed for sorted results with metadata. Also this
crate's `process_entries` callback allows you to arbitrarily sort/filter/skip
entries before they are yielded.

### Why not use this crate?

Directory traversal is already pretty fast. If you don't need this crate's speed
then `walkdir` provides a smaller and more tested single threaded
implementation.

### Benchmarks

[Benchmarks](https://github.com/jessegrosjean/jwalk/blob/master/benches/benchmarks.md)
comparing this crate with `jwalk` and `ignore`.