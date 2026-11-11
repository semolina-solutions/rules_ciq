# Garmin Connect IQ Rules For Bazel

## Overview

This repository provides [Bazel](https://bazel.build/) rules for building, simulating, and testing Garmin Connect IQ applications. It streamlines the development process by integrating the Connect IQ SDK with Bazel's powerful build system.

## Features

*   **Project Scaffolding**: Automatically generate `manifest.xml` and `jungle` files based on your build targets.
*   **Resource Scaling**: Automatically scale drawable resources for different device resolutions and icon sizes.
*   **Font Generation**: Automatically generate and scale bitmap fonts from TrueType/OpenType files, ensuring they match specific screen dimensions or native SDK font sizes.
*   **Device Builds**: Build `.prg` application files for specific devices with type checking and optimization options.
*   **Simulation**: Launch your application in the Connect IQ Simulator directly from the command line (optionally with hot-reload).
*   **Headless Simulator Automation**: Run applications in a containerized Connect IQ Simulator (`ciq_automation`) to script button presses, touchscreen gestures, `.SET` settings overrides, and `.fit` activity playback, and verify behavior via log assertions (`ciq_log_test`) and golden screenshot comparisons (`ciq_screenshot_test`).
*   **Testing**: Run unit tests in the simulator (optionally with hot-reload).
*   **Device Deployment**: Upload applications to physical Garmin devices via MTP (Media Transfer Protocol).
*   **Log Retrieval**: Fetch debug logs from physical devices.
*   **Performance Profiling**: Analyze device profiling logs generating statistical reports with call stack resolution.
*   **Release Packaging**: Export `.iq` files for submission to the Connect IQ Store.
*   **App Store Assets**: Generate device-framed screenshots and optimized animated GIFs for the Connect IQ Store.
*   **Multi-Device Management**: Use macros to generate build, test, and simulation targets for multiple devices at once.

## Getting started

1. Before using these rules, ensure you have the following installed:

    *  **Bazel**: It's recommended to use [Bazelisk](https://github.com/bazelbuild/bazelisk) to manage your Bazel version. 
    *  **libmtp** (macOS & Linux): Required for physical device communication (Windows uses the built-in Windows Portable Devices API):
        *   macOS: Install via Homebrew: `brew install libmtp`
        *   Linux: Install via apt: `sudo apt-get install libmtp-dev`
    * Optional:
        * **[Docker](https://www.docker.com/)** (or a compatible runtime such as [Colima](https://github.com/abiosoft/colima), [OrbStack](https://orbstack.dev/), or [Rancher Desktop](https://rancherdesktop.io/)): Required for headless simulator automation and screenshotting (`ciq_automation`, `ciq_log_test`, `ciq_screenshot_test`). See [Headless simulator automation](#headless-simulator-automation).
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

## Headless simulator automation

The `ciq_automation` rule runs a `ciq_device_build` inside a containerized Linux Connect IQ Simulator (running under Xvfb), drives it with scripted input, and produces two build outputs:

*   `<name>.png`: a screenshot of the device display at the end of the session.
*   `<name>.log`: the application's stdout (e.g. `System.println` output) captured during the session.

Because these are regular Bazel build outputs, they are cached, can be generated for many devices in parallel, and can be fed into downstream rules: assertions (`ciq_log_test`, `ciq_screenshot_test`) or store asset generation (`ciq_field_screenshot`, `ciq_framed_screenshot`). This makes it possible to produce up-to-date store screenshots and run UI regression tests from the command line or in CI, without opening the simulator GUI.

### How it works

*   The container image is built on demand from [`simulator/Dockerfile`](simulator/Dockerfile) (Ubuntu, `linux/amd64`) and cached by content hash, so it is only rebuilt when the Dockerfile or automation hook changes.
*   The Linux build of the Connect IQ SDK is provisioned hermetically alongside your other SDKs, and your hermetic `Devices/` and `Fonts/` directories are mounted read-only into the container. Your host OS does not need to be Linux.
*   A small GTK hook injected into the simulator replays the requested menu selections, button presses, touch gestures, and dialog interactions, then saves the display to PNG.
*   The rules locate the `docker` binary and daemon socket automatically, including Homebrew installs and Colima, OrbStack, Docker Desktop, and Rancher Desktop sockets. Set `DOCKER_HOST` to override.

> **Note**: The container runs as `linux/amd64`. On Apple Silicon Macs this needs x86_64 emulation (e.g. Docker Desktop with Rosetta, or `colima start --arch x86_64`). The Docker VM must also be able to mount your Bazel output base and temp directory (e.g. with Colima: `--mount /Users:w --mount /private/var:w`).

### Example

```python
load(
    "@rules_ciq//:defs.bzl",
    "ciq_automation",
    "ciq_device_build",
    "ciq_framed_screenshot",
    "ciq_log_test",
    "ciq_screenshot_test",
)

ciq_device_build(
    name = "my_app_fenix6_debug_build",
    device_id = "fenix6",
    project = ":my_project",
)

# Produces :my_app_menu.png and :my_app_menu.log.
ciq_automation(
    name = "my_app_menu",
    device_build = ":my_app_fenix6_debug_build",
    language = "deu",
    time_display = "24 Hour",
    units_display = "Metric",
    settings = {
        "theme": "string:dark",
        "targetHr": "int:165",
    },
    wait_for_log = "app ready",  # Wait for this before sending input.
    actions = [
        "press:down",
        "wait:300",
        "press:enter",
        "hold:menu",
    ],
)

# Assert on the captured log output.
ciq_log_test(
    name = "my_app_menu_log_test",
    log = ":my_app_menu.log",
    expect_logs = ["page=2"],
    forbid_logs = ["ERROR"],
)

# Compare the screenshot against a checked-in golden image.
ciq_screenshot_test(
    name = "my_app_menu_screenshot_test",
    expected = "golden/my_app_menu.png",
    screenshot = ":my_app_menu.png",
)

# Put the screenshot into a device frame for the Connect IQ Store.
ciq_framed_screenshot(
    name = "my_app_menu_framed",
    device_id = "fenix6",
    screenshots = [":my_app_menu.png"],
)
```

```bash
bazel build //:my_app_menu        # Generate the screenshot and log
bazel test //:my_app_menu_log_test //:my_app_menu_screenshot_test
```

To create or refresh a golden image, build the `ciq_automation` target and copy `bazel-bin/<package>/<name>.png` into your source tree.

### Automation options

| Attribute | Description |
| --- | --- |
| `actions` | Ordered input sequence. Supported: `press:<button>`, `hold:<button>` (long press), `tap:<x>,<y>`, `touch_hold:<x>,<y>`, `swipe:<left\|right\|up\|down>`, `swipe:<x1>,<y1>,<x2>,<y2>` / `drag:...`, `wait:<ms>`, and `menu:<simulator menu path>`. Button names come from the device's `simulator.json` (e.g. `up`, `down`, `enter`/`select`/`start`, `esc`/`back`, `menu`). Coordinates are display pixels or percentages (e.g. `tap:50%,50%`). |
| `settings` | Application settings/properties written to a `.SET` file before launch, as `{"key": "type:value"}` with `int`, `long`, `float`, `double`, `bool`, or `string` types. |
| `language`, `time_display`, `units_display` | Simulator locale and display settings (e.g. `"deu"`, `"24 Hour"`, `"Metric"`). |
| `layout`, `background_color` | Data field layout (e.g. `"3 Fields A"`) and background (`"White"` / `"Black"`). |
| `fit_file`, `simulate_activity` | Play back a `.fit`/`.gpx` activity file for deterministic sensor data, or use simulated activity data. |
| `timer_actions` | Activity timer controls for data fields: `start`, `pause`, `resume`, `lap`, `workout_step`, `next_multisport`, `save`, `discard`. |
| `glance_mode` | Launch glance-capable apps in glance view instead of the full app. |
| `wait_for_log` | Substring or regex to wait for in the app log before sending input. |
| `delay_ms`, `timeout_secs` | Settle time before the final screenshot (default 1500 ms), and the overall session timeout (default 120 s). |

`ciq_log_test` checks `expect_logs` / `forbid_logs` substrings or regexes against a log. `ciq_screenshot_test` compares pixels exactly by default; use `pixel_tolerance` (per-channel difference) and `max_diff_pixels` to allow small differences.

See [`samples/simulator`](samples/simulator) for complete examples, and the [Stardoc reference](stardoc/defs.md) for every attribute.

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
| **[samples/simulator](samples/simulator)** | Demonstrates headless simulator automation (`ciq_automation`), custom `.SET` settings, `.fit` playback, button/touch gestures, log assertions (`ciq_log_test`), and golden screenshot diffing (`ciq_screenshot_test`). |
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
