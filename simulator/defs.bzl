"""
Rules for running applications in the Connect IQ Simulator.
"""

load("//build:defs.bzl", "DeviceBuildInfo", "ManifestInfo")
load("@local_ciq//sdk:defs.bzl", "SdkInfo")
load("@bazel_skylib//lib:paths.bzl", "paths")

def _ciq_simulation_impl(ctx):
    sdk_info = ctx.attr.device_build[SdkInfo]
    manifest_info = ctx.attr.device_build[ManifestInfo]
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")

    # The runfiles script enables use of rlocation, and $(rlocation ...) is
    # used to find the absolute paths to particular files that seem to be in
    # different locations depending on the consumption pattern of the module.
    script_content = """
        source {runfiles_script}
        GET_APPLICATION_ID_TOOL=$(rlocation {get_application_id_tool})
        MANIFEST_XML_PATH=$(rlocation {manifest_xml_path})
        APPLICATION_ID=$($GET_APPLICATION_ID_TOOL $MANIFEST_XML_PATH)
        {simulator_tool} "{simulator_path}" "{shell_path}" "$APPLICATION_ID" "{prg_path}" "{debug_xml_path}" "{settings_json_path}" {device_id}
    """
    ctx.actions.write(
        output = output_script,
        content = script_content.format(
            runfiles_script = ctx.file._runfiles_script.short_path,
            get_application_id_tool = paths.normalize(paths.join(ctx.workspace_name, ctx.executable._get_application_id_tool.short_path)),
            manifest_xml_path = paths.normalize(paths.join(ctx.workspace_name, manifest_info.manifest_file.short_path)),
            simulator_tool = ctx.executable._simulator_tool.short_path,
            simulator_path = sdk_info.simulator_path,
            shell_path = sdk_info.shell_path,
            prg_path = device_build_info.prg_file.short_path,
            debug_xml_path = device_build_info.prg_debug_xml_file.short_path,
            settings_json_path = device_build_info.settings_json_file.short_path,
            device_id = device_build_info.device_id,
        ),
        is_executable = True,
    )
    return [
        DefaultInfo(
            executable = output_script,
            runfiles = ctx.runfiles(
                files = [
                    output_script,
                    device_build_info.prg_file,
                    device_build_info.prg_debug_xml_file,
                    device_build_info.settings_json_file,
                    manifest_info.manifest_file,
                    ctx.file._runfiles_script,
                    ctx.executable._get_application_id_tool,
                    ctx.executable._simulator_tool,
                ],
            ),
        ),
        testing.ExecutionInfo({
            "local": "1",
        }),
    ]

ciq_simulation = rule(
    implementation = _ciq_simulation_impl,
    doc = "Creates a script to run the application in the Connect IQ Simulator.",
    executable = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to run in the simulator.",
            mandatory = True,
            providers = [SdkInfo, ManifestInfo, DeviceBuildInfo],
        ),
        "_runfiles_script": attr.label(
            default = Label("@bazel_tools//tools/bash/runfiles"),
            allow_single_file = True,
        ),
        "_get_application_id_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:get_application_id"),
        ),
        "_simulator_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:simulate"),
        ),
    },
)

ciq_test = rule(
    implementation = _ciq_simulation_impl,
    doc = "Creates a script to run the application tests in the Connect IQ Simulator.",
    test = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target (with include_tests=True) to test in the simulator.",
            mandatory = True,
            providers = [SdkInfo, ManifestInfo, DeviceBuildInfo],
        ),
        "_runfiles_script": attr.label(
            default = Label("@bazel_tools//tools/bash/runfiles"),
            allow_single_file = True,
        ),
        "_get_application_id_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:get_application_id"),
        ),
        "_simulator_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:test"),
        ),
    },
)
