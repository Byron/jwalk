#!/usr/bin/env bash
set -eu -o pipefail

# Directory symlink at root level
mkdir -p a
touch a/zzz
ln -s a a-link
