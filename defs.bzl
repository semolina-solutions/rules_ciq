"""Bazel rules and providers for building Garmin Connect IQ applications.

This module defines custom Bazel rules for building Connect IQ projects, including:
- Jungle file generation for device-specific resources and sources
- Manifest generation with device filtering based on API level and app type
- Device builds and exports using the MonkeyC compiler
- Simulator support for testing applications
"""

load("@bazel_skylib//lib:paths.bzl", "paths")
load("@local_ciq//:defs.bzl", _devices = "devices")
load("@local_ciq//sdk:defs.bzl", "SdkInfo")

# Re-exports:
devices = _devices

# The environment variable name used to locate the developer key for signing.
_CIQ_DEVELOPER_KEY_PATH_ENV_VAR = "CIQ_DEVELOPER_KEY_PATH"

_TYPE_CHECK_LEVELS = [
    0,  # Silent
    1,  # Gradual
    2,  # Informative
    3,  # Strict
]

# Maps from preferred expression in this module to the two possible expressions
# used in the manifest XML and compiler JSON.
_APP_TYPE_AUDIO_CONTENT_PROVIDER = "audio_content_provider"
_APP_TYPE_DATA_FIELD = "data_field"
_APP_TYPE_WATCH_APP = "watch_app"
_APP_TYPE_WATCH_FACE = "watch_face"
_APP_TYPE_WIDGET = "widget"

_APP_TYPE_FORMATS = {
    _APP_TYPE_AUDIO_CONTENT_PROVIDER: ["audio-content-provider-app", "audioContentProvider"],
    _APP_TYPE_DATA_FIELD: ["datafield", "datafield"],
    _APP_TYPE_WATCH_APP: ["watch-app", "watchApp"],
    _APP_TYPE_WATCH_FACE: ["watchface", "watchFace"],
    _APP_TYPE_WIDGET: ["widget", "widget"],
}

def _filename_without_extension(file):
    """Extracts the filename without its extension from a file object.

    Args:
        file: A file object with a basename property

    Returns:
        The filename without extension (e.g., "somefont" from "somefont.cft")
    """
    return file.basename[0:file.basename.rindex(".")]

JunglesInfo = provider(
    "Provider for jungle files used in Garmin Connect IQ projects.",
    fields = {
        "jungle_files": "List of jungle files that define source and resource paths per device.",
    },
)

def jungle_generator(ctx, generator_func, device_ids = devices.keys()):
    """Generates a jungle file and associated resources/sources using a generator function.

    `generator_func` is given the base directories for any generated source or
    resource files; any files declared and generated should be prefixed with
    the `sources_dir` or `resources_dir` respectively.

    Args:
        ctx: The rule context.
        generator_func: A function that takes (`ctx`, `device_id`, `device_metadata`, `sources_dir`, `resources_dir`) and returns a list of generated files or `None` to skip.
        device_ids: A list of device IDs to generate for.

    Returns:
        A list of providers [JunglesInfo, DefaultInfo].
    """
    jungle_content = ""
    sources_base_dir = paths.join(ctx.label.name, "sources")
    resources_base_dir = paths.join(ctx.label.name, "resources")
    jungle_file = ctx.actions.declare_file(ctx.label.name + ".jungle")
    outputs = [jungle_file]

    for device_id, device_metadata in devices.items():
        if device_id not in device_ids:
            continue

        device_sources_dir = paths.join(sources_base_dir, device_id)
        device_resources_dir = paths.join(resources_base_dir, device_id)

        result = generator_func(ctx, device_id, device_metadata, device_sources_dir, device_resources_dir)
        if not result:
            continue

        generated_files = result
        outputs.extend(generated_files)

        has_sources = False
        has_resources = False

        for f in generated_files:
            if device_sources_dir in f.short_path:
                has_sources = True
            if device_resources_dir in f.short_path:
                has_resources = True

        if has_sources:
            jungle_content += "{id}.sourcePath = $({id}.sourcePath);{dir}\n".format(
                id = device_id,
                dir = device_sources_dir,
            )

        if has_resources:
            jungle_content += "{id}.resourcePath = $({id}.resourcePath);{dir}\n".format(
                id = device_id,
                dir = device_resources_dir,
            )

    ctx.actions.write(jungle_file, jungle_content)

    return [
        JunglesInfo(jungle_files = [jungle_file]),
        DefaultInfo(files = depset(outputs)),
    ]

