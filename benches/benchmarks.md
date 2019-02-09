Time to walk linux's source tree:

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