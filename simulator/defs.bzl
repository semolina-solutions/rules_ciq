"""
Rules for running, automating, and testing applications in the Connect IQ Simulator.
"""

load("@bazel_skylib//lib:paths.bzl", "paths")
load("@ciq//sdk:defs.bzl", "SdkInfo")
load("//build:defs.bzl", "DeviceBuildInfo", "ManifestInfo")

# See https://github.com/bazelbuild/bazel/blob/master/tools/bash/runfiles/runfiles.bash
# and https://bazel.build/docs/runfiles
# This is the canonical snippet needed to locate and load the Bazel bash runfiles
# library robustly across Bazel versions without manual Starlark path resolution.
_BASH_RUNFILES_BOILERPLATE = """
# --- begin runfiles.bash initialization v3 ---
# Copy-pasted from the Bazel Bash runfiles library v3.
set -uo pipefail; set +e; f=bazel_tools/tools/bash/runfiles/runfiles.bash
source "${RUNFILES_DIR:-/dev/null}/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "${RUNFILES_MANIFEST_FILE:-/dev/null}" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$0.runfiles/$f" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  source "$(grep -sm1 "^$f " "$0.exe.runfiles_manifest" | cut -f2- -d' ')" 2>/dev/null || \\
  { echo>&2 "ERROR: cannot find $f"; exit 1; }; f=; set -e
# --- end runfiles.bash initialization v3 ---
"""

def _shell_quote(s):
    return "'" + str(s).replace("'", "'\"'\"'") + "'"

AutomationInfo = provider(
    "Provider for Garmin Connect IQ simulator automation outputs.",
    fields = {
        "screenshot_file": "The captured simulator display screenshot (.png).",
        "log_file": "The captured application stdout log (.log).",
        "device_id": "The device ID simulated (e.g., 'fenix6').",
    },
)