_DRAWABLES_BITMAP_XML_TEMPLATE = """
<drawables
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:noNamespaceSchemaLocation="https://developer.garmin.com/downloads/connect-iq/resources.xsd"
>
    <bitmap id="{id}" filename="{filename}" automaticPalette="true" />
</drawables>
"""

def _ciq_scaled_drawable_generator(ctx, device_id, device_metadata, _sources_dir, resources_dir):
    compiler = device_metadata["compiler"]

    fraction = ctx.attr.percent / 100.0
    width = None
    height = None

    if ctx.attr.mode == "icon":
        width = int(compiler["launcherIcon"]["width"] * fraction)
        height = int(compiler["launcherIcon"]["height"] * fraction)
    elif ctx.attr.mode == "screen_width":
        width = int(compiler["resolution"]["width"] * fraction)
        height = 0
    elif ctx.attr.mode == "screen_height":
        width = 0
        height = int(compiler["resolution"]["height"] * fraction)
    elif ctx.attr.mode == "screen_fill":
        width = int(compiler["resolution"]["width"] * fraction)
        height = int(compiler["resolution"]["height"] * fraction)
    elif ctx.attr.mode == "font_height":
        if len(ctx.attr.font_name) == 0:
            fail("font_name must be specified")
        simulator = device_metadata["simulator"]
        matched_font_sets = [
            font
            for font in simulator["fonts"]
            if font["fontSet"] == ctx.attr.font_set
        ]
        if len(matched_font_sets) != 1:
            fail("Expected to find exactly 1 matching font set for device_id={}, set={}".format(
                device_id,
                ctx.attr.font_set,
            ))
        fonts = matched_font_sets[0]["fonts"]
        matched_fonts = [
            font
            for font in fonts
            if font["name"] == ctx.attr.font_name
        ]
        if len(matched_fonts) != 1:
            fail("Expected to find exactly 1 matching font for device_id={}, set={}, name={}".format(
                device_id,
                ctx.attr.font_set,
                ctx.attr.font_name,
            ))
        font = matched_fonts[0]
        matched_font_files = [
            file
            for file in ctx.attr._fonts.files.to_list()
            if _filename_without_extension(file) == font["filename"]
        ]
        if len(matched_font_files) == 0:
            # Skip generation for devices with broken font references.
            return None
        font_file = matched_font_files[0]

        width = 0
        if "type" in font and font["type"] == "ttf":
            height = int(font["size"] * simulator["ppi"] / 72)
        else:
            height = """$({tool} "{font_path}")""".format(
                tool = ctx.executable._measure_cft_tool.path,
                font_path = font_file.path,
            )

    if width == None or height == None:
        return None

    image_file = ctx.actions.declare_file(
        paths.join(resources_dir, ctx.file.src.basename),
    )

    ctx.actions.run_shell(
        inputs = [ctx.file.src] + ctx.attr._fonts.files.to_list(),
        outputs = [image_file],
        tools = [
            ctx.executable._measure_cft_tool,
            ctx.executable._scale_image_tool,
        ],
        command = """
            {tool} "{input_path}" "{output_path}" {width} {height}
        """.format(
            tool = ctx.executable._scale_image_tool.path,
            input_path = ctx.file.src.path,
            output_path = image_file.path,
            width = str(width),
            height = str(height),
        ),
    )

    drawables_xml_file = ctx.actions.declare_file(paths.join(resources_dir, "drawables.xml"))
    ctx.actions.write(
        drawables_xml_file,
        _DRAWABLES_BITMAP_XML_TEMPLATE.format(
            id = ctx.attr.resource_id,
            filename = image_file.basename,
        ),
    )

    return [image_file, drawables_xml_file]

