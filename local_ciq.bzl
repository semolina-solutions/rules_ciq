"""Bazel tools for machine-local Garmin Connect IQ integration.

This module provides repository rules for integrating the machine-local Garmin
Connect IQ installation with Bazel builds, including SDK provider wrappers
and device metadata configuration.
"""

load("@bazel_skylib//lib:paths.bzl", "paths")

SDK_DEFS_CONTENT = """
SdkInfo = provider(
    fields = [
        "monkeyc_path",
        "shell_path",
        "simulator_path"
    ],
)

def _sdk_provider_wrapper_impl(ctx):
    return [
        SdkInfo(
            monkeyc_path = ctx.attr.monkeyc_path,
            shell_path = ctx.attr.shell_path,
            simulator_path = ctx.attr.simulator_path,
        ),
    ]

sdk_provider_wrapper = rule(
    implementation = _sdk_provider_wrapper_impl,
    attrs = {
        "monkeyc_path": attr.string(),
        "shell_path": attr.string(),
        "simulator_path": attr.string(),
    },
)
"""

SDK_PROVIDER_WRAPPER_TEMPLATE = """
sdk_provider_wrapper(
    name = "{name}",
    monkeyc_path = "{monkeyc_path}",
    shell_path = "{shell_path}",
    simulator_path = "{simulator_path}",
    visibility = ["//visibility:public"],
)
"""

SDK_BUILD_TEMPLATE = """
load("//sdk:defs.bzl", "sdk_provider_wrapper")
load("@bazel_skylib//:bzl_library.bzl", "bzl_library")

{sdk_provider_wrappers}

bzl_library(
    name = "defs",
    srcs = ["defs.bzl"],
    visibility = ["//visibility:public"],
)
"""

DEFS_CONTENT = """
devices = {devices};
"""

BUILD_CONTENT = """
load("@bazel_skylib//:bzl_library.bzl", "bzl_library")

filegroup(
    name = "fonts",
    srcs = glob(include=["fonts/*"]),
    visibility = ["//visibility:public"],
)

bzl_library(
    name = "defs",
    srcs = ["defs.bzl"],
    visibility = ["//visibility:public"],
)
"""

def _local_ciq_impl(repository_ctx):
    if repository_ctx.os.name != "mac os x":
        fail("The local_ciq repository rule is currently only supported on macOS.")
    home = repository_ctx.os.environ.get("HOME")
    local_ciq_path = paths.join(home, "Library/Application Support/Garmin/ConnectIQ")

    current_sdk_cfg_path = paths.join(local_ciq_path, "current-sdk.cfg")
    local_current_sdk_path = repository_ctx.read(current_sdk_cfg_path)

    current_sdk_provider_wrapper = SDK_PROVIDER_WRAPPER_TEMPLATE.format(
        name = "current",
        monkeyc_path = paths.join(local_current_sdk_path, "bin", "monkeyc"),
        shell_path = paths.join(local_current_sdk_path, "bin", "shell"),
        # Rather than just calling "open /.../ConnectIQ.app", reach into the
        # package and find the executable, such that it can be terminated by
        # the runner script.
        simulator_path = paths.join(
            local_current_sdk_path,
            "bin",
            "ConnectIQ.app/Contents/MacOS/simulator",
        ),
    )

    repository_ctx.file("sdk/defs.bzl", SDK_DEFS_CONTENT)
    repository_ctx.file(
        "sdk/BUILD.bazel",
        SDK_BUILD_TEMPLATE.format(
            sdk_provider_wrappers = current_sdk_provider_wrapper,
        ),
    )

    fonts_dir_path = paths.join(local_ciq_path, "Fonts")
    symlinked_fonts_dir = "fonts"
    for path in repository_ctx.path(fonts_dir_path).readdir():
        basename = path.basename
        if not basename.endswith(".cft") and not basename.endswith(".ttf"):
            continue
        symlinked_path = repository_ctx.path(
            paths.join(symlinked_fonts_dir, basename),
        )
        repository_ctx.symlink(path, symlinked_path)

    repository_ctx.file("BUILD.bazel", BUILD_CONTENT)

    devices_dir_path = paths.join(local_ciq_path, "Devices")
    device_metadata_dict = {}
    for device_dir in repository_ctx.path(devices_dir_path).readdir():
        if not device_dir.is_dir:
            continue
        device_id = device_dir.basename
        compiler_json_path = device_dir.get_child("compiler.json")
        compiler_json_string = repository_ctx.read(compiler_json_path)
        compiler_json = json.decode(compiler_json_string)
        simulator_json_path = device_dir.get_child("simulator.json")
        simulator_json_string = repository_ctx.read(simulator_json_path)
        simulator_json = json.decode(simulator_json_string)
        device_metadata_dict[device_id] = {
            "compiler": compiler_json,
            "simulator": simulator_json,
        }

    repository_ctx.file("defs.bzl", DEFS_CONTENT.format(
        devices = str(device_metadata_dict).replace("true", "True").replace("false", "False"),
    ))

local_ciq = repository_rule(
    implementation = _local_ciq_impl,
    local = True,
)