def _ciq_simulation_impl(ctx):
    sdk_info = ctx.attr.device_build[SdkInfo]
    manifest_info = ctx.attr.device_build[ManifestInfo]
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")

    # The runfiles script enables use of rlocation, and $(rlocation ...) is
    # used to find the absolute paths to particular files that seem to be in
    # different locations depending on the consumption pattern of the module.
    script_content = """
        #!/usr/bin/env bash
        {runfiles_boilerplate}
        GET_APPLICATION_ID_TOOL=$(rlocation "{get_application_id_tool}")
        MANIFEST_XML_PATH=$(rlocation "{manifest_xml_path}")
        APPLICATION_ID=$($GET_APPLICATION_ID_TOOL "$MANIFEST_XML_PATH")
        {simulator_tool} "{simulator_path}" "{shell_path}" "$APPLICATION_ID" "{prg_path}" "{debug_xml_path}" "{settings_json_path}" {device_id}
    """
    ctx.actions.write(
        output = output_script,
        content = script_content.format(
            runfiles_boilerplate = _BASH_RUNFILES_BOILERPLATE,
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

    # Merge runfiles from the _runfiles target so that root_symlinks
    # and other runtime data are available.
    runfiles = ctx.runfiles(
        files = [
            output_script,
            device_build_info.prg_file,
            device_build_info.prg_debug_xml_file,
            device_build_info.settings_json_file,
            manifest_info.manifest_file,
            ctx.executable._get_application_id_tool,
            ctx.executable._simulator_tool,
        ],
    ).merge(ctx.attr._runfiles[DefaultInfo].default_runfiles)

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = runfiles,
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
        "_runfiles": attr.label(
            default = Label("@bazel_tools//tools/bash/runfiles"),
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
        "_runfiles": attr.label(
            default = Label("@bazel_tools//tools/bash/runfiles"),
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

def _ciq_automation_impl(ctx):
    sdk_info = ctx.attr.device_build[SdkInfo]
    manifest_info = ctx.attr.device_build[ManifestInfo]
    device_build_info = ctx.attr.device_build[DeviceBuildInfo]
    output_png = ctx.outputs.screenshot_out
    output_log = ctx.outputs.log_out

    args = ctx.actions.args()
    args.add("--ensure-linux-sdk", sdk_info.ensure_linux_sdk_path)
    args.add("--hermetic-ciq-dir", sdk_info.hermetic_ciq_path)
    args.add("--dockerfile", ctx.file._dockerfile)
    args.add("--hook-src", ctx.file._hook_src)
    args.add("--manifest-xml", manifest_info.manifest_file)
    args.add("--prg", device_build_info.prg_file)
    args.add("--debug-xml", device_build_info.prg_debug_xml_file)
    args.add("--settings-json", device_build_info.settings_json_file)
    args.add("--device", device_build_info.device_id)
    args.add("--output", output_png)
    args.add("--log-output", output_log)
    args.add("--delay-ms", str(ctx.attr.delay_ms))
    args.add("--timeout-secs", str(ctx.attr.timeout_secs))

    if ctx.attr.layout:
        args.add("--layout", ctx.attr.layout)
    if ctx.attr.background_color:
        args.add("--background-color", ctx.attr.background_color)
    if ctx.attr.time_display:
        args.add("--time-display", ctx.attr.time_display)
    if ctx.attr.units_display:
        args.add("--units-display", ctx.attr.units_display)
    if ctx.attr.language:
        args.add("--language", ctx.attr.language)
    if ctx.attr.simulate_activity:
        args.add("--simulate-activity")
    if ctx.file.fit_file:
        args.add("--fit-file", ctx.file.fit_file)
    for timer_action in ctx.attr.timer_actions:
        args.add("--timer-action", timer_action)
    for key, val in ctx.attr.settings.items():
        args.add("--setting", "{}={}".format(key, val))
    for action in ctx.attr.actions:
        args.add("--action", action)
    if ctx.attr.wait_for_log:
        args.add("--wait-for-log", ctx.attr.wait_for_log)

    inputs = [
        ctx.file._dockerfile,
        ctx.file._hook_src,
        manifest_info.manifest_file,
        device_build_info.prg_file,
        device_build_info.prg_debug_xml_file,
        device_build_info.settings_json_file,
    ]
    if ctx.file.fit_file:
        inputs.append(ctx.file.fit_file)

    ctx.actions.run(
        outputs = [output_png, output_log],
        inputs = inputs,
        executable = ctx.executable._automate_tool,
        arguments = [args],
        mnemonic = "CiqAutomation",
        progress_message = "Running Connect IQ simulator automation for %s (%s)" % (
            ctx.label.name,
            device_build_info.device_id,
        ),
        use_default_shell_env = True,
        execution_requirements = {
            "local": "1",
            "no-sandbox": "1",
            "requires-network": "1",
        },
    )

    return [
        AutomationInfo(
            screenshot_file = output_png,
            log_file = output_log,
            device_id = device_build_info.device_id,
        ),
        DefaultInfo(
            files = depset([output_png, output_log]),
        ),
        OutputGroupInfo(
            screenshot = depset([output_png]),
            log = depset([output_log]),
        ),
    ]

_ciq_automation = rule(
    implementation = _ciq_automation_impl,
    doc = "Internal rule that runs a ciq_device_build target in a containerized Connect IQ Simulator and produces a screenshot (.png) and application log (.log).",
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to run in the containerized simulator.",
            mandatory = True,
            providers = [SdkInfo, ManifestInfo, DeviceBuildInfo],
        ),
        "screenshot_out": attr.output(
            doc = "Predeclared output file for the captured display screenshot PNG.",
            mandatory = True,
        ),
        "log_out": attr.output(
            doc = "Predeclared output file for the captured application stdout log.",
            mandatory = True,
        ),
        "layout": attr.string(
            doc = "Optional Data Field layout (e.g., '1 Field', '2 Fields', '3 Fields A', '3 Fields B', '4 Fields A').",
        ),
        "background_color": attr.string(
            doc = "Optional Data Field background color ('White' or 'Black').",
        ),
        "time_display": attr.string(
            doc = "Optional simulator time display format ('12 Hour' or '24 Hour').",
        ),
        "units_display": attr.string(
            doc = "Optional simulator units display format ('Metric' or 'Statute').",
        ),
        "language": attr.string(
            doc = "Optional simulator language (e.g., 'English', 'French', 'German', or ISO code like 'eng', 'fre', 'deu').",
        ),
        "simulate_activity": attr.bool(
            default = False,
            doc = "Whether to simulate FIT/sensor activity data during the session.",
        ),
        "fit_file": attr.label(
            allow_single_file = [".fit", ".gpx"],
            doc = "Optional .fit or .gpx activity file to play back during the session.",
        ),
        "timer_actions": attr.string_list(
            doc = "Optional sequence of Data Field Activity Timer controls to trigger (e.g., ['start'], ['start', 'lap'], ['start', 'workout_step'], ['start', 'pause']).",
        ),
        "settings": attr.string_dict(
            doc = "Optional dictionary of custom application settings/properties to apply (e.g., {'myKey': 'int:42', 'theme': 'dark', 'enabled': 'true'}).",
        ),
        "actions": attr.string_list(
            doc = "Optional ordered sequence of button, touch, menu, or wait actions to execute (e.g., ['press:down', 'wait:500', 'press:enter', 'swipe:left', 'tap:50%,50%', 'tap:130,130', 'drag:80%,50%,20%,50%']). Touch coordinates default to pixels relative to the display origin and also accept `<n>%` percentage coordinates relative to the display dimensions.",
        ),
        "wait_for_log": attr.string(
            doc = "Optional substring or regex to wait for in application stdout logs before triggering automation.",
        ),
        "delay_ms": attr.int(
            default = 1500,
            doc = "Delay in milliseconds to wait after launching and executing actions before capturing the final screenshot and log.",
        ),
        "timeout_secs": attr.int(
            default = 120,
            doc = "Overall timeout in seconds for the containerized simulator session.",
        ),
        "_dockerfile": attr.label(
            allow_single_file = True,
            default = Label("//simulator:Dockerfile"),
        ),
        "_hook_src": attr.label(
            allow_single_file = True,
            default = Label("//simulator:gtk_automation_hook.rs"),
        ),
        "_automate_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:automate"),
        ),
    },
)