def _ciq_scaled_drawable_jungle_impl(ctx):
    return jungle_generator(ctx, _ciq_scaled_drawable_generator, ctx.attr.device_ids)

ciq_scaled_drawable_jungle = rule(
    implementation = _ciq_scaled_drawable_jungle_impl,
    doc = "Generates a jungle file for a scaled drawable resource.",
    attrs = {
        "src": attr.label(
            doc = "Source image file to scale for different devices.",
            mandatory = True,
            allow_single_file = True,
        ),
        "resource_id": attr.string(
            doc = "Resource ID to use in the generated drawables.xml file.",
            mandatory = True,
        ),
        "mode": attr.string(
            doc = "Scaling mode: 'icon' (launcher icon size), 'screen_width', 'screen_height', 'screen_fill' (full screen), or 'font_height' (based on font metrics).",
            mandatory = True,
            values = ["icon", "screen_width", "screen_height", "screen_fill", "font_height"],
        ),
        "font_name": attr.string(
            doc = "Font name to use for 'font_height' mode. Required when mode is 'font_height'.",
        ),
        "font_set": attr.string(
            doc = "Font set to use for 'font_height' mode (e.g., 'ww' for worldwide).",
            default = "ww",
        ),
        "percent": attr.int(
            doc = "Percentage of the base size to scale to (100 = original size).",
            default = 100,
        ),
        "device_ids": attr.string_list(
            doc = "List of device IDs to generate scaled resources for.",
            default = devices.keys(),
        ),
        "_fonts": attr.label(
            default = Label("@local_ciq//:fonts"),
        ),
        "_measure_cft_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//tools:measure_cft"),
        ),
        "_scale_image_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//tools:scale_image"),
        ),
    },
)

def _ciq_jungle_impl(ctx):
    sources_dir = paths.join(ctx.label.name, "sources")
    resources_dir = paths.join(ctx.label.name, "resources")
    jungle_file = ctx.actions.declare_file(ctx.label.name + ".jungle")
    outputs = [jungle_file]
    jungle_content = ""
    for source_file in ctx.files.sources:
        symlinked_source_file = ctx.actions.declare_file(
            paths.join(sources_dir, source_file.short_path),
        )
        outputs.append(symlinked_source_file)
        ctx.actions.symlink(
            output = symlinked_source_file,
            target_file = source_file,
        )
    for resource_file in ctx.files.resources:
        symlinked_resource_file = ctx.actions.declare_file(
            paths.join(resources_dir, resource_file.short_path),
        )
        outputs.append(symlinked_resource_file)
        ctx.actions.symlink(
            output = symlinked_resource_file,
            target_file = resource_file,
        )
    for device_id in ctx.attr.device_ids:
        jungle_content += "{id}.sourcePath = $({id}.sourcePath);{dir}\n".format(
            id = device_id,
            dir = sources_dir,
        )
        jungle_content += "{id}.resourcePath = $({id}.resourcePath);{dir}\n".format(
            id = device_id,
            dir = resources_dir,
        )
    ctx.actions.write(jungle_file, jungle_content)
    return [
        JunglesInfo(jungle_files = [jungle_file]),
        DefaultInfo(files = depset(outputs)),
    ]

ciq_jungle = rule(
    implementation = _ciq_jungle_impl,
    doc = "Generates a jungle file mapping sources and resources for specific devices.",
    attrs = {
        "sources": attr.label_list(
            doc = "List of source files (.mc) to include in the jungle file.",
            allow_files = True,
        ),
        "resources": attr.label_list(
            doc = "List of resource files (e.g., .xml, images) to include in the jungle file.",
            allow_files = True,
        ),
        "device_ids": attr.string_list(
            doc = "List of device IDs to generate jungle entries for.",
            default = devices.keys(),
        ),
    },
)

