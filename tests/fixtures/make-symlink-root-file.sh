#!/usr/bin/env bash
set -eu -o pipefail

# File symlink at root level
touch a
ln -s a a-link