def ciq_automation(name, screenshot_out = None, log_out = None, **kwargs):
    """Runs a ciq_device_build target in a containerized Connect IQ Simulator and produces two outputs: `<name>.png` and `<name>.log`.

    Downstream rules can reference the predeclared output files directly via `:<name>.png` and `:<name>.log`,
    or pass `:<name>` to rules that accept `AutomationInfo`.

    Args:
        name: The target name.
        screenshot_out: Optional custom filename for the generated screenshot PNG (defaults to `<name>.png`).
        log_out: Optional custom filename for the generated application log (defaults to `<name>.log`).
        **kwargs: Arguments forwarded to the underlying automation rule (`device_build`, `layout`, `background_color`, `settings`, `actions`, `timer_actions`, `simulate_activity`, `fit_file`, `language`, `time_display`, `units_display`, `wait_for_log`, `delay_ms`, `timeout_secs`, etc.).
    """
    _ciq_automation(
        name = name,
        screenshot_out = screenshot_out or (name + ".png"),
        log_out = log_out or (name + ".log"),
        **kwargs
    )

def _resolve_log_file(target):
    if AutomationInfo in target:
        return target[AutomationInfo].log_file
    files = target[DefaultInfo].files.to_list()
    if len(files) == 1:
        return files[0]
    log_files = [f for f in files if f.extension == "log"]
    if len(log_files) == 1:
        return log_files[0]
    fail("Expected a single .log file or a ciq_automation target, got: {}".format(files))

def _resolve_screenshot_file(target):
    if AutomationInfo in target:
        return target[AutomationInfo].screenshot_file
    files = target[DefaultInfo].files.to_list()
    if len(files) == 1:
        return files[0]
    png_files = [f for f in files if f.extension == "png"]
    if len(png_files) == 1:
        return png_files[0]
    fail("Expected a single .png file or a ciq_automation target, got: {}".format(files))

