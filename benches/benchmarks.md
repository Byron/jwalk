Time to walk linux's source tree on iMac (Retina 5K, 27-inch, Late 2015):

|                                | jwalk        | ignore       | walkdir      |
|--------------------------------|--------------|--------------|--------------|
| Unsorted, parallel             | 54.026 ms    | 68.594 ms    | -            |
| unsorted, parallel (2 threads) | 98.869 ms    | -            | -            |
| sorted, parallel               | 55.988 ms    | 94.374 ms    | -            |
| sorted, parallel, metadata     | 97.502 ms    | 131.50 ms    | -            |
| unsorted, serial               | 170.86 ms    | -            | -            |
| sorted, parallel, first 100    | 9.0272 ms    | -            | -            |
| unsorted                       | -            | -            | 162.97 ms    |
| sorted                         | -            | -            | 198.16 ms    |
| sorted, metadata               | -            | -            | 422.49 ms    |
