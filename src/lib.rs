/*! Fast recursive directory iterator.

- Walk is performed in parallel using rayon
- Results are streamed in sorted order

# Example
Recursively iterate over the "foo" directory and print each entry's path:

```no_run
use jwalk::WalkDir;
# fn main() {
for entry in WalkDir::new("foo") {
    println!("{}", entry.path().display());
}
# }
```
*/

mod walk;
mod work_tree;

pub mod core;

pub use crate::walk::*;