def _check_app_type_exact(device_id, app_type):
    """Checks if a device explicitly supports a specific app type in its compiler.json.

    Args:
        device_id: The device ID to check.
        app_type: The application type to check for (e.g. "data_field").

    Returns:
        True if the device explicitly supports the specified app type, False otherwise.
    """
    _manifest_xml_type, compiler_json_type = _APP_TYPE_FORMATS[app_type]
    for supported_app_type in devices[device_id]["compiler"]["appTypes"]:
        if supported_app_type["type"] == compiler_json_type:
            return True
    return False

def supports_app_type(device_id, app_type):
    """Checks if a device supports a specific app type.

    Args:
        device_id: The device ID to check.
        app_type: The application type to check for (e.g. "data_field").

    Returns:
        True if the device supports the specified app type, False otherwise.
    """
    if _check_app_type_exact(device_id, app_type):
        return True

    # CIQ 4+ introduces "super apps", which coerce "widget" types into
    # "watchApp" types. Thus, CIQ 4+ devices that support "watchApp" but
    # not "widget" types should be considered to support "widget" types.
    if app_type == _APP_TYPE_WIDGET and supports_min_sdk(device_id, "4.0.0"):
        if _check_app_type_exact(device_id, _APP_TYPE_WATCH_APP):
            return True

    return False

def supports_min_sdk(device_id, min_api_level):
    """Checks if a device supports a minimum API level.

    Args:
        device_id: The device ID to check.
        min_api_level: The minimum API level required (e.g., "3.0.0").

    Returns:
        True if the device supports the specified minimum API level, False otherwise.
    """
    min_api_level_parts = [int(x) for x in min_api_level.split(".")]
    one_device_part_number_supports_min_sdk = False
    for part_number in devices[device_id]["compiler"]["partNumbers"]:
        version = part_number["connectIQVersion"]
        version_parts = [int(x) for x in version.split(".")]
        supports_min_sdk = True
        for i, m in enumerate(min_api_level_parts):
            v = version_parts[i] if len(version_parts) > i else 0
            if v > m:
                break
            if v < m:
                supports_min_sdk = False
                break
        if supports_min_sdk:
            one_device_part_number_supports_min_sdk = True
            break
    return one_device_part_number_supports_min_sdk

ManifestInfo = provider(
    "Provider for Garmin Connect IQ manifest file.",
    fields = {
        "manifest_file": "The manifest.xml file that defines the application metadata and supported devices.",
    },
)

_MANIFEST_TEMPLATE = """
<iq:manifest version="3" xmlns:iq="http://www.garmin.com/xml/connectiq">
    <iq:application
        id="{id}"
        type="{type}"
        entry="{entry}"
        name="@Strings.{name_string_resource_id}"
        minApiLevel="{min_api_level}"
        launcherIcon="@Drawables.{launcher_icon_drawable_resource_id}"
    >
        <iq:products>
            {products}
        </iq:products>
        <iq:permissions>
            {permissions}
        </iq:permissions>
        <iq:languages/>
        <iq:barrels/>
    </iq:application>
</iq:manifest>
"""

def _ciq_manifest_impl(ctx):
    manifest_file = ctx.actions.declare_file(ctx.label.name + ".xml")
    manifest_xml_type, _compiler_json_type = _APP_TYPE_FORMATS[ctx.attr.type]
    products = ""
    for device_id in ctx.attr.device_ids:
        if not supports_app_type(device_id, ctx.attr.type):
            continue
        if not supports_min_sdk(device_id, ctx.attr.min_api_level):
            continue
        products += """<iq:product id="{}"/>""".format(device_id)
    permissions = ""
    for permission in ctx.attr.permissions:
        permissions += """<iq:uses-permission id="{}"/>""".format(permission)
    ctx.actions.write(
        manifest_file,
        """<?xml version="1.0"?>""" + _MANIFEST_TEMPLATE.format(
            id = ctx.attr.id,
            type = manifest_xml_type,
            entry = ctx.attr.entry,
            name_string_resource_id = ctx.attr.name_string_resource_id,
            launcher_icon_drawable_resource_id = ctx.attr.launcher_icon_drawable_resource_id,
            min_api_level = ctx.attr.min_api_level,
            products = products,
            permissions = permissions,
        ),
    )
    return [
        DefaultInfo(
            files = depset([manifest_file]),
        ),
    ]

