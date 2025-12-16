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

_DEBUG_BUILD_TEMPLATE = "{name}_{device_id}_debug_build"
_DEBUG_BUILD_FOR_PROFILING_TEMPLATE = "{name}_{device_id}_debug_build_for_profiling"
_DEBUG_BUILD_FOR_TESTING_TEMPLATE = "{name}_{device_id}_debug_build_for_testing"
_DEBUG_LOG_CAT_TEMPLATE = "{name}_{device_id}_debug_log_cat"
_DEBUG_SIMULATION_TEMPLATE = "{name}_{device_id}_debug_simulation"
_DEBUG_UPLOAD_TEMPLATE = "{name}_{device_id}_debug_upload"
_DEBUG_UPLOAD_FOR_PROFILING_TEMPLATE = "{name}_{device_id}_debug_upload_for_profiling"
_RELEASE_BUILD_TEMPLATE = "{name}_{device_id}_release_build"
_RELEASE_BUILD_FOR_PROFILING_TEMPLATE = "{name}_{device_id}_release_build_for_profiling"
_RELEASE_LOG_CAT_TEMPLATE = "{name}_{device_id}_release_log_cat"
_RELEASE_SIMULATION_TEMPLATE = "{name}_{device_id}_release_simulation"
_RELEASE_UPLOAD_TEMPLATE = "{name}_{device_id}_release_upload"
_RELEASE_UPLOAD_FOR_PROFILING_TEMPLATE = "{name}_{device_id}_release_upload_for_profiling"
_TEST_TEMPLATE = "{name}_{device_id}_test"

def device_targets_macro(name, visibility = None, project = None, device_ids = devices.keys(), type_check_level = None):
    """Generates debug and release build, simulation, and upload targets for multiple devices.

    For example, if `name` is "my_app" and `device_ids` includes "fenix6",
    the following debug and release targets will be generated:
    - `//path/to/package:my_app_fenix6_debug_build`
    - `//path/to/package:my_app_fenix6_debug_build_for_profiling`
    - `//path/to/package:my_app_fenix6_debug_build_for_testing`
    - `//path/to/package:my_app_fenix6_debug_log_cat`
    - `//path/to/package:my_app_fenix6_debug_simulation`
    - `//path/to/package:my_app_fenix6_debug_upload`
    - `//path/to/package:my_app_fenix6_debug_upload_for_profiling`
    - `//path/to/package:my_app_fenix6_release_build`
    - `//path/to/package:my_app_fenix6_release_build_for_profiling`
    - `//path/to/package:my_app_fenix6_release_log_cat`
    - `//path/to/package:my_app_fenix6_release_simulation`
    - `//path/to/package:my_app_fenix6_release_upload`
    - `//path/to/package:my_app_fenix6_release_upload_for_profiling`
    - `//path/to/package:my_app_fenix6_test`

    These targets can be built or run using `bazel build` or `bazel run`.
    For example:
    - `bazel build //path/to/package:my_app_fenix6_debug_build`
    - `bazel run //path/to/package:my_app_fenix6_debug_simulation`
    - `bazel run //path/to/package:my_app_fenix6_debug_upload`
    - `bazel test //path/to/package:my_app_fenix6_test`

    Args:
        name: Base name for the generated targets.
        visibility: Visibility specification for the build targets.
        project: The Connect IQ project to build.
        device_ids: List of device IDs to generate targets for. Defaults to all available devices.
        type_check_level: Type check level for the build (0=Silent, 1=Gradual, 2=Informative, 3=Strict).
    """
    for device_id in device_ids:
        ciq_device_build(
            name = _DEBUG_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            project = project,
            device_id = device_id,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_device_build(
            name = _DEBUG_BUILD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            project = project,
            device_id = device_id,
            profiling = True,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_device_build(
            name = _DEBUG_BUILD_FOR_TESTING_TEMPLATE.format(name = name, device_id = device_id),
            project = project,
            device_id = device_id,
            testing = True,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_device_log_cat(
            name = _DEBUG_LOG_CAT_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _DEBUG_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_simulation(
            name = _DEBUG_SIMULATION_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _DEBUG_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_device_upload(
            name = _DEBUG_UPLOAD_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _DEBUG_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_device_upload(
            name = _DEBUG_UPLOAD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _DEBUG_BUILD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_device_build(
            name = _RELEASE_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            project = project,
            device_id = device_id,
            release = True,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_device_build(
            name = _RELEASE_BUILD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            project = project,
            device_id = device_id,
            release = True,
            profiling = True,
            type_check_level = type_check_level,
            visibility = visibility,
        )
        ciq_device_log_cat(
            name = _RELEASE_LOG_CAT_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _RELEASE_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_simulation(
            name = _RELEASE_SIMULATION_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _RELEASE_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_device_upload(
            name = _RELEASE_UPLOAD_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _RELEASE_BUILD_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_device_upload(
            name = _RELEASE_UPLOAD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _RELEASE_BUILD_FOR_PROFILING_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
        ciq_test(
            name = _TEST_TEMPLATE.format(name = name, device_id = device_id),
            device_build = _DEBUG_BUILD_FOR_TESTING_TEMPLATE.format(name = name, device_id = device_id),
            visibility = visibility,
        )
