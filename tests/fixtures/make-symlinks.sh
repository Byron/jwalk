#!/usr/bin/env bash
set -eu -o pipefail

# File and directory symlinks
mkdir -p a
touch a_file
ln -s a_file a_file_link
ln -s a a_dir_link
touch a/zzz