ciq_manifest = rule(
    implementation = _ciq_manifest_impl,
    doc = "Generates a manifest.xml file for the application.",
    attrs = {
        "id": attr.string(
            doc = "Unique application ID (as a UUID).",
            mandatory = True,
        ),
        "type": attr.string(
            doc = "Application type: 'audio_content_provider', 'data_field', 'watch_face', 'widget', or 'watchApp'.",
            mandatory = True,
            values = _APP_TYPE_FORMATS.keys(),
        ),
        "entry": attr.string(
            doc = "Entry point class name for the application (e.g., 'MyApp').",
            mandatory = True,
        ),
        "name_string_resource_id": attr.string(
            doc = "String resource ID for the application name (e.g., 'AppName').",
            mandatory = True,
        ),
        "launcher_icon_drawable_resource_id": attr.string(
            doc = "Drawable resource ID for the launcher icon.",
            mandatory = True,
        ),
        "min_api_level": attr.string(
            doc = "Minimum Connect IQ API level required (e.g., '3.0.0').",
            mandatory = True,
        ),
        "permissions": attr.string_list(
            doc = "List of permissions required by the application (e.g., ['Positioning', 'Communications']).",
        ),
        "device_ids": attr.string_list(
            doc = "List of device IDs to include in the manifest. Devices are filtered by app type and API level support.",
            default = devices.keys(),
        ),
    },
)

_PROJECT_JUNGLE_TEMPLATE = """
project.manifest = {manifest}
base.sourcePath = nowhere
base.resourcePath = nowhere
base.personality = nowhere
"""

def _ciq_project_impl(ctx):
    name = ctx.label.name
    jungle_file = ctx.actions.declare_file(name + ".jungle")
    ctx.actions.write(
        output = jungle_file,
        content = _PROJECT_JUNGLE_TEMPLATE.format(
            manifest = paths.relativize(ctx.file.manifest.path, jungle_file.dirname),
        ),
    )
    outputs = [jungle_file, ctx.file.manifest]
    all_jungle_files = [jungle_file]
    for jungle in ctx.attr.jungles:
        info = jungle[JunglesInfo]
        all_jungle_files += info.jungle_files
        outputs += jungle.files.to_list()
    return [
        ManifestInfo(
            manifest_file = ctx.file.manifest,
        ),
        JunglesInfo(
            jungle_files = all_jungle_files,
        ),
        DefaultInfo(
            files = depset(outputs),
        ),
    ]

ciq_project = rule(
    implementation = _ciq_project_impl,
    doc = "Defines a Connect IQ project, linking a manifest file with jungle files and resources.",
    attrs = {
        "manifest": attr.label(
            doc = "The manifest.xml file for the project (typically from ciq_manifest).",
            mandatory = True,
            allow_single_file = True,
        ),
        "jungles": attr.label_list(
            doc = "List of jungle assets (typically from ciq_jungle or ciq_scaled_drawable_jungle) containing sources and resources.",
            allow_files = True,
            mandatory = True,
            providers = [JunglesInfo],
        ),
    },
)

DeviceBuildInfo = provider(
    "Provider for Garmin Connect IQ device build outputs.",
    fields = {
        "prg_file": "The compiled .prg file that can be run on the device or simulator.",
        "settings_json_file": "The settings JSON file generated during compilation.",
        "device_id": "The device ID this build was compiled for (e.g., 'fenix6').",
    },
)

