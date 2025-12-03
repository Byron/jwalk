#!/usr/bin/env bash
set -eu -o pipefail

# Sibling directories with files
mkdir -p foo bar
touch foo/a foo/b bar/a bar/b
