"""Macros for generating Connect IQ build, simulation, and upload targets for multiple devices."""

load("@local_ciq//:defs.bzl", "devices")
load(
    "//build:defs.bzl",
    "ciq_device_build",
)
load(
    "//device:defs.bzl",
    "ciq_sideload_app",
    "ciq_view_app_log",
    "ciq_view_app_profiling",
)
load(
    "//simulator:defs.bzl",
    "ciq_simulation",
    "ciq_test",
)

_MODE_DEBUG = "debug"
_MODE_RELEASE = "release"

_BUILD_TEMPLATE = "{name}_{device_id}_{mode}_build"
_PROFILING_BUILD_TEMPLATE = "{name}_{device_id}_{mode}_profiling_build"
_BUILD_FOR_TESTING_TEMPLATE = "{name}_{device_id}_{mode}_build_for_testing"
_VIEW_APP_LOG_TEMPLATE = "{name}_{device_id}_{mode}_view_app_log"
_VIEW_APP_PROFILING_TEMPLATE = "{name}_{device_id}_{mode}_view_app_profiling"
_SIMULATION_TEMPLATE = "{name}_{device_id}_{mode}_simulation"
_PROFILING_SIMULATION_TEMPLATE = "{name}_{device_id}_{mode}_profiling_simulation"
_SIDELOAD_APP_TEMPLATE = "{name}_{device_id}_{mode}_sideload_app"
_PROFILING_SIDELOAD_APP_TEMPLATE = "{name}_{device_id}_{mode}_profiling_sideload_app"
_TEST_TEMPLATE = "{name}_{device_id}_test"

def ciq_device_targets_macro(name, visibility = None, project = None, device_ids = devices.keys(), type_check_level = None):
    """Generates debug and release build, simulation, and device interaction targets for multiple devices.

    For example, if `name` is "my_app" and `device_ids` includes "fenix6",
    the following targets will be generated:
    - `//path/to/package:my_app_fenix6_debug_build` (build)
    - `//path/to/package:my_app_fenix6_debug_build_for_testing` (build)
    - `//path/to/package:my_app_fenix6_debug_profiling_build` (build)
    - `//path/to/package:my_app_fenix6_debug_profiling_sideload_app` (run)
    - `//path/to/package:my_app_fenix6_debug_profiling_simulation` (run)
    - `//path/to/package:my_app_fenix6_debug_sideload_app` (run)
    - `//path/to/package:my_app_fenix6_debug_simulation` (run)
    - `//path/to/package:my_app_fenix6_debug_view_app_log` (run)
    - `//path/to/package:my_app_fenix6_debug_view_app_profiling` (run)
    - `//path/to/package:my_app_fenix6_release_build` (build)
    - `//path/to/package:my_app_fenix6_release_profiling_build` (build)
    - `//path/to/package:my_app_fenix6_release_profiling_sideload_app` (run)
    - `//path/to/package:my_app_fenix6_release_profiling_simulation` (run)
    - `//path/to/package:my_app_fenix6_release_sideload_app` (run)
    - `//path/to/package:my_app_fenix6_release_simulation` (run)
    - `//path/to/package:my_app_fenix6_release_view_app_log` (run)
    - `//path/to/package:my_app_fenix6_release_view_app_profiling` (run)
    - `//path/to/package:my_app_fenix6_test` (test)

    These targets can be built or run using `bazel build` or `bazel run`.
    For example:
    - `bazel build //path/to/package:my_app_fenix6_debug_build`
    - `bazel run //path/to/package:my_app_fenix6_debug_simulation`
    - `bazel run //path/to/package:my_app_fenix6_debug_sideload_app`
    - `bazel test //path/to/package:my_app_fenix6_test`

    Profiling note: The *_profiling_simulation targets ensure profiling is
    started along with the simulation, capturing startup execution. Profiling
    may otherwise be started manually when using any *_simulation target.

    Args:
        name: Base name for the generated targets.
        visibility: Visibility specification for the build targets.
        project: The Connect IQ project to build.
        device_ids: List of device IDs to generate targets for. Defaults to all available devices.
        type_check_level: Type check level for the build (0=Silent, 1=Gradual, 2=Informative, 3=Strict).
    """
    for device_id in device_ids:
        for mode in [_MODE_DEBUG, _MODE_RELEASE]:
            is_release = (mode == _MODE_RELEASE)
            
            # Build
            ciq_device_build(
                name = _BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                project = project,
                device_id = device_id,
                release = is_release,
                type_check_level = type_check_level,
                visibility = visibility,
            )
            
            # Build (profiling)
            ciq_device_build(
                name = _PROFILING_BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                project = project,
                device_id = device_id,
                release = is_release,
                profiling = True,
                type_check_level = type_check_level,
                visibility = visibility,
            )

            # View App Log
            ciq_view_app_log(
                name = _VIEW_APP_LOG_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )

            # Simulation
            ciq_simulation(
                name = _SIMULATION_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )
            
            # Simulation (where profiling starts immediately)
            ciq_simulation(
                name = _PROFILING_SIMULATION_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _PROFILING_BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )

            # Sideload App
            ciq_sideload_app(
                name = _SIDELOAD_APP_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )
            
            # Sideload App (profiling)
            ciq_sideload_app(
                name = _PROFILING_SIDELOAD_APP_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _PROFILING_BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )

            # View App Profiling (using the profiling build)
            ciq_view_app_profiling(
                name = _VIEW_APP_PROFILING_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _PROFILING_BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )

        # Testing (debug only)
        ciq_device_build(
            name = _BUILD_FOR_TESTING_TEMPLATE.format(name = name, device_id = device_id, mode = _MODE_DEBUG),
            project = project,
            device_id = device_id,
            testing = True,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_test(
            name = _TEST_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _BUILD_FOR_TESTING_TEMPLATE.format(name = name, device_id = device_id, mode = _MODE_DEBUG),
            visibility = visibility,
        )