def _get_developer_key_path(ctx):
    """Retrieves the Garmin Connect IQ developer key path from the environment.

    Args:
        ctx: The rule context

    Returns:
        The developer key path string

    Fails:
        If the CIQ_DEVELOPER_KEY_PATH environment variable is not set
    """
    env = ctx.configuration.default_shell_env
    developer_key_path = env.get(_CIQ_DEVELOPER_KEY_PATH_ENV_VAR)
    if not developer_key_path:
        fail("{} environment variable unset".format(
            _CIQ_DEVELOPER_KEY_PATH_ENV_VAR,
        ))
    return developer_key_path

def _ciq_device_build_impl(ctx):
    key_path = _get_developer_key_path(ctx)
    sdk_info = ctx.attr.sdk[SdkInfo]
    manifest_info = ctx.attr.project[ManifestInfo]
    jungles_info = ctx.attr.project[JunglesInfo]
    name = ctx.label.name
    prg_file = ctx.actions.declare_file(name + ".prg")
    prg_debug_xml_file = ctx.actions.declare_file(name + ".prg.debug.xml")
    settings_json_file = ctx.actions.declare_file(name + "-settings.json")
    fit_contributions_json_file = ctx.actions.declare_file(name + "-fit_contributions.json")
    release_flag = "-r" if ctx.attr.release else ""
    test_flag = "-t" if ctx.attr.include_tests else ""
    type_check_level_flag = "-l {}".format(ctx.attr.type_check_level) if ctx.attr.type_check_level else ""
    ctx.actions.run_shell(
        execution_requirements = {
            # Allows default.jungle to be written.
            "no-sandbox": "1",
        },
        inputs = ctx.attr.project.files.to_list(),
        outputs = [
            prg_file,
            prg_debug_xml_file,
            settings_json_file,
            fit_contributions_json_file,
        ],
        command = """
      touch {settings_json} && \\
      touch {fit_contributions_json} && \\
      "{executable}" -o "{prg}" -y "{key}" -d {device_id} -f "{jungles}" {release_flag} {test_flag} {type_check_level_flag}
    """.format(
            settings_json = settings_json_file.path,
            fit_contributions_json = fit_contributions_json_file.path,
            executable = sdk_info.monkeyc_path,
            prg = prg_file.path,
            key = key_path,
            device_id = ctx.attr.device_id,
            jungles = ";".join([f.path for f in jungles_info.jungle_files]),
            release_flag = release_flag,
            test_flag = test_flag,
            type_check_level_flag = type_check_level_flag,
        ),
    )
    return [
        sdk_info,
        manifest_info,
        DeviceBuildInfo(
            prg_file = prg_file,
            settings_json_file = settings_json_file,
            device_id = ctx.attr.device_id,
        ),
        DefaultInfo(
            files = depset([
                prg_file,
                prg_debug_xml_file,
                settings_json_file,
                fit_contributions_json_file,
                manifest_info.manifest_file,
            ]),
        ),
    ]

ciq_device_build = rule(
    implementation = _ciq_device_build_impl,
    doc = "Builds the application (.prg) for a specific device.",
    attrs = {
        "project": attr.label(
            doc = "The ciq_project target containing the manifest and jungle assets.",
            mandatory = True,
            providers = [ManifestInfo, JunglesInfo],
            allow_files = True,
        ),
        "release": attr.bool(
            doc = "Build in release mode (optimized, no debug symbols).",
        ),
        "sdk": attr.label(
            doc = "Connect IQ SDK to use for compilation.",
            default = Label("@local_ciq//sdk:current"),
            providers = [SdkInfo],
        ),
        "device_id": attr.string(
            doc = "Target device ID to build for (e.g., 'fenix6').",
            mandatory = True,
        ),
        "include_tests": attr.bool(
            doc = "Include test code in the build.",
        ),
        "type_check_level": attr.int(
            doc = "Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict).",
            values = _TYPE_CHECK_LEVELS,
        ),
    },
)