def _ciq_log_test_impl(ctx):
    log_file = _resolve_log_file(ctx.attr.log)
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")

    cmd_parts = [
        _shell_quote(ctx.executable._verify_log_tool.short_path),
        "--log",
        _shell_quote(log_file.short_path),
    ]
    for pattern in ctx.attr.expect_logs:
        cmd_parts.extend(["--expect-log", _shell_quote(pattern)])
    for pattern in ctx.attr.forbid_logs:
        cmd_parts.extend(["--forbid-log", _shell_quote(pattern)])

    script_content = "#!/usr/bin/env bash\nset -euo pipefail\nexec {}\n".format(" ".join(cmd_parts))
    ctx.actions.write(
        output = output_script,
        content = script_content,
        is_executable = True,
    )

    runfiles = ctx.runfiles(
        files = [
            output_script,
            ctx.executable._verify_log_tool,
            log_file,
        ],
    )

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = runfiles,
        ),
    ]

ciq_log_test = rule(
    implementation = _ciq_log_test_impl,
    doc = "Verifies that an application log file (from ciq_automation) matches expected and forbidden substring/regex patterns.",
    test = True,
    attrs = {
        "log": attr.label(
            allow_files = True,
            doc = "The .log file (e.g., ':my_automation.log') or ciq_automation target to assert against.",
            mandatory = True,
        ),
        "expect_logs": attr.string_list(
            doc = "List of substrings or regex patterns that must appear in the log file.",
        ),
        "forbid_logs": attr.string_list(
            doc = "List of substrings or regex patterns that must not appear in the log file.",
        ),
        "_verify_log_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:verify_log"),
        ),
    },
)

def _ciq_screenshot_test_impl(ctx):
    screenshot_file = _resolve_screenshot_file(ctx.attr.screenshot)
    expected_file = ctx.file.expected
    output_script = ctx.actions.declare_file(ctx.label.name + ".sh")

    cmd_parts = [
        _shell_quote(ctx.executable._verify_screenshot_tool.short_path),
        "--actual",
        _shell_quote(screenshot_file.short_path),
        "--expected",
        _shell_quote(expected_file.short_path),
        "--pixel-tolerance",
        _shell_quote(ctx.attr.pixel_tolerance),
        "--max-diff-pixels",
        _shell_quote(ctx.attr.max_diff_pixels),
    ]

    script_content = "#!/usr/bin/env bash\nset -euo pipefail\nexec {}\n".format(" ".join(cmd_parts))
    ctx.actions.write(
        output = output_script,
        content = script_content,
        is_executable = True,
    )

    runfiles = ctx.runfiles(
        files = [
            output_script,
            ctx.executable._verify_screenshot_tool,
            screenshot_file,
            expected_file,
        ],
    )

    return [
        DefaultInfo(
            executable = output_script,
            runfiles = runfiles,
        ),
    ]

ciq_screenshot_test = rule(
    implementation = _ciq_screenshot_test_impl,
    doc = "Compares a captured screenshot PNG (from ciq_automation, ciq_field_screenshot, or ciq_framed_screenshot) against a golden reference image.",
    test = True,
    attrs = {
        "screenshot": attr.label(
            allow_files = True,
            doc = "The screenshot .png file (e.g., ':my_automation.png') or ciq_automation target to compare.",
            mandatory = True,
        ),
        "expected": attr.label(
            allow_single_file = [".png"],
            doc = "The golden reference .png image to compare against.",
            mandatory = True,
        ),
        "pixel_tolerance": attr.int(
            default = 0,
            doc = "Maximum per-channel (R, G, B, A) absolute difference (0-255) treated as matching for a given pixel.",
        ),
        "max_diff_pixels": attr.int(
            default = 0,
            doc = "Maximum number of pixels allowed to exceed pixel_tolerance.",
        ),
        "_verify_screenshot_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:verify_screenshot"),
        ),
    },
)
