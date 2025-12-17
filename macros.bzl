"""Macros for generating Connect IQ build, simulation, and upload targets for multiple devices."""

load("@local_ciq//:defs.bzl", "devices")
load(
    ":defs.bzl",
    "ciq_device_build",
    "ciq_device_log_cat",
    "ciq_device_upload",
    "ciq_simulation",
    "ciq_test",
)

_MODE_DEBUG = "debug"
_MODE_RELEASE = "release"

_BUILD_TEMPLATE = "{name}_{device_id}_{mode}_build"
_PROFILING_BUILD_TEMPLATE = "{name}_{device_id}_{mode}_profiling_build"
_BUILD_FOR_TESTING_TEMPLATE = "{name}_{device_id}_{mode}_build_for_testing"
_LOG_CAT_TEMPLATE = "{name}_{device_id}_{mode}_log_cat"
_SIMULATION_TEMPLATE = "{name}_{device_id}_{mode}_simulation"
_PROFILING_SIMULATION_TEMPLATE = "{name}_{device_id}_{mode}_profiling_simulation"
_UPLOAD_TEMPLATE = "{name}_{device_id}_{mode}_upload"
_PROFILING_UPLOAD_TEMPLATE = "{name}_{device_id}_{mode}_profiling_upload"
_TEST_TEMPLATE = "{name}_{device_id}_test"

def device_targets_macro(name, visibility = None, project = None, device_ids = devices.keys(), type_check_level = None):
    """Generates debug and release build, simulation, and upload targets for multiple devices.

    For example, if `name` is "my_app" and `device_ids` includes "fenix6",
    the following debug and release targets will be generated:
    - `//path/to/package:my_app_fenix6_debug_build`
    - `//path/to/package:my_app_fenix6_debug_profiling_build`
    - `//path/to/package:my_app_fenix6_debug_build_for_testing`
    - `//path/to/package:my_app_fenix6_debug_log_cat`
    - `//path/to/package:my_app_fenix6_debug_simulation`
    - `//path/to/package:my_app_fenix6_debug_upload`
    - `//path/to/package:my_app_fenix6_debug_profiling_upload`
    - `//path/to/package:my_app_fenix6_release_build`
    - `//path/to/package:my_app_fenix6_release_profiling_build`
    - `//path/to/package:my_app_fenix6_release_log_cat`
    - `//path/to/package:my_app_fenix6_release_simulation`
    - `//path/to/package:my_app_fenix6_release_profiling_simulation`
    - `//path/to/package:my_app_fenix6_release_upload`
    - `//path/to/package:my_app_fenix6_release_profiling_upload`
    - `//path/to/package:my_app_fenix6_test`

    These targets can be built or run using `bazel build` or `bazel run`.
    For example:
    - `bazel build //path/to/package:my_app_fenix6_debug_build`
    - `bazel run //path/to/package:my_app_fenix6_debug_simulation`
    - `bazel run //path/to/package:my_app_fenix6_debug_upload`
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

            # Log cat
            ciq_device_log_cat(
                name = _LOG_CAT_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
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

            # Upload
            ciq_device_upload(
                name = _UPLOAD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                device_build = _BUILD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
                visibility = visibility,
            )
            
            # Upload (profiling)
            ciq_device_upload(
                name = _PROFILING_UPLOAD_TEMPLATE.format(name = name, device_id = device_id, mode = mode),
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
