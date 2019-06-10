Time to walk linux's source tree on iMac (Retina 5K, 27-inch, Late 2015):

|                    | threads  | jwalk      | ignore     | walkdir      |
|--------------------|----------|------------|------------|--------------|
| unsorted           | 8        | 54.631 ms  | 70.848 ms  | -            |
| sorted             | 8        | 56.133 ms  | 93.345 ms  | -            |
| sorted, metadata   | 8        | 86.985 ms  | 122.08 ms  | -            |
| sorted, first 100  | 8        | 8.9931 ms  | -          | -            |
| unsorted           | 2        | 88.416 ms  | 108.97 ms  | -            |
| unsorted           | 1        | 141.66 ms  | -          | 134.28 ms    |
| sorted             | 1        | 150.89 ms  | -          | 170.24 ms    |
| sorted, metadata   | 1        | 313.91 ms  | -          | 310.26 ms    |

## Notes

Comparing the performance of `jwalk`, `ignore`, and `walkdir` and how well they
can use multiple threads.

Options:

- "unsorted" means entries are returned in `read_dir` order.
- "sorted" means entries are returned sorted by name.
- "metadata" means filesystem metadata is loaded for each entry.
- "first 100" means only first 100 entries are taken.