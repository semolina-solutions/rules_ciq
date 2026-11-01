"""Bazel repository rules for hermetic Garmin Connect IQ provisioning via connect-iq-sdk-manager-cli."""

load("@bazel_skylib//lib:paths.bzl", "paths")

_SETUP_INSTRUCTIONS = """
To accept the Garmin Connect IQ license agreement and log in interactively, run:
  bazel run @rules_ciq//:sdk_manager -- agreement view
  bazel run @rules_ciq//:sdk_manager -- agreement accept
  bazel run @rules_ciq//:sdk_manager -- login

For non-interactive / CI environments, set CIQ_AGREEMENT_HASH (or `agreement_hash`), GARMIN_USERNAME, and GARMIN_PASSWORD.
"""

_SDK_MANAGER_WRAPPER_TEMPLATE = """#!/usr/bin/env bash
export PATH="${{PATH:-/usr/bin:/bin:/usr/sbin:/sbin}}"
export HOME="${{CIQ_HERMETIC_HOME:-{sdk_manager_home}}}"
export USERPROFILE="$HOME"
export APPDATA="$HOME/AppData/Roaming"
exec "{sdk_manager_bin}" "$@"
"""

_SDK_MANAGER_BAT_WRAPPER_TEMPLATE = """@echo off
if "%CIQ_HERMETIC_HOME%"=="" (
  set "HOME={sdk_manager_home}"
) else (
  set "HOME=%CIQ_HERMETIC_HOME%"
)
set "USERPROFILE=%HOME%"
set "APPDATA=%HOME%\\AppData\\Roaming"
"{sdk_manager_bin}" %*
"""

_SDK_MANAGER_BIN_DEFS = """
def _prebuilt_binary_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ctx.attr.ext)
    ctx.actions.symlink(
        output = out,
        target_file = ctx.file.src,
        is_executable = True,
    )
    return [DefaultInfo(executable = out)]

prebuilt_binary = rule(
    implementation = _prebuilt_binary_impl,
    attrs = {
        "ext": attr.string(default = ""),
        "src": attr.label(allow_single_file = True, mandatory = True),
    },
    executable = True,
)
"""

_SDK_MANAGER_BUILD_TEMPLATE = """
load(":defs.bzl", "prebuilt_binary")

exports_files([
    "{wrapper_name}",
    "connect-iq-sdk-manager-bin.exe",
])

prebuilt_binary(
    name = "sdk_manager",
    ext = "{wrapper_ext}",
    src = "{wrapper_name}",
    visibility = ["//visibility:public"],
)
"""

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
sdk_versions = {sdk_versions};
"""

BUILD_CONTENT = """
load("@bazel_skylib//:bzl_library.bzl", "bzl_library")

