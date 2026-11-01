# Garmin Connect IQ Rules For Bazel

## Overview

This repository provides [Bazel](https://bazel.build/) rules for building, simulating, and testing Garmin Connect IQ applications. It streamlines the development process by integrating the Connect IQ SDK with Bazel's powerful build system.

## Features

*   **Project Scaffolding**: Automatically generate `manifest.xml` and `jungle` files based on your build targets.
*   **Resource Scaling**: Automatically scale drawable resources for different device resolutions and icon sizes.
*   **Font Generation**: Automatically generate and scale bitmap fonts from TrueType/OpenType files, ensuring they match specific screen dimensions or native SDK font sizes.
*   **Device Builds**: Build `.prg` application files for specific devices with type checking and optimization options.
*   **Simulation**: Launch your application in the Connect IQ Simulator directly from the command line (optionally with hot-reload).
*   **Testing**: Run unit tests in the simulator (optionally with hot-reload).
*   **Device Deployment**: Upload applications to physical Garmin devices via MTP (Media Transfer Protocol).
*   **Log Retrieval**: Fetch debug logs from physical devices.
*   **Performance Profiling**: Analyze device profiling logs generating statistical reports with call stack resolution.
*   **Release Packaging**: Export `.iq` files for submission to the Connect IQ Store.
*   **App Store Assets**: Generate device-framed screenshots and optimized animated GIFs for the Connect IQ Store.
*   **Multi-Device Management**: Use macros to generate build, test, and simulation targets for multiple devices at once.

## Limitations

Currenly only macOS is supported.

## Getting started

1. Before using these rules, ensure you have the following installed:

    *  **Bazel**: It's recommended to use [Bazelisk](https://github.com/bazelbuild/bazelisk) to manage your Bazel version. 
    *  **libmtp**: Required for physical device communication:
        *   macOS: Install via Homebrew: `brew install libmtp`
    * Optional:
        * **[Bazel watcher](https://github.com/bazelbuild/bazel-watcher)** aka **ibazel**
        * **[Bazel plugin for Visual Studio Code](https://marketplace.visualstudio.com/items?itemName=BazelBuild.vscode-bazel)**

1. Authenticate once using the bundled [`connect-iq-sdk-manager-cli`](https://github.com/lindell/connect-iq-sdk-manager-cli) target so `rules_ciq` can hermetically provision the Garmin Connect IQ SDKs, devices, and fonts inside your Bazel output base:

    ```bash
    bazel run @rules_ciq//:sdk_manager -- agreement view
    bazel run @rules_ciq//:sdk_manager -- agreement accept
    bazel run @rules_ciq//:sdk_manager -- login
    ```

    *(For headless / CI environments, you can instead set `CIQ_AGREEMENT_HASH`, `GARMIN_USERNAME`, and `GARMIN_PASSWORD` in your environment.)*

1. Create a `.bazelrc` file (see [documentation](https://bazel.build/run/bazelrc))
containing the following reference to your CIQ developer key:

    ```
    build --action_env=CIQ_DEVELOPER_KEY_PATH=/path/to/your/developer_key.der
    ```

1. Create a `MODULE.bazel` file for your project and add the `rules_ciq` dependency:

    ```python
    bazel_dep(name = "rules_ciq", version = "0.2.0")
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
    load(
        "@rules_ciq//:defs.bzl",
        "ciq_manifest_metadata",
        "ciq_project",
    )

    ciq_project(
        name = "my_project",
        jungles = [":sources"],
        manifest = ciq_manifest_metadata(
            entry = "MyApp",
            id = "00000000-0000-0000-0000-000000000001",
            launcher_icon_drawable_resource_id = "MyLauncherIconResource",
            min_api_level = "3.0.0",
            name_string_resource_id = "MyAppNameResource",
            type = "widget",
        ),
        sdk = "9.3.0",
    )
    ```
1. Build, run or test your Bazel targets using `bazel build`, `bazel run`, or
`bazel test`. To build the project from the minimal example above, in
the same directory as the `BUILD.bazel` file, run in the terminal:

    ```bash
    bazel build my_project
    ```

    The generated file will be written to the `bazel-bin` directory. You can
    observe that all devices that support widgets and a minimum API level
    `3.0.0` are included.

## Managing SDKs, Devices, and Fonts

All Garmin Connect IQ assets (`Sdks/`, `Devices/`, `Fonts/`, `sdkmanager-config.ini`, and `token.json`) are stored hermetically inside your workspace's Bazel output base under `$(bazel info output_base)/ciq_hermetic_home` (at `Library/Application Support/Garmin/ConnectIQ` on macOS, `.Garmin/ConnectIQ` on Linux, or `AppData/Roaming/Garmin/ConnectIQ` on Windows).

*   **Selecting an SDK version**: Projects specify their target SDK version explicitly via the `sdk` attribute on `ciq_project` (e.g. `sdk = "9.3.0"`). The targeted SDK version is downloaded automatically on first use. To list available SDK versions from Garmin:
    ```bash
    bazel run @rules_ciq//:sdk_manager -- sdk list
    ```
*   **Incremental refresh (newly released SDKs, devices, and fonts)**: To check Garmin's servers for newly added devices, fonts, and SDK releases without wiping existing downloads:
    ```bash
    bazel fetch --configure
    ```
*   **Full re-download of devices and fonts**: Because existing device directories and font files are cached on disk, if Garmin updates an existing device definition in place, remove the cached `Devices` and `Fonts` directories (keeping your login token, accepted agreement, and downloaded SDKs intact) and re-fetch:
    ```bash
    rm -rf "$(bazel info output_base)/ciq_hermetic_home/Library/Application Support/Garmin/ConnectIQ/"{Devices,Fonts}
    bazel fetch --configure
    ```

## Samples


| Sample | Purpose |
| --- | --- |
| **[samples/hello_world](samples/hello_world)** | Introductory example demonstrating project structure, manifest generation, simulating, unit testing, uploading to physical devices, and `.iq` export. |
| **[samples/i18n](samples/i18n)** | Demonstrates internationalization (i18n) and localized string resources with automatic manifest language detection. |
| **[samples/device_parametrics](samples/device_parametrics)** | Illustrates how to generate device-specific resources like icons and backgrounds. |
| **[samples/custom_font](samples/custom_font)** | Examples of generating custom bitmap fonts, including scaling relative to screen size or matching native SDK font heights. |
| **[samples/screen_ppi](samples/screen_ppi)** | Demonstrates how to scale resources based on physical screen dimensions. |
| **[samples/store](samples/store)** | Examples of generating store assets, including device-framed screenshots and animated GIFs. |

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
