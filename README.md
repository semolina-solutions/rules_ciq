# Garmin Connect IQ Rules For Bazel

## Overview

This repository provides [Bazel](https://bazel.build/) rules for building, simulating, and testing Garmin Connect IQ applications. It streamlines the development process by integrating the Connect IQ SDK with Bazel's powerful build system.

## Limitations

Currenly only macOS is supported.

## Getting started

1. Before using these rules, ensure you have the following installed:

    *  **Bazel**: It's recommended to use [Bazelisk](https://github.com/bazelbuild/bazelisk) to manage your Bazel version. 
    *  **Garmin Connect IQ SDK**: Install an SDK using the Connect IQ SDK Manager.
    *  **libmtp**: Required for physical device communication:
        *   macOS: Install via Homebrew: `brew install libmtp`
    * Optional:
        * **[Bazel watcher](https://github.com/bazelbuild/bazel-watcher)** aka **ibazel**
        * **[Bazel plugin for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=BazelBuild.vscode-bazel)**

1. Create a `MODULE.bazel` file for your product and add the `rules_ciq` dependency:

    ```python
    bazel_dep(name = "rules_ciq", version = "0.1.0")
    ```

1. Create a `BUILD.bazel` file for your project and declare targets using the
rules provided by this repository. See the [samples](samples) directory for
sample projects.

## Documentation

Stardoc-generated documentation is available in the [stardoc](stardoc) directory.

| File | Purpose |
| --- | --- |
| **[stardoc/defs.bzl](stardoc/defs.md)** | Documentation for rules and supporting functions in `defs.bzl` |
| **[stardoc/macros.bzl](stardoc/macros.md)** | Documentation for helper macros in `macros.bzl` |

## Commercial support

This SDK is free and open-source under the MIT license.

If you're using it in a commercial or production environment and need:
- Priority support
- Integration help
- Feature development
- Long-term maintenance assurances

you can purchase a commercial support contract.

Contact: garmin@semolina.solutions