filegroup(
    name = "fonts",
    srcs = glob(include=["fonts/*"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "devices",
    srcs = glob(["devices/**/*"]),
    visibility = ["//visibility:public"],
)

bzl_library(
    name = "defs",
    srcs = ["defs.bzl"],
    visibility = ["//visibility:public"],
)

exports_files(["devices.json"])
"""

_ENSURE_SDK_SNIPPET = """if [ ! -f "{sdk_dir}/.complete" ] && [ -n "{sdk_manager_bin}" ]; then
  lockdir="{hermetic_home}/.sdk_download_{version}.lock"
  while ! mkdir "$lockdir" 2>/dev/null; do
    sleep 1
  done
  trap 'rmdir "$lockdir" 2>/dev/null || true' EXIT
  if [ ! -f "{sdk_dir}/.complete" ]; then
    if [ ! -f "{sdk_dir}/bin/monkeybrains.jar" ]; then
      rm -rf "{sdk_dir}"
      "{sdk_manager_bin}" sdk download "{version}" >&2
    fi
    touch "{sdk_dir}/.complete"
  fi
  rmdir "$lockdir" 2>/dev/null || true
  trap - EXIT
fi"""

_MONKEYC_WRAPPER_TEMPLATE = """#!/bin/bash
set -euo pipefail
export PATH="${{PATH:+$PATH:}}/usr/bin:/bin:/usr/sbin:/sbin"
export MSYS2_ARG_CONV_EXCL="*"
export HOME="{hermetic_home}"
export USERPROFILE="{hermetic_home}"
export CFFIXED_USER_HOME="{hermetic_home}"
export APPDATA="{hermetic_home}/AppData/Roaming"
export JAVA_TOOL_OPTIONS="${{JAVA_TOOL_OPTIONS:+$JAVA_TOOL_OPTIONS }}-Duser.home={hermetic_home}"
""" + _ENSURE_SDK_SNIPPET + """
exec "{java_bin}" -Xms1g -Dfile.encoding=UTF-8 -Dapple.awt.UIElement=true -classpath "{sdk_dir}/bin/monkeybrains.jar" com.garmin.monkeybrains.Monkeybrains "$@"
"""

_SHELL_WRAPPER_TEMPLATE = """#!/bin/bash
set -euo pipefail
export PATH="${{PATH:+$PATH:}}/usr/bin:/bin:/usr/sbin:/sbin"
export MSYS2_ARG_CONV_EXCL="*"
export HOME="{hermetic_home}"
export USERPROFILE="{hermetic_home}"
export CFFIXED_USER_HOME="{hermetic_home}"
export APPDATA="{hermetic_home}/AppData/Roaming"
""" + _ENSURE_SDK_SNIPPET + """
exec "{shell_path}" "$@"
"""

_SIMULATOR_WRAPPER_TEMPLATE = """#!/bin/bash
set -euo pipefail
export PATH="${{PATH:+$PATH:}}/usr/bin:/bin:/usr/sbin:/sbin"
export MSYS2_ARG_CONV_EXCL="*"
export HOME="{hermetic_home}"
export USERPROFILE="{hermetic_home}"
export CFFIXED_USER_HOME="{hermetic_home}"
export APPDATA="{hermetic_home}/AppData/Roaming"
""" + _ENSURE_SDK_SNIPPET + """
exec "{simulator_path}" "$@"
"""

def _connect_iq_sdk_manager_impl(repository_ctx):
    is_windows = "windows" in repository_ctx.os.name.lower()
    go_bin = repository_ctx.path(repository_ctx.attr._go_bin)
    raw_bin = repository_ctx.path("connect-iq-sdk-manager-bin.exe")
    src_dir = repository_ctx.path("src")

    clone_args = ["git", "clone"]
    if repository_ctx.attr.branch:
        clone_args.extend(["--branch", repository_ctx.attr.branch])
    clone_args.extend([repository_ctx.attr.remote, str(src_dir)])
    clone_res = repository_ctx.execute(clone_args)
    if clone_res.return_code != 0:
        fail("Failed to clone connect-iq-sdk-manager-cli:\n{}\n{}".format(
            clone_res.stdout,
            clone_res.stderr,
        ))

    if repository_ctx.attr.commit:
        checkout_res = repository_ctx.execute(
            ["git", "checkout", repository_ctx.attr.commit],
            working_directory = str(src_dir),
        )
        if checkout_res.return_code != 0:
            fail("Failed to checkout commit {} in connect-iq-sdk-manager-cli:\n{}\n{}".format(
                repository_ctx.attr.commit,
                checkout_res.stdout,
                checkout_res.stderr,
            ))

    repository_ctx.report_progress("Building connect-iq-sdk-manager")
    res = repository_ctx.execute(
        [go_bin, "build", "-modcacherw", "-o", str(raw_bin), "."],
        working_directory = str(src_dir),
    )
    if res.return_code != 0:
        fail("Failed to build connect-iq-sdk-manager:\n{}\n{}".format(res.stdout, res.stderr))

    # Store the hermetic home at `<output_base>/ciq_hermetic_home` (outside `external/` so
    # Bazel's Skyframe tracks its files as normal external paths rather than a synthetic
    # `@@ciq_hermetic_home` repository, and so the path never contains `+`).
    sdk_manager_home = str(repository_ctx.path(".").dirname.dirname.get_child("ciq_hermetic_home"))
    wrapper_name = "connect-iq-sdk-manager.bat" if is_windows else "connect-iq-sdk-manager"
    wrapper_ext = ".bat" if is_windows else ""
    wrapper_template = _SDK_MANAGER_BAT_WRAPPER_TEMPLATE if is_windows else _SDK_MANAGER_WRAPPER_TEMPLATE
    repository_ctx.file(
        wrapper_name,
        wrapper_template.format(
            sdk_manager_home = sdk_manager_home,
            sdk_manager_bin = str(raw_bin),
        ),
        executable = True,
    )
    repository_ctx.file("defs.bzl", _SDK_MANAGER_BIN_DEFS)
    repository_ctx.file(
        "BUILD.bazel",
        _SDK_MANAGER_BUILD_TEMPLATE.format(
            wrapper_name = wrapper_name,
            wrapper_ext = wrapper_ext,
        ),
    )

connect_iq_sdk_manager = repository_rule(
    implementation = _connect_iq_sdk_manager_impl,
    attrs = {
        "branch": attr.string(
            doc = "Optional git branch when cloning from `remote`.",
        ),
        "commit": attr.string(
            doc = "Optional git commit SHA to check out after cloning `remote`.",
        ),
        "remote": attr.string(
            default = "https://github.com/semolina-solutions/connect-iq-sdk-manager-cli.git",
            doc = "Git remote URL for connect-iq-sdk-manager-cli.",
        ),
        "_go_bin": attr.label(
            default = Label("@go_default_sdk//:bin/go"),
        ),
    },
)

def _os_config(repository_ctx):
    """Returns OS-specific subpath and SDK layout metadata for the hermetic home.

    Garmin's SDK tools (`monkeybrains.jar`, `simulator`, `shell`) and
    `connect-iq-sdk-manager-cli` do not accept a flag or environment variable to
    override the ConnectIQ data directory directly; instead, they resolve it by
    appending an OS-specific relative path to the user's home directory (`$HOME` /
    Java `user.home`, or `%APPDATA%` on Windows). To keep all Connect IQ assets
    hermetic inside `<output_base>/ciq_hermetic_home`, we override `HOME`,
    `user.home`, and `APPDATA` to point to `ciq_hermetic_home` and mirror the
    expected OS-specific relative path beneath it.
    """
    os_name = repository_ctx.os.name.lower()
    if os_name == "mac os x" or os_name == "darwin":
        # On macOS, reach into ConnectIQ.app to find the simulator executable
        # directly rather than calling `open`, so it can be terminated by the
        # runner script.
        return struct(
            ciq_rel_path = "Library/Application Support/Garmin/ConnectIQ",
            sdks_json_key = "mac",
            shell_bin = "shell",
            simulator_bin = "ConnectIQ.app/Contents/MacOS/simulator",
        )
    elif os_name == "linux":
        return struct(
            ciq_rel_path = ".Garmin/ConnectIQ",
            sdks_json_key = "linux",
            shell_bin = "shell",
            simulator_bin = "simulator",
        )
    elif "windows" in os_name:
        return struct(
            ciq_rel_path = "AppData/Roaming/Garmin/ConnectIQ",
            sdks_json_key = "windows",
            shell_bin = "shell.exe",
            simulator_bin = "simulator.exe",
        )
    else:
        fail("Unsupported operating system: {}".format(repository_ctx.os.name))

def _sdk_version_from_folder_name(name):
    if not name.startswith("connectiq-sdk-"):
        return None
    parts = name.split("-")
    if len(parts) < 4 or not parts[3] or not parts[3][0].isdigit():
        return None
    return parts[3]

def _sdk_paths_for_version(repository_ctx, version, sdk_dir, hermetic_home, sdk_manager_bin, java_bin, os_cfg):
    sdk_dir = sdk_dir.replace("\\", "/")
    hermetic_home = hermetic_home.replace("\\", "/")
    sdk_manager_bin = (sdk_manager_bin or "").replace("\\", "/")
    java_bin = (java_bin or "java").replace("\\", "/")
    raw_shell_path = paths.join(sdk_dir, "bin", os_cfg.shell_bin)
    raw_simulator_path = paths.join(sdk_dir, "bin", os_cfg.simulator_bin)

    monkeyc_rel = "sdk/{}/monkeyc_wrapper.sh".format(version)
    shell_rel = "sdk/{}/shell_wrapper.sh".format(version)
    simulator_rel = "sdk/{}/simulator_wrapper.sh".format(version)

    repository_ctx.file(
        monkeyc_rel,
        _MONKEYC_WRAPPER_TEMPLATE.format(
            hermetic_home = hermetic_home,
            sdk_dir = sdk_dir,
            sdk_manager_bin = sdk_manager_bin,
            java_bin = java_bin,
            version = version,
        ).replace("\r\n", "\n"),
        executable = True,
    )
    repository_ctx.file(
        shell_rel,
        _SHELL_WRAPPER_TEMPLATE.format(
            hermetic_home = hermetic_home,
            sdk_dir = sdk_dir,
            sdk_manager_bin = sdk_manager_bin,
            version = version,
            shell_path = raw_shell_path,
        ).replace("\r\n", "\n"),
        executable = True,
    )
    repository_ctx.file(
        simulator_rel,
        _SIMULATOR_WRAPPER_TEMPLATE.format(
            hermetic_home = hermetic_home,
            sdk_dir = sdk_dir,
            sdk_manager_bin = sdk_manager_bin,
            version = version,
            simulator_path = raw_simulator_path,
        ).replace("\r\n", "\n"),
        executable = True,
    )
    return struct(
        monkeyc_path = str(repository_ctx.path(monkeyc_rel)).replace("\\", "/"),
        shell_path = str(repository_ctx.path(shell_rel)).replace("\\", "/"),
        simulator_path = str(repository_ctx.path(simulator_rel)).replace("\\", "/"),
    )

def _populate_ciq_repo(
        repository_ctx,
        hermetic_ciq_path,
        hermetic_home,
        sdk_manager_bin,
        sdks_manifest,
        os_cfg):
    sdks_dir_path = paths.join(hermetic_ciq_path, "Sdks")
    sdk_dirs_by_version = {}

    if sdks_manifest:
        for entry in sdks_manifest:
            version = entry.get("version")
            archive = entry.get(os_cfg.sdks_json_key, "")
            if version and archive:
                folder_name = archive.removesuffix(".dmg").removesuffix(".zip")
                sdk_dirs_by_version[version] = paths.join(sdks_dir_path, folder_name)

    sdks_dir = repository_ctx.path(sdks_dir_path)
    if sdks_dir.exists:
        for entry in sdks_dir.readdir():
            if not entry.is_dir:
                continue
            version = _sdk_version_from_folder_name(entry.basename)
            if version:
                sdk_dirs_by_version[version] = str(entry)

    if not sdk_dirs_by_version:
        fail("No Garmin Connect IQ SDKs found in {}".format(sdks_dir_path))

    java_path = repository_ctx.which("java")
    java_bin = str(java_path).replace("\\", "/") if java_path else "java"
    sorted_versions = sorted(sdk_dirs_by_version.keys())
    wrapper_blocks = []
    for version in sorted_versions:
        sdk_paths = _sdk_paths_for_version(
            repository_ctx,
            version,
            sdk_dirs_by_version[version],
            hermetic_home,
            sdk_manager_bin,
            java_bin,
            os_cfg,
        )
        wrapper_blocks.append(
            SDK_PROVIDER_WRAPPER_TEMPLATE.format(
                name = version,
                monkeyc_path = sdk_paths.monkeyc_path,
                shell_path = sdk_paths.shell_path,
                simulator_path = sdk_paths.simulator_path,
            ),
        )

    repository_ctx.file("sdk/defs.bzl", SDK_DEFS_CONTENT)
    repository_ctx.file(
        "sdk/BUILD.bazel",
        SDK_BUILD_TEMPLATE.format(
            sdk_provider_wrappers = "\n".join(wrapper_blocks),
        ),
    )

    fonts_dir_path = paths.join(hermetic_ciq_path, "Fonts")
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

    devices_dir_path = paths.join(hermetic_ciq_path, "Devices")
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

        repository_ctx.symlink(device_dir, "devices/" + device_id)

    repository_ctx.file("defs.bzl", DEFS_CONTENT.format(
        devices = str(device_metadata_dict).replace("true", "True").replace("false", "False"),
        sdk_versions = str(sorted_versions),
    ))

    repository_ctx.file("devices.json", json.encode(device_metadata_dict))

def _provision_hermetic_ciq(repository_ctx, hermetic_home, hermetic_ciq_path):
    sdk_manager_bin = repository_ctx.path(repository_ctx.attr._sdk_manager_bin)
    env = {
        "HOME": hermetic_home,
        "USERPROFILE": hermetic_home,
        "CIQ_HERMETIC_HOME": hermetic_home,
        "APPDATA": paths.join(hermetic_home, "AppData/Roaming"),
    }

    agreement_hash = repository_ctx.attr.agreement_hash or repository_ctx.os.environ.get("CIQ_AGREEMENT_HASH")
    if agreement_hash:
        res = repository_ctx.execute(
            [
                sdk_manager_bin,
                "agreement",
                "accept",
                "--agreement-hash=" + agreement_hash,
            ],
            environment = env,
        )
        if res.return_code != 0:
            fail("Failed to accept Garmin Connect IQ agreement:\n{}\n{}".format(res.stdout, res.stderr))

    token_path = repository_ctx.path(paths.join(hermetic_ciq_path, "token.json"))
    if not token_path.exists:
        username = repository_ctx.os.environ.get("GARMIN_USERNAME")
        password = repository_ctx.os.environ.get("GARMIN_PASSWORD")
        if username and password:
            repository_ctx.report_progress("Logging in to Garmin Connect IQ")
            login_res = repository_ctx.execute(
                [sdk_manager_bin, "login"],
                environment = env,
            )
            if login_res.return_code != 0:
                fail("Failed to log in to Garmin Connect IQ:\n{}\n{}".format(
                    login_res.stdout,
                    login_res.stderr,
                ))

    repository_ctx.report_progress("Downloading Garmin Connect IQ devices and fonts")
    device_res = repository_ctx.execute(
        [
            sdk_manager_bin,
            "device",
            "download",
            "--download-all",
            "--include-fonts",
        ],
        environment = env,
    )
    if device_res.return_code != 0:
        fail("Failed to download Garmin Connect IQ devices and fonts:\n{}\n{}\n{}".format(
            device_res.stdout,
            device_res.stderr,
            _SETUP_INSTRUCTIONS,
        ))

def _ciq_impl(repository_ctx):
    os_cfg = _os_config(repository_ctx)

    # Store the hermetic home at `<output_base>/ciq_hermetic_home` (outside `external/` so
    # Bazel's Skyframe tracks its files as normal external paths rather than a synthetic
    # `@@ciq_hermetic_home` repository, and so the path never contains `+` which
    # `monkeyc`'s ANTLR `.jungle` parser rejects when reading `<sdk>/bin/default.jungle`).
    hermetic_home = str(repository_ctx.path(".").dirname.dirname.get_child("ciq_hermetic_home"))
    hermetic_ciq_path = paths.join(hermetic_home, os_cfg.ciq_rel_path)

    _provision_hermetic_ciq(repository_ctx, hermetic_home, hermetic_ciq_path)
    repository_ctx.download(
        url = "https://developer.garmin.com/downloads/connect-iq/sdks/sdks.json",
        output = "sdks.json",
    )
    sdks_manifest = json.decode(repository_ctx.read("sdks.json"))
    _populate_ciq_repo(
        repository_ctx,
        hermetic_ciq_path,
        hermetic_home = hermetic_home,
        sdk_manager_bin = str(repository_ctx.path(repository_ctx.attr._sdk_manager_bin)),
        sdks_manifest = sdks_manifest,
        os_cfg = os_cfg,
    )

ciq = repository_rule(
    implementation = _ciq_impl,
    attrs = {
        "agreement_hash": attr.string(
            doc = "Optional MD5 hash of the Garmin Connect IQ license agreement to accept non-interactively.",
        ),
        "_sdk_manager_bin": attr.label(
            default = Label("@connect_iq_sdk_manager_cli//:connect-iq-sdk-manager-bin.exe"),
        ),
    },
    configure = True,
    environ = [
        "CIQ_AGREEMENT_HASH",
        "GARMIN_PASSWORD",
        "GARMIN_USERNAME",
    ],
    local = False,
)

