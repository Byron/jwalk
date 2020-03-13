# 0.5 Breaking changes

First major change is that API and behavior are now closer to [`walkdir`] and
jwalk now runs the majority of `walkdir`s tests.

Second major change is the walk can now be parameterized with a client state
type. This state can be manipulated from the `process_read_dir` callback and
then is passed down when reading descendens with the `process_read_dir`
callback.