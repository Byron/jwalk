Time to walk linux's source tree on iMac (Retina 5K, 27-inch, Late 2015):

| Crate   | Options                        | Time      |
|---------|--------------------------------|-----------|
| jwalk   | unsorted, parallel             | 54.026 ms |
| jwalk   | sorted, parallel               | 55.988 ms |
| jwalk   | sorted, parallel, metadata     | 97.502 ms |
| jwalk   | unsorted, parallel (2 threads) | 98.869 ms |
| jwalk   | unsorted, serial               | 170.86 ms |
| jwalk   | sorted, parallel, first 100    | 9.0272 ms |
| ignore  | unsorted, parallel             | 68.594 ms |
| ignore  | sorted, parallel               | 94.374 ms |
| ignore  | sorted, parallel, metadata     | 131.50 ms |
| walkdir | unsorted                       | 162.97 ms |
| walkdir | sorted                         | 198.16 ms |
| walkdir | sorted, metadata               | 422.49 ms |