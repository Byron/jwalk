#!/usr/bin/env bash
set -eu -o pipefail

# Self-referencing symlink
ln -s a a
