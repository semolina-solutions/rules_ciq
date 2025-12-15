# Garmin Connect IQ Rules For Bazel

## Overview

This repository provides [Bazel](https://bazel.build/) rules for building, simulating, and testing Garmin Connect IQ applications. It streamlines the development process by integrating the Connect IQ SDK with Bazel's powerful build system.

## Features

*   **Project Scaffolding**: Automatically generate `manifest.xml` and `jungle` files based on your build targets.
*   **Resource Scaling**: Automatically scale drawable resources for different device resolutions and icon sizes.
*   **Device Builds**: Build `.prg` application files for specific devices with type checking and optimization options.
*   **Simulation**: Launch your application in the Connect IQ Simulator directly from the command line (optionally with hot-reload).
*   **Testing**: Run unit tests in the simulator (optionally with hot-reload).
*   **Device Deployment**: Upload applications to physical Garmin devices via MTP (Media Transfer Protocol).
*   **Log Retrieval**: Fetch debug logs from physical devices.
*   **Release Packaging**: Export `.iq` files for submission to the Connect IQ Store.
*   **Multi-Device Management**: Use macros to generate build, test, and simulation targets for multiple devices at once.

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

1. Create a `.bazelrc` file (see [documentation](https://bazel.build/run/bazelrc))
containing the following reference to your CIQ developer key:

    ```
    build --action_env=CIQ_DEVELOPER_KEY_PATH=/path/to/your/developer_key.der
    ```

1. Create a `MODULE.bazel` file for your project and add the `rules_ciq` dependency:

    ```python
    bazel_dep(name = "rules_ciq", version = "0.1.0")
    ```

    If you'd prefer to be on the bleeding edge, instead use:

    ```python
    bazel_dep(name = "rules_ciq")

    git_override(
        module_name = "rules_ciq",
        remote = "https://github.com/semolina-solutions/rules_ciq",
        branch = "main",
    )
    ```

1. Create a `BUILD.bazel` file for your project and declare targets using the
rules provided by this repository. See the[`samples/hello_world`](samples/hello_world)
project for an introductory example. As a minimal demonstration, here is what to put in
the `BUILD.bazel` file to generate an app manifest for a widget file:

    ```python
    load("@rules_ciq//:defs.bzl", "ciq_manifest")

    ciq_manifest(
        name = "my_manifest",
        entry = "MyApp",
        id = "00000000-0000-0000-0000-000000000000",
        launcher_icon_drawable_resource_id = "MyLauncherIconResource",
        min_api_level = "3.0.0",
        name_string_resource_id = "MyAppNameResource",
        type = "widget",
    )
    ```
1. Build, run or test your Bazel targets using `bazel build`, `bazel run`, or
`bazel test`. To generate the manifest file from the minimal example above, in
the same directory as the `BUILD.bazel` file, run in the terminal:

    ```bash
    bazel build my_manifest
    ```

    The generated file will be written to the `bazel-bin` directory. You can
    observe that all devices that support widgets and a minimum API level
    `3.0.0` are included.


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