def _ciq_export_impl(ctx):
    key_path = _get_developer_key_path(ctx)
    sdk_info = ctx.attr.sdk[SdkInfo]
    manifest_info = ctx.attr.project[ManifestInfo]
    jungles_info = ctx.attr.project[JunglesInfo]
    name = ctx.label.name
    iq_file = ctx.actions.declare_file(name + ".iq")
    type_check_level = "-l {}".format(ctx.attr.type_check_level) if ctx.attr.type_check_level else ""
    ctx.actions.run_shell(
        execution_requirements = {
            # Allows default.jungle to be written.
            "no-sandbox": "1",
        },
        inputs = ctx.attr.project.files.to_list(),
        outputs = [iq_file],
        # The manifest file must be chmod'd to be successfully copied by the
        # monkeyc tool.
        command = """
      chmod 644 "{manifest}" &&
      "{executable}" -o "{iq}" -y "{key}" -f "{jungles}" -e -r {type_check_level}
    """.format(
            manifest = manifest_info.manifest_file.path,
            executable = sdk_info.monkeyc_path,
            iq = iq_file.path,
            key = key_path,
            jungles = ";".join([f.path for f in jungles_info.jungle_files]),
            type_check_level = type_check_level,
        ),
    )
    return [
        DefaultInfo(
            files = depset([iq_file]),
        ),
    ]

ciq_export = rule(
    implementation = _ciq_export_impl,
    doc = "Exports the application (.iq) for distribution.",
    attrs = {
        "project": attr.label(
            doc = "The ciq_project target containing the manifest and jungle assets.",
            mandatory = True,
            providers = [ManifestInfo, JunglesInfo],
            allow_files = True,
        ),
        "sdk": attr.label(
            doc = "Connect IQ SDK to use for compilation.",
            default = Label("@local_ciq//sdk:current"),
            providers = [SdkInfo],
        ),
        "type_check_level": attr.int(
            doc = "Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict). Default is 0.",
            values = _TYPE_CHECK_LEVELS,
        ),
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
        source {runfiles_script}
        GET_APPLICATION_ID_TOOL=$(rlocation {get_application_id_tool})
        MANIFEST_XML_PATH=$(rlocation {manifest_xml_path})
        APPLICATION_ID=$($GET_APPLICATION_ID_TOOL $MANIFEST_XML_PATH)
        {simulator_tool} "{simulator_path}" "{shell_path}" "$APPLICATION_ID" "{prg_path}" "{settings_json_path}" {device_id}
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
            default = Label("//tools:get_application_id"),
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
            default = Label("//tools:get_application_id"),
        ),
        "_simulator_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//simulator:test"),
        ),
    },
)

def _calculatePrgPath(prg_file):
    return paths.join("/GARMIN/Apps", prg_file.basename)

def _calculateLogPath(prg_file):
    txt_basename = paths.replace_extension(prg_file.basename, ".TXT")
    return paths.join("/GARMIN/Apps/LOGS", txt_basename)

def _ciq_device_upload_impl(ctx):
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

ciq_device_upload = rule(
    implementation = _ciq_device_upload_impl,
    doc = "Uploads the application (.prg) to a connected physical Garmin device via MTP.",
    executable = True,
    attrs = {
        "device_build": attr.label(
            doc = "The ciq_device_build target to upload.",
            mandatory = True,
            providers = [DeviceBuildInfo],
        ),
        "_mtp_upload_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//mtp:upload"),
        ),
    },
)

def _ciq_device_log_cat_impl(ctx):
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

ciq_device_log_cat = rule(
    implementation = _ciq_device_log_cat_impl,
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
            default = Label("//mtp:download"),
        ),
    },
)

