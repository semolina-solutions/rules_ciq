# Local Repositories

This directory contains local Bazel repositories that are used to bridge system dependencies and other local resources into the Bazel build environment.

## Contents

*   **`osx_homebrew`**: A local repository that exposes libraries installed via Homebrew on MacOS
(e.g., `libmtp`) to the Bazel build. This allows the project to link against system libraries that
are not easily available as pre-packaged Bazel modules.
