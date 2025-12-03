#!/usr/bin/env bash
set -eu -o pipefail

# Basic directory structure for simple tests
# Creates: dir/a, dir/foo, dir/foo/a
# One dir with one file inside
mkdir -p foo
touch foo/a