def _ciq_framed_screenshot_impl(ctx):
    device_id = ctx.attr.device_id

    device_metadata = devices[device_id]
    simulator = device_metadata["simulator"]
    
    display = simulator["display"]
    x = display["location"]["x"]
    y = display["location"]["y"]

    image_filename = simulator["image"]
    expected_path_suffix = "devices/{}/{}".format(device_id, image_filename)
    
    bg_file = [f for f in ctx.files._device_files if f.path.endswith(expected_path_suffix)][0]
    output = ctx.actions.declare_file(ctx.label.name + ".png")
    
    ctx.actions.run(
        inputs = [ctx.file.screenshot, bg_file],
        outputs = [output],
        executable = ctx.executable._frame_screenshot_tool,
        arguments = [
            bg_file.path,
            ctx.file.screenshot.path,
            str(x),
            str(y),
            output.path,
        ] + (["--crop-to-screenshot"] if ctx.attr.crop else []),
    )
    
    return [DefaultInfo(files = depset([output]))]

ciq_framed_screenshot = rule(
    implementation = _ciq_framed_screenshot_impl,
    doc = "Superimposes a simulator screenshot into a device background image.",
    attrs = {
        "screenshot": attr.label(
            doc = "The simulator screenshot image file.",
            mandatory = True,
            allow_single_file = True,
        ),
        "device_id": attr.string(
            doc = "The device ID to simulate (e.g. 'fenix6').",
            mandatory = True,
        ),
        "crop": attr.bool(
            doc = "If True, crops the output image to the dimensions of the screenshot with the background superimposed.",
            default = False,
        ),
        "_frame_screenshot_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//tools:frame_screenshot"),
        ),
        "_device_files": attr.label(
            default = Label("@local_ciq//:devices"),
            allow_files = True,
        ),
    },
)

def _ciq_store_image_impl(ctx):
    if len(ctx.files.images) > 1:
        extension = "gif"
    else:
        extension = ctx.files.images[0].extension

    output_file = ctx.actions.declare_file(ctx.label.name + "." + extension)

    args = ctx.actions.args()
    args.add("--output-path", output_file)
    args.add("--max-size-kb", str(ctx.attr.max_size_kb))

    if ctx.attr.transition_millis:
        for t in ctx.attr.transition_millis:
            args.add("--transition-millis", str(t))

    args.add("--")
    args.add_all([f.path for f in ctx.files.images])

    ctx.actions.run(
        inputs = ctx.files.images,
        outputs = [output_file],
        executable = ctx.executable._compose_store_image_tool,
        arguments = [args],
    )

    return [
        DefaultInfo(
            files = depset([output_file]),
        ),
    ]

_ciq_store_image = rule(
    implementation = _ciq_store_image_impl,
    doc = "Private rule to generate a store image.",
    attrs = {
        "images": attr.label_list(
            doc = "List of image files to compose.",
            allow_files = True,
            mandatory = True,
        ),
        "transition_millis": attr.int_list(
            doc = "List of transition durations in milliseconds for the GIF.",
        ),
        "max_size_kb": attr.int(
            doc = "Maximum size of the output image in kilobytes.",
            mandatory = True,
        ),
        "_compose_store_image_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//tools:compose_store_image"),
        ),
    },
)

def ciq_store_image(name, images, max_size_kb, transition_millis = None, **kwargs):
    """Generates a store image (GIF, PNG or JPG) from a list of files with size constraints.

    Args:
        name: The name of the target.
        images: A list of image files to include. An animated GIF will be generated if more than one image is provided.
        max_size_kb: The maximum allowed size for the output image in kilobytes.
        transition_millis: Optional. A single integer or a list of integers representing transition durations in milliseconds.
        **kwargs: Standard Bazel rule arguments (tags, visibility, etc.).
    """
    if transition_millis == None:
        transition_millis_list = []
    elif type(transition_millis) == "int":
        transition_millis_list = [transition_millis]
    elif type(transition_millis) == "list":
        transition_millis_list = transition_millis
    else:
        fail("transition_millis must be an int or a list of ints")

    _ciq_store_image(
        name = name,
        images = images,
        transition_millis = transition_millis_list,
        max_size_kb = max_size_kb,
        **kwargs
    )
