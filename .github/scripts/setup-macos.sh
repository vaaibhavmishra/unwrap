#!/usr/bin/env bash
set -euo pipefail

export HOMEBREW_NO_AUTO_UPDATE=1
brew install cmake pkgconf protobuf python@3.13
printf '%s/libexec/bin\n' "$(brew --prefix python@3.13)" >> "$GITHUB_PATH"
rustup show active-toolchain
rustup component add rustfmt clippy
xcodebuild -downloadComponent MetalToolchain
xcrun --find metal

git lfs fsck
if rg -l '^version https://git-lfs.github.com/spec/v1$' crates/input_classifier/models; then
  echo 'Git LFS model files were not downloaded.' >&2
  exit 1
fi
