"""
Rules for interacting with physical Garmin devices.
"""

load("//build:defs.bzl", "DeviceBuildInfo")
load("@bazel_skylib//lib:paths.bzl", "paths")

def _calculatePrgPath(prg_file):
    """Calculates the destination path for the .prg file on the device.

    Args:
        prg_file: A .prg file.

    Returns:
        The absolute path on the device where the .prg file should be placed.
    """
    return paths.join("/GARMIN/Apps", prg_file.basename)

def _calculateLogPath(prg_file):
    """Calculates the path for the log file corresponding to the .prg file on the device.

    Args:
        prg_file: A .prg file.

    Returns:
        The absolute path on the device where the log file is located.
    """
    txt_basename = paths.replace_extension(prg_file.basename, ".TXT")
    return paths.join("/GARMIN/Apps/LOGS", txt_basename)

def _ciq_sideload_app_impl(ctx):
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")

    empty_log_file = ctx.actions.declare_file(ctx.label.name + ".empty.txt")
    ctx.actions.write(
        output = empty_log_file,
        content = "",
    )

    # The "?" prefix on the log_dst path means that the upload will "soft-fail":
    # if the file already exists on the device, it will not be replaced.
    script_content = """
        {tool} "{prg_src}" "{prg_dst}" "{log_src}" "?{log_dst}"
    """

    ctx.actions.write(
        output = output_script,
        content = script_content.format(
            tool = ctx.executable._mtp_upload_tool.short_path,
            prg_src = device_build_info.prg_file.short_path,
            prg_dst = _calculatePrgPath(device_build_info.prg_file),
            log_src = empty_log_file.short_path,
            log_dst = _calculateLogPath(device_build_info.prg_file),
        ),
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = ctx.runfiles(
                files = [
                    empty_log_file,
                    output_script,
                    device_build_info.prg_file,
                    device_build_info.settings_json_file,
                ] + ctx.attr._mtp_upload_tool.files.to_list(),
            ),
        ),
    ]

ciq_sideload_app = rule(
    implementation = _ciq_sideload_app_impl,
    doc = "Sideloads the application (.prg) to a connected physical Garmin device via MTP.",
    executable = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to sideload.",
            mandatory = True,
            providers = [DeviceBuildInfo],
        ),
        "_mtp_upload_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//device:upload"),
        ),
    },
)

def _ciq_view_app_log_impl(ctx):
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")
    log_dst_path = "{}.txt".format(ctx.label.name)

    ctx.actions.write(
        output = output_script,
        content = """
            touch "{log_dst}"
            {tool} "{log_src}" "{log_dst}"
            cat "{log_dst}"
        """.format(
            tool = ctx.executable._mtp_download_tool.short_path,
            log_src = _calculateLogPath(device_build_info.prg_file),
            log_dst = log_dst_path,
        ),
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = ctx.runfiles(
                files = [
                    output_script,
                ] + ctx.attr._mtp_download_tool.files.to_list(),
            ),
        ),
    ]

ciq_view_app_log = rule(
    implementation = _ciq_view_app_log_impl,
    doc = "Downloads and outputs the log file from a connected physical Garmin device.",
    executable = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to retrieve the log file for.",
            mandatory = True,
            providers = [DeviceBuildInfo],
        ),
        "_mtp_download_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//device:download"),
        ),
    },
)

def _calculatePrfPath(prg_file):
    """Calculates the path for the profiling log file corresponding to the .prg file on the device.

    Args:
        prg_file: A .prg file.

    Returns:
        The absolute path on the device where the profiling log file is located.
    """
    prf_basename = paths.replace_extension(prg_file.basename, ".PRF")
    return paths.join("/GARMIN/Apps/LOGS", prf_basename)

def _ciq_view_app_profiling_impl(ctx):
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")
    prf_dst_path = "{}.PRF".format(ctx.label.name)

    # We don't declare the PRF as an output artifact because it's dynamic/downloaded at runtime.
    # However, we need to make sure the debug XML is available.

    ctx.actions.write(
        output = output_script,
        content = """
            {download_tool} "{prf_src}" "{prf_dst}"
            {interpret_tool} "{prf_dst}" --debug-xml "{debug_xml}" "$@"
        """.format(
            download_tool = ctx.executable._mtp_download_tool.short_path,
            interpret_tool = ctx.executable._interpret_profiling_tool.short_path,
            prf_src = _calculatePrfPath(device_build_info.prg_file),
            prf_dst = prf_dst_path,
            debug_xml = device_build_info.prg_debug_xml_file.short_path,
        ),
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = ctx.runfiles(
                files = [
                    output_script,
                    device_build_info.prg_debug_xml_file,
                ] + ctx.attr._mtp_download_tool.files.to_list() + ctx.attr._interpret_profiling_tool.files.to_list(),
            ),
        ),
    ]

ciq_view_app_profiling = rule(
    implementation = _ciq_view_app_profiling_impl,
    doc = "Downloads the profiling log (.PRF) from the device and analyzes it using the debug XML.",
    executable = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to retrieve the profiling log for.",
            mandatory = True,
            providers = [DeviceBuildInfo],
        ),
        "_mtp_download_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//device:download"),
        ),
        "_interpret_profiling_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//device:interpret_profiling_log"),
        ),
    },
)
