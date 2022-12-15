All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.8.1 (2022-12-15)

### New Features

 - <csr-id-b49e157b539150a44b761e43d8b09621367e760c/> re-export `rayon` in the crate root.
   This makes creating a `ThreadPool` easier as it doesn't force us to
   maintain our own `rayon` dependency.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - re-export `rayon` in the crate root. ([`b49e157`](https://github.com/Byron/jwalk/commit/b49e157b539150a44b761e43d8b09621367e760c))
</details>

## 0.8.0 (2022-12-15)

### New Features (BREAKING)

 - <csr-id-3a717219411a7478b90c4d694d57e28d8941dde1/> `Parallelism::RayonExistingPool::busy_timeout` is now optional.
   That way we can indicate that no waiting should be done as we know the
   given threadpool has enough resources.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release jwalk v0.8.0 ([`be0bd21`](https://github.com/Byron/jwalk/commit/be0bd21bd5213033ac55b90ec7753d6e72b4bd84))
    - `Parallelism::RayonExistingPool::busy_timeout` is now optional. ([`3a71721`](https://github.com/Byron/jwalk/commit/3a717219411a7478b90c4d694d57e28d8941dde1))
</details>

## 0.7.0 (2022-12-15)

This release makes iterator creation fallible to avoid potential hangs when there is no available thread to process
any of the iterator work.

### New Features

 - <csr-id-7d5b8b870bfca2b1b68de1427fbdc0ec1a1bff2b/> `WalkDirGeneric::try_into_iter()` for early error handling.
   If we can't instantiate the iterator due to a busy thread-pool,
   we can now abort early instead of yielding a fake-entry just to
   show an error occurred. This is the preferred way to instantiate
   a  `jwalk` iterator.

### New Features (BREAKING)

 - <csr-id-3bf1bc226571869e4a5c357d4f6e40ad0a28f3ff/> Detect possible deadlocks when instantiating a parallel iterator.
   Deadlocks can happen if the producer for results doesn't start as there
   is no free thread on the rayon pool, and the only way for it to become free
   is if the iterator produces results.
   
   We now offer a `busy_timeout` in the relevant variants of the
   `Parallelism` enumeration to allow controlling how long we will wait
   until we abort with an error.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 1 day passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release jwalk v0.7.0 ([`c265744`](https://github.com/Byron/jwalk/commit/c265744d74b3ea231eacee6332a99ee292f3018a))
    - prepare changelog prior to release ([`67364f9`](https://github.com/Byron/jwalk/commit/67364f910f1ba1ffeb5c30d0f75709431b212e2b))
    - refactor ([`a94d14b`](https://github.com/Byron/jwalk/commit/a94d14b34980e5dd53ea6dde9c5676f44c80a7fa))
    - thanks clippy ([`7e300c6`](https://github.com/Byron/jwalk/commit/7e300c68691f462ebb0848db915d7798b49dfccc))
    - `WalkDirGeneric::try_into_iter()` for early error handling. ([`7d5b8b8`](https://github.com/Byron/jwalk/commit/7d5b8b870bfca2b1b68de1427fbdc0ec1a1bff2b))
    - Detect possible deadlocks when instantiating a parallel iterator. ([`3bf1bc2`](https://github.com/Byron/jwalk/commit/3bf1bc226571869e4a5c357d4f6e40ad0a28f3ff))
    - fix various IDE warnings ([`cc0009f`](https://github.com/Byron/jwalk/commit/cc0009f626ac86f92e20ce3d4e7c2a8f00d979a0))
</details>

## 0.6.2 (2022-12-13)

### Bug Fixes

 - <csr-id-bd3e88017ea29c3b89b518f3a721ba35577b7666/> stalling issue when threadpool is used is no more.
   The issue seems to have been that `install` blocks whereas `spawn`
   properly releases the main thread.
   
   This seems to have been changed subtly due to changes in `rayon`,
   which breaks an assumption on how the code is executed.
   
   Replacing `install()` with `spawn` calls resolved the issue.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release jwalk v0.6.2 ([`2d1b2fb`](https://github.com/Byron/jwalk/commit/2d1b2fbe59a0ebb0413b54a8b8b0bba713e4d0e3))
    - Merge branch 'stalling-issue' ([`7bd2f35`](https://github.com/Byron/jwalk/commit/7bd2f35d4fd106edbafa187ef4481333bb60da7d))
    - stalling issue when threadpool is used is no more. ([`bd3e880`](https://github.com/Byron/jwalk/commit/bd3e88017ea29c3b89b518f3a721ba35577b7666))
    - refactor ([`1032308`](https://github.com/Byron/jwalk/commit/10323089dbf00e01a0280a35f826ca269b6eeea6))
    - print each path seen during iteration ([`5e83ad5`](https://github.com/Byron/jwalk/commit/5e83ad5f09852a6449f63b0c954eec81413de1c2))
</details>

## 0.6.1 (2022-12-13)

The first release under new ownership with no user-facing changes.

- The project uses GitHub CI and `cargo smart-release` for releases.
- some code cleanup based on `cargo clippy`.

### Changing project ownership to @Byron

- Thanks and good luck! (By https://github.com/jessegrosjean)
- Thank you, my pleasure (By https://github.com/Byron)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 30 commits contributed to the release over the course of 705 calendar days.
 - 705 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release jwalk v0.6.1 ([`6a2781c`](https://github.com/Byron/jwalk/commit/6a2781c6211a6db777c08bcdeb60f0317c00bc3e))
    - prepare changelog prior to release ([`c772967`](https://github.com/Byron/jwalk/commit/c77296707df392193dc47bcd1f465fa813215b82))
    - another round of link adjustments ([`7c12dc3`](https://github.com/Byron/jwalk/commit/7c12dc333d5086ed41228ee410653def9ff5adf7))
    - thanks clippy ([`51e2b0d`](https://github.com/Byron/jwalk/commit/51e2b0d0330b264972422d44ca25affcced981d5))
    - run benchmarks on CI ([`cc0fd74`](https://github.com/Byron/jwalk/commit/cc0fd74d439fb24b063d3d6d4d05f3370d41bf65))
    - set version back to what's current, change URLs and add myself as author ([`aa5b24d`](https://github.com/Byron/jwalk/commit/aa5b24dff23b4fd3b48e6a3e4c59a504a380beda))
    - cleanup tests ([`d8f0756`](https://github.com/Byron/jwalk/commit/d8f07566eb2487f6f73bd06edc8286d0bf3ee015))
    - rename 'master' to 'main' ([`362d03c`](https://github.com/Byron/jwalk/commit/362d03c1059658f88a77b7b9db7d14a2ccc2e6b5))
    - enable CI ([`a115e7a`](https://github.com/Byron/jwalk/commit/a115e7acbf6c147c2bfeba3981c6bc5d587c2aef))
    - Moved to example/crash ([`1a09da5`](https://github.com/Byron/jwalk/commit/1a09da59cda994758a856c30f931afe4ee0208d8))
    - Changing project ownership ([`cd5d1ae`](https://github.com/Byron/jwalk/commit/cd5d1aed268ad5a0692f698a419bdea835aa71a5))
    - Add crash example ([`c0b262b`](https://github.com/Byron/jwalk/commit/c0b262bc4b4bbb0134c7987cbb6995a46a005070))
    - More unneeded code removal ([`69c00ec`](https://github.com/Byron/jwalk/commit/69c00ec6f60b0010c4a99db7ea67093f401d8260))
    - Added failing combine with rayon test ([`5eccf5e`](https://github.com/Byron/jwalk/commit/5eccf5efe09c1a24e839157b6098a9bc1c8948cc))
    - Remove some unneeded rayon calls ([`f61a535`](https://github.com/Byron/jwalk/commit/f61a53585514f50f772a4cb81c57150ac1fd4450))
    - Update to rayon 1.6.1, remove jwalk_par_bridge ([`94c0385`](https://github.com/Byron/jwalk/commit/94c0385959c4004aff3ec2c60e729fa654579141))
    - Merge pull request #33 from bootandy/patch-2 ([`9575132`](https://github.com/Byron/jwalk/commit/9575132b76513bea64405a7cc5a98708d31d5743))
    - Fix typo ([`9ae51a5`](https://github.com/Byron/jwalk/commit/9ae51a5823a2c530871b2ba7fbce5096a8d37339))
    - Merge pull request #32 from Byron/master ([`5c857d4`](https://github.com/Byron/jwalk/commit/5c857d4e7fa2d587617e442d7a81e070c2c55175))
    - Don't ignore hidden files in `du` example ([`0786beb`](https://github.com/Byron/jwalk/commit/0786bebf3962e862c56577da389d9b14dfb3b5f1))
    - Remove unused imports in example ([`80a6d2e`](https://github.com/Byron/jwalk/commit/80a6d2e3054e84b36ae6c45791b5b62d579dbea7))
    - Update readme ([`6f9ebf5`](https://github.com/Byron/jwalk/commit/6f9ebf54dcfcc561c1e0afe0b797fcef8dc65b51))
    - Update to from latest rayon src/iter/par_bridge.rs ([`e3a46c1`](https://github.com/Byron/jwalk/commit/e3a46c1b02111725b7d3c7929a6b34079692f154))
    - Add simple du example ([`ae905c6`](https://github.com/Byron/jwalk/commit/ae905c60762c72aab9230717c8ae7fd6e9fcf720))
    - Rename new bench to "rayon" ([`5ee29f5`](https://github.com/Byron/jwalk/commit/5ee29f5755f7ca21c2f7d2d07b54d195c08acc72))
    - Merge pull request #31 from Byron/master ([`4817c1a`](https://github.com/Byron/jwalk/commit/4817c1a6817f659f175bce271701bfbbad5b233b))
    - reduce the recursion to its core, keep no state ([`247cf38`](https://github.com/Byron/jwalk/commit/247cf387210231ae699c5a247a024fb7724846ec))
    - Support for file metadata to approximate typical tool usage ([`a6afcfe`](https://github.com/Byron/jwalk/commit/a6afcfe535eee18fc9ce216a6facee35b58d6ed1))
    - Add simple recursive way of building a file tree for reference ([`9c66ef6`](https://github.com/Byron/jwalk/commit/9c66ef6009c47283b43b44cf08eefbdc25765737))
    - (cargo-release) start next development iteration 0.6.0 ([`a1d5209`](https://github.com/Byron/jwalk/commit/a1d5209c32723e81ed7c76e0c0ee8e3de2a07748))
</details>

## v0.6.0 (2021-01-06)

Added depth and path being read to params to ProcessReadDirFunction callback.

Allow setting initial root_read_dir_state (ReadDirState) instead of always
getting ::default() value.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Change release to 0.6 because of breaking changes ([`d74cc13`](https://github.com/Byron/jwalk/commit/d74cc130c506d5b2744bb9cfda8078ac2da0208f))
    - (cargo-release) start next development iteration 0.5.2 ([`6e4ba03`](https://github.com/Byron/jwalk/commit/6e4ba039756cccff449a317576705c4979bb8fbc))
</details>

## v0.5.2 (2020-12-28)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 289 calendar days.
 - 289 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.5.2 ([`85ef009`](https://github.com/Byron/jwalk/commit/85ef009dfe12ee23c1dacda54955281a441ec97a))
    - update dependencies ([`ea66ef8`](https://github.com/Byron/jwalk/commit/ea66ef8b8a957566d994d59048be285a23f29462))
    - Add more capability to ProcessReadDirFunction ([`2459776`](https://github.com/Byron/jwalk/commit/24597762a6bec16dcc5f421bdd0f3dee960f68cd))
    - Add test processing jwalk entries with rayon par_bridge() ([`57860ff`](https://github.com/Byron/jwalk/commit/57860ff3cfd69dc172d35666d4986055b5ba2e05))
    - cargo fmt ([`a431561`](https://github.com/Byron/jwalk/commit/a431561f11d59defcf0eed3707b46683d1f89655))
    - Merge pull request #25 from brmmm3/fix_warnings ([`fe12f26`](https://github.com/Byron/jwalk/commit/fe12f2666db66cc3e056267438b7b7b9f946f3b7))
    - Fix warnings ([`64ddb05`](https://github.com/Byron/jwalk/commit/64ddb05cbc130b865cc872d1e56c4ef71ad1a9bd))
    - Merge branch 'master' of https://github.com/jessegrosjean/walk ([`32e46c6`](https://github.com/Byron/jwalk/commit/32e46c68fb3933a2fd6fc9a329b7a72305f5eb64))
    - Note preload_metadata removal ([`3bd6618`](https://github.com/Byron/jwalk/commit/3bd6618d2a2dc689837abf97c1c9fc1f86475164))
    - Merge pull request #23 from bootandy/patch-1 ([`b4776b9`](https://github.com/Byron/jwalk/commit/b4776b9c4d1ce6f194f542d92a795653c7960408))
    - fix typo ([`d630f80`](https://github.com/Byron/jwalk/commit/d630f80335edcec2abdd6b0fb6a525565b9adda9))
    - (cargo-release) start next development iteration 0.5.1 ([`6d93359`](https://github.com/Byron/jwalk/commit/6d93359836bd7c4e1ddefa4462eb8083f45a82b1))
</details>

## v0.5.1 (2020-03-13)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.5.1 ([`1621bee`](https://github.com/Byron/jwalk/commit/1621bee2077484836558f41ec6e90be40643dcd6))
    - More tests for relative paths ([`246c997`](https://github.com/Byron/jwalk/commit/246c99701e03a671274b2a17cd3660f2388fc9c4))
    - Use path for root dir_entry.file_name if path has now filename of own ([`80f6f7a`](https://github.com/Byron/jwalk/commit/80f6f7ae7699651ec4684510eb70e4289e0ac28c))
    - Merge pull request #19 from brmmm3/simplify_and_then ([`ff234bb`](https://github.com/Byron/jwalk/commit/ff234bbbcf8999585e6c3d7e4ba8c2b44f1b8d17))
    - (cargo-release) start next development iteration 0.5.0 ([`5e97e11`](https://github.com/Byron/jwalk/commit/5e97e1159843b3ce39617a0a3ca2fea3e535e629))
</details>

## v0.5.0 (2020-03-13)

<csr-id-11fa0bc9e8541af333aafb41cc89218435474df4/>

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

### Chore

 - <csr-id-11fa0bc9e8541af333aafb41cc89218435474df4/> Update criterion to 0.3

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 294 calendar days.
 - 294 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.5.0 ([`cf9d248`](https://github.com/Byron/jwalk/commit/cf9d248d13932b4afa531f704f8f3eb8e2d55ec3))
    - Merge pull request #21 from jessegrosjean/symlinks ([`6410407`](https://github.com/Byron/jwalk/commit/641040798c71ab68fdba8874b6221f8e9d6295d7))
    - Update dependencies ([`85dff6e`](https://github.com/Byron/jwalk/commit/85dff6e02f5c52b887b429d12411f9faf1db2a5c))
    - Merge branch 'master' of https://github.com/jessegrosjean/walk into symlinks ([`0dfefa9`](https://github.com/Byron/jwalk/commit/0dfefa9e0e068a6f1902ffe0da5c99511bb88792))
    - Get follow links working and passing tests ([`797f76f`](https://github.com/Byron/jwalk/commit/797f76f8c2cfcaf8b02c0891794d9310e2bf30f6))
    - Clean ([`ff8f491`](https://github.com/Byron/jwalk/commit/ff8f491a1945ef2a148adffe9bb999b49ca2dc73))
    - Merge pull request #18 from brmmm3/remove_unnecessary_clone ([`898ff91`](https://github.com/Byron/jwalk/commit/898ff914ae0c9d6f1f49e2fbe64b21742a93b524))
    - Simplify and_then ([`b330914`](https://github.com/Byron/jwalk/commit/b3309145eee88c8b48daf1453d68e5d6d98d3ce7))
    - Remove unnecessary clone. ([`0f8b1c8`](https://github.com/Byron/jwalk/commit/0f8b1c8350c4d31e6de88fddbc7fd61ecd990065))
    - Merge pull request #12 from ignatenkobrain/patch-1 ([`f9a144b`](https://github.com/Byron/jwalk/commit/f9a144bd504027d44d0fd86c1aeef077ce3e543e))
    - Update criterion to 0.3 ([`11fa0bc`](https://github.com/Byron/jwalk/commit/11fa0bc9e8541af333aafb41cc89218435474df4))
    - Make Parallelism param actually have effect. Fix some related bugs. ([`3b90d8e`](https://github.com/Byron/jwalk/commit/3b90d8e40ba39d378077bb2f20da789af6addda0))
    - use walkdir error struct ([`850954f`](https://github.com/Byron/jwalk/commit/850954f6b3756bbca66e2bccfde9f5f9297f2bb3))
    - test/benches back to working ([`45529ef`](https://github.com/Byron/jwalk/commit/45529efd5496a4ab1694aeb5a82d8eab0f683e36))
    - in progress more closesly follow walkdir ([`66d46ac`](https://github.com/Byron/jwalk/commit/66d46acfb2a375e30d1a904e1ab17a9b5855f31e))
    - symlink work in progress ([`7ee0cb3`](https://github.com/Byron/jwalk/commit/7ee0cb3e72eb074f9081b5fd3cbc475e61fb9a43))
    - Merge branch 'master' of https://github.com/jessegrosjean/walk ([`17ddb91`](https://github.com/Byron/jwalk/commit/17ddb917a12d3e9787b6cbb2ace28758a712a95e))
    - Walk is now parameterized with client_state type ([`4e4218f`](https://github.com/Byron/jwalk/commit/4e4218f7dcd187ace10bd793515822976dd0259c))
    - Merge pull request #8 from vks/cleanup ([`1074566`](https://github.com/Byron/jwalk/commit/10745666a4c6620337d8d1c1c099ddc2a90d02c8))
    - Fix compiler warnings ([`064ee60`](https://github.com/Byron/jwalk/commit/064ee604277c94b38ea9768447a88070fcb641d8))
    - Remove Cargo.lock ([`8683484`](https://github.com/Byron/jwalk/commit/86834849ce473fd2a910caf5d890c3c6067dcac8))
    - fix table formatting ([`0ef72d1`](https://github.com/Byron/jwalk/commit/0ef72d1838e732771cafcd05ef04a70280628017))
    - Update benchmarks ([`fd9af20`](https://github.com/Byron/jwalk/commit/fd9af20e6e523449f07f3810fd8eb2987812ec13))
    - Merge pull request #5 from spacekookie/patch-1 ([`c10dbaa`](https://github.com/Byron/jwalk/commit/c10dbaaaf2130de162c163c86f6d23bb1ace506a))
    - Reformatting benchmarks table ([`99678be`](https://github.com/Byron/jwalk/commit/99678be587f1da6b869fe5afdd95c3a14b7666ef))
    - (cargo-release) start next development iteration 0.4.0 ([`29b6b1e`](https://github.com/Byron/jwalk/commit/29b6b1ecedc85a2baa01fddee3b6e75dbc230b54))
</details>

## v0.4.0 (2019-05-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 91 calendar days.
 - 91 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.4.0 ([`5d68189`](https://github.com/Byron/jwalk/commit/5d681896e37c7518c9c31690467412f49fa3a418))
    - Added content spec error reporting for root DirEntry ([`9643ce0`](https://github.com/Byron/jwalk/commit/9643ce091daa437b7dce082a25664688bb1ada90))
    - (cargo-release) start next development iteration 0.3.0 ([`956b1a5`](https://github.com/Byron/jwalk/commit/956b1a5d615a7dda34238626bc5097afc4be0d12))
</details>

## v0.3.0 (2019-02-21)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 9 calendar days.
 - 9 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.3.0 ([`09ebe45`](https://github.com/Byron/jwalk/commit/09ebe4508697551cf411664047e69220b8bd7ddb))
    - Update dependencies ([`f47607d`](https://github.com/Byron/jwalk/commit/f47607d959e497b2577d5b7ceb1b911405ba07a1))
    - Spelling ([`b8e9aea`](https://github.com/Byron/jwalk/commit/b8e9aea52d1e3e0f643b42cf17d80ac467f775bf))
    - Fix bug when max_depth was set to 0 ([`29c035e`](https://github.com/Byron/jwalk/commit/29c035e70990e9a442bb4f79735f1bd406688c03))
    - Revert "Simplify, stop tracking depth in read dir specs." ([`331a896`](https://github.com/Byron/jwalk/commit/331a8964286b121f5bfe55140d2f4d7f0d08e28f))
    - Simplify, stop tracking depth in read dir specs. ([`c1bffdd`](https://github.com/Byron/jwalk/commit/c1bffdd2bc56ddec23bac58f93c8bc52d8150015))
    - More badges! ([`e0d3e3e`](https://github.com/Byron/jwalk/commit/e0d3e3e67bcd2602433a84cb8e729f093cf38637))
    - (cargo-release) start next development iteration 0.2.1 ([`133d168`](https://github.com/Byron/jwalk/commit/133d168a4586313a4e5eceb2ea70c348f387a5b4))
</details>

## v0.2.1 (2019-02-11)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - (cargo-release) version 0.2.1 ([`1dee5e1`](https://github.com/Byron/jwalk/commit/1dee5e12ec7f07d2d177e116d3eceac0ec5e2bbf))
    - Fix usage documentation. Other documentation updates. ([`2631dbd`](https://github.com/Byron/jwalk/commit/2631dbd01c92eaa50ca877c819151ef129fcb6ef))
    - Update usage ([`04f897f`](https://github.com/Byron/jwalk/commit/04f897f2be26b576c06af2ecb327ecb4c8ddfcda))
    - (cargo-release) start next development iteration 0.2.0 ([`da74631`](https://github.com/Byron/jwalk/commit/da746315a12af5a9bbe4ece4897303f938d7d469))
</details>

## v0.2.0 (2019-02-11)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 37 commits contributed to the release over the course of 13 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Expose DirEntry fields for easy destructure. ([`ad9404e`](https://github.com/Byron/jwalk/commit/ad9404e19e78b2714b2db3da4ab85a91a8dbc481))
    - Clippy suggestions and more doc updates ([`8fd5ae1`](https://github.com/Byron/jwalk/commit/8fd5ae172783d0e2ce3a696540e12d2576e4db9c))
    - Rename "children" to content. Simplify sort to boolean. ([`f432c4f`](https://github.com/Byron/jwalk/commit/f432c4f806b01cfcaa16f4573a1d89d6817ab0ca))
    - More docs for ReadDirSpec ([`347bc92`](https://github.com/Byron/jwalk/commit/347bc92cec8181b6a25cc80a663345cd3d87a3bb))
    - More docs on how DirEntry is implemented ([`0bb13ee`](https://github.com/Byron/jwalk/commit/0bb13eecfa2f3f5b6448b5a7c4701add75d857cf))
    - Fixing too many keywords! ([`ff5fba1`](https://github.com/Byron/jwalk/commit/ff5fba1ebe53d0240d64fb6f5e16e0dc1a11d2f8))
    - More readme tweaks ([`0b23848`](https://github.com/Byron/jwalk/commit/0b238487c71a600dad31f05d57ca7f49d337d40d))
    - Shorter readme page. ([`0ee3b3c`](https://github.com/Byron/jwalk/commit/0ee3b3c58af31b95099d79446dc67de88967115c))
    - More doc updates ([`b3ac65c`](https://github.com/Byron/jwalk/commit/b3ac65cdca26fc1ca9e936d35256df517025fbca))
    - Remove unused dependency ([`301e2a5`](https://github.com/Byron/jwalk/commit/301e2a5c053675dee50960dbea319ebcea901f28))
    - Add badge ([`ab07f7a`](https://github.com/Byron/jwalk/commit/ab07f7ae7890a4fc18f509d9bea30f6a73e78f78))
    - Create .travis.yml ([`66828d7`](https://github.com/Byron/jwalk/commit/66828d74f5cb3119bda9d99a7993ad8373bb5166))
    - Add usage ([`58c7707`](https://github.com/Byron/jwalk/commit/58c770758cfd9f2076893b463490ef2e94dd3659))
    - split code up into more files ([`566da8f`](https://github.com/Byron/jwalk/commit/566da8f0ac14f470dc181471e3c98a940ea96641))
    - More docs cleanup ([`ade977d`](https://github.com/Byron/jwalk/commit/ade977dc29c59850b8ceb6f00089080c8e3cee18))
    - Fix readme headings ([`738d9f6`](https://github.com/Byron/jwalk/commit/738d9f616005f22285985134258e49b800ff92b6))
    - Fix README example ([`1f42a3e`](https://github.com/Byron/jwalk/commit/1f42a3eee18e602d65d420a04a8aace5cb87a20a))
    - tests and cleanup ([`5fc2859`](https://github.com/Byron/jwalk/commit/5fc2859b730992aeb8cd74ae7398393ea61e2ff6))
    - Fix DirEntry depth ([`5cc9db7`](https://github.com/Byron/jwalk/commit/5cc9db710664d51948f29fbad42e4a1be19b7390))
    - Much cleaner, ready for real testing now. ([`2e58da7`](https://github.com/Byron/jwalk/commit/2e58da7524859f571b0206d480a471b17c7e7e75))
    - Preping to box instead of template client function. ([`5cd8e04`](https://github.com/Byron/jwalk/commit/5cd8e04b8cd0cf2bdd8ce8a329a081316b5d06af))
    - Add more walk options and tests ([`c6f8385`](https://github.com/Byron/jwalk/commit/c6f8385f4d43a66d2c1b2a4bfc29fec1fcde42c1))
    - Add hidden file for tests ([`6372a1a`](https://github.com/Byron/jwalk/commit/6372a1aebbcd49aead0833e18c874f0609d09a63))
    - Add fts to bench ([`32a527d`](https://github.com/Byron/jwalk/commit/32a527d4e39d853234396289b7cb7e1d3c479232))
    - Cleanup ([`d3899aa`](https://github.com/Byron/jwalk/commit/d3899aacebfde722a1a37b4ecbecb81ce701df8e))
    - Always return ReadDirResutls, no longer an option ([`32d6b86`](https://github.com/Byron/jwalk/commit/32d6b865ea6b967bb8a7e66710c82039bcf25b80))
    - Merge pull request #2 from jessegrosjean/ordered ([`5b4745e`](https://github.com/Byron/jwalk/commit/5b4745e47937fdcccc413f522dfea1a47125b483))
    - Merge branch 'master' into ordered ([`3ecc5b6`](https://github.com/Byron/jwalk/commit/3ecc5b62bd86430f5ee4380f113288bdfc7df346))
    - Ready for feedback? ([`5d9ee63`](https://github.com/Byron/jwalk/commit/5d9ee635e30697cf1beab1b224b918cbff214ec9))
    - more work on work_tree ([`4ced894`](https://github.com/Byron/jwalk/commit/4ced894c71e82a96f4f3c644fe7ec1dd51ad124f))
    - working on more generic "work tree" ([`ebf9789`](https://github.com/Byron/jwalk/commit/ebf9789a8799b5e1c9d12d52f303fcb6ca59e0b9))
    - Merge pull request #1 from jessegrosjean/ordered ([`36b2d2f`](https://github.com/Byron/jwalk/commit/36b2d2fba8ff5b224f961b9bcbe486e196a90943))
    - Cleaned up ([`6b0df4a`](https://github.com/Byron/jwalk/commit/6b0df4aa401541481da9a86d40bb574ef95b9ca6))
    - parameterized walk ([`115a5fe`](https://github.com/Byron/jwalk/commit/115a5fefb83968e283ae48bd68c599547735fae6))
    - In progress ordered walk ([`ba02dda`](https://github.com/Byron/jwalk/commit/ba02ddaa4cf48b3e2546ae0a2ca4859f8151df50))
    - Adding ordered version ([`b63f810`](https://github.com/Byron/jwalk/commit/b63f810eb7ca3ad3b93ea4746ea99a8ce4b16ea6))
    - init commit ([`8f64d5a`](https://github.com/Byron/jwalk/commit/8f64d5ae22a3221058f50c17bc97b3c90afac3e0))
</details>

