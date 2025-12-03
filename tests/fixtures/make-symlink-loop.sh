#!/usr/bin/env bash
set -eu -o pipefail

# Symlink that creates a loop
mkdir -p a/b/c
ln -s ../../../a a/b/c/a-link
