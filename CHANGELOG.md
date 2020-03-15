# 0.5 Breaking changes

First major change is that API and behavior are now closer to [`walkdir`] and
jwalk now runs the majority of `walkdir`s tests.

Second major change is the walk can now be parameterized with a client state
type. This state can be manipulated from the `process_read_dir` callback and
then is passed down when reading descendens with the `process_read_dir`
callback.

Part of this second change is that `preload_metadata` option is removed. That
means `DirEntry.metadata()` is never a cached value. Instead you want to read
metadata you should do it in the `process_entries` callback and store whatever
values you need as `client_state`. See this [benchmark] as an example.

[benchmark]: https://github.com/jessegrosjean/jwalk/blob/master/benches/walk_benchmark.rs#L45