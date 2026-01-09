"""
Bazel rules and providers for building Garmin Connect IQ applications.

This module defines custom Bazel rules for building Connect IQ projects, including:
- Jungle file generation for device-specific resources and sources
- Manifest generation with device filtering based on API level and app type
- Device builds and exports using the Monkey C compiler
"""

load("@bazel_skylib//lib:paths.bzl", "paths")
load("@local_ciq//:defs.bzl", "devices")
load("@local_ciq//sdk:defs.bzl", "SdkInfo")
load("//metrics:defs.bzl", "DeviceDependentMetricInfo")

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

        source_path = None
        resource_path = None

        for f in generated_files:
            if device_sources_dir in f.short_path and not source_path:
                source_path = paths.relativize(f.dirname, jungle_file.dirname)
            if device_resources_dir in f.short_path and not resource_path:
                resource_path = paths.relativize(f.dirname, jungle_file.dirname)

        if source_path:
            jungle_content += "{id}.sourcePath = $({id}.sourcePath);{dir}\n".format(
                id = device_id,
                dir = source_path,
            )

        if resource_path:
            jungle_content += "{id}.resourcePath = $({id}.resourcePath);{dir}\n".format(
                id = device_id,
                dir = resource_path,
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

def _ciq_scaled_drawable_generator(ctx, device_id, _device_metadata, _sources_dir, resources_dir):
    width = None
    height = None

    if ctx.attr.device_dependent_width:
        width = ctx.attr.device_dependent_width[DeviceDependentMetricInfo].map.get(device_id)
        if width == None:
            return None
    if ctx.attr.device_dependent_height:
        height = ctx.attr.device_dependent_height[DeviceDependentMetricInfo].map.get(device_id)
        if height == None:
            return None

    image_file = ctx.actions.declare_file(
        paths.join(resources_dir, ctx.file.src.basename),
    )

    cmd_args = [
        ctx.executable._scale_image_tool.path,
        ctx.file.src.path,
        image_file.path,
    ]
    if width != None:
        cmd_args.insert(0, "UNSCALED_WIDTH={};".format(width))
        cmd_args.insert(1, 'SCALED_WIDTH="$({tool} $UNSCALED_WIDTH {percent})";'.format(
            tool = ctx.executable._scale_value_tool.path,
            percent = ctx.attr.percent,
        ))
        cmd_args.append("--width $SCALED_WIDTH")
    if height != None:
        cmd_args.insert(0, "UNSCALED_HEIGHT={};".format(height))
        cmd_args.insert(1, "SCALED_HEIGHT=$({tool} $UNSCALED_HEIGHT {percent});".format(
            tool = ctx.executable._scale_value_tool.path,
            percent = ctx.attr.percent,
        ))
        cmd_args.append("--height $SCALED_HEIGHT")

    metric_files = []
    if ctx.attr.device_dependent_width:
        metric_files.append(ctx.attr.device_dependent_width[DefaultInfo].files)
    if ctx.attr.device_dependent_height:
        metric_files.append(ctx.attr.device_dependent_height[DefaultInfo].files)

    ctx.actions.run_shell(
        inputs = [ctx.file.src] + ctx.attr._fonts.files.to_list(),
        outputs = [image_file],
        tools = [
            ctx.executable._scale_image_tool,
            ctx.executable._scale_value_tool,
        ] + depset(transitive = metric_files).to_list(),
        command = " ".join(cmd_args),
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
        "device_dependent_width": attr.label(
            doc = "Metric target to use for width scaling.",
            providers = [DeviceDependentMetricInfo],
        ),
        "device_dependent_height": attr.label(
            doc = "Metric target to use for height scaling.",
            providers = [DeviceDependentMetricInfo],
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
        "_scale_image_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:scale_image"),
        ),
        "_scale_value_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:scale_value"),
        ),
    },
)

_FONTS_XML_TEMPLATE = """
<fonts>
    <font id="{id}" filename="{fnt_filename}" filter="" antialias="{antialias}" />
</fonts>
"""

def _ciq_bmfont_generator(ctx, device_id, _device_metadata, _sources_dir, resources_dir):
    height = ctx.attr.device_dependent_height[DeviceDependentMetricInfo].map.get(device_id)
    if height == None:
        # No metric for this device, skip generation.
        return None

    output_base = paths.join(resources_dir, ctx.label.name)
    output_fnt = ctx.actions.declare_file(output_base + ".fnt")
    output_png = ctx.actions.declare_file(output_base + ".png")
    output_fonts_xml = ctx.actions.declare_file(paths.join(resources_dir, "fonts.xml"))

    cmd = [
        "UNSCALED_HEIGHT={};".format(height),
        "SCALED_HEIGHT={};".format("$({tool} $UNSCALED_HEIGHT {percent} --snap {snap})".format(
            tool = ctx.executable._scale_value_tool.path,
            percent = ctx.attr.percent,
            snap = ctx.attr.snap,
        )),
        ctx.executable._generate_bmfont_tool.path,
        ctx.file.font.path,
        output_fnt.path[:-4],
        "$SCALED_HEIGHT",
    ]
    if ctx.attr.chars:
        cmd.append("--chars")
        cmd.append('"{}"'.format(ctx.attr.chars))
    if ctx.attr.reference_chars:
        cmd.append("--reference-chars")
        cmd.append('"{}"'.format(ctx.attr.reference_chars))
    if ctx.attr.anti_alias:
        cmd.append("--anti-alias")

    inputs = [
        ctx.file.font,
        ctx.executable._scale_value_tool,
    ] + ctx.attr._fonts.files.to_list()
    tools = [
        ctx.executable._scale_value_tool,
        ctx.executable._generate_bmfont_tool,
    ] + ctx.attr.device_dependent_height[DefaultInfo].files.to_list()

    ctx.actions.run_shell(
        inputs = inputs,
        outputs = [output_fnt, output_png],
        tools = tools,
        command = " ".join(cmd),
    )

    ctx.actions.write(
        output = output_fonts_xml,
        content = _FONTS_XML_TEMPLATE.format(
            id = ctx.attr.resource_id,
            fnt_filename = output_fnt.basename,
            antialias = "true" if ctx.attr.anti_alias else "false",
        ),
    )

    return [output_fnt, output_png, output_fonts_xml]

def _ciq_bmfont_jungle_impl(ctx):
    return jungle_generator(ctx, _ciq_bmfont_generator, ctx.attr.device_ids)

ciq_scaled_bmfont_jungle = rule(
    implementation = _ciq_bmfont_jungle_impl,
    doc = "Generates a BMFont (.fnt and .png) and fonts.xml from a TrueType/OpenType font for specific devices.",
    attrs = {
        "font": attr.label(
            doc = "Input font file (.ttf or .otf).",
            allow_single_file = True,
            mandatory = True,
        ),
        "resource_id": attr.string(
            doc = "Resource ID to use in the generated fonts.xml file.",
            mandatory = True,
        ),
        "chars": attr.string(
            doc = "Characters to include in the font. Defaults to a standard ASCII set if unspecified.",
        ),
        "reference_chars": attr.string(
            doc = "String of characters to use as a height reference for scaling. If unspecified, no additional scaling is applied.",
        ),
        "device_dependent_height": attr.label(
            doc = "Metric target to use for height scaling.",
            mandatory = True,
            providers = [DeviceDependentMetricInfo],
        ),
        "percent": attr.int(
            doc = "Percentage of the base size to scale to (100 = original size).",
            default = 100,
        ),
        "anti_alias": attr.bool(
            doc = "Enable anti-aliasing.",
            default = False,
        ),
        "snap": attr.int(
            doc = "Pixel multiple to snap scaled font to.",
            default = 1,
        ),
        "device_ids": attr.string_list(
            doc = "List of device IDs to generate font resources for.",
            default = devices.keys(),
        ),
        "_fonts": attr.label(
            default = Label("@local_ciq//:fonts"),
        ),
        "_generate_bmfont_tool": attr.label(
            default = Label("//build:generate_bmfont"),
            executable = True,
            cfg = "exec",
        ),
        "_scale_value_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:scale_value"),
        ),
    },
)

def _ciq_jungle_impl(ctx):
    jungle_file = ctx.actions.declare_file(ctx.label.name + ".jungle")
    outputs = [jungle_file]

    sources_dir_root = paths.join(ctx.label.name, "sources")
    source_basenames = {}
    root_source_path = None

    for source_file in ctx.files.sources:
        basename = source_file.basename
        count = source_basenames.get(basename, 0)
        source_basenames[basename] = count + 1

        if count == 0:
            target_dir = sources_dir_root
        else:
            target_dir = paths.join(sources_dir_root, str(count))

        symlinked_source_file = ctx.actions.declare_file(
            paths.join(target_dir, basename),
        )
        outputs.append(symlinked_source_file)
        ctx.actions.symlink(
            output = symlinked_source_file,
            target_file = source_file,
        )

        if count == 0:
            root_source_path = paths.relativize(symlinked_source_file.dirname, jungle_file.dirname)

    resources_dir_root = paths.join(ctx.label.name, "resources")
    resource_basenames = {}
    root_resource_path = None

    for resource_file in ctx.files.resources:
        basename = resource_file.basename
        count = resource_basenames.get(basename, 0)
        resource_basenames[basename] = count + 1

        if count == 0:
            target_dir = resources_dir_root
        else:
            target_dir = paths.join(resources_dir_root, str(count))

        symlinked_resource_file = ctx.actions.declare_file(
            paths.join(target_dir, basename),
        )
        outputs.append(symlinked_resource_file)
        ctx.actions.symlink(
            output = symlinked_resource_file,
            target_file = resource_file,
        )

        if count == 0:
            root_resource_path = paths.relativize(symlinked_resource_file.dirname, jungle_file.dirname)

    jungle_content = ""

    for device_id in ctx.attr.device_ids:
        if root_source_path:
            jungle_content += "{id}.sourcePath = $({id}.sourcePath);{dir}\n".format(
                id = device_id,
                dir = root_source_path,
            )
        if root_resource_path:
            jungle_content += "{id}.resourcePath = $({id}.resourcePath);{dir}\n".format(
                id = device_id,
                dir = root_resource_path,
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
        "prg_debug_xml_file": "The debug XML file generated during compilation.",
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
    flags = []
    if ctx.attr.release:
        flags.append("-r")
    if ctx.attr.testing:
        flags.append("-t")
    if ctx.attr.profiling:
        flags.append("-k")
    if ctx.attr.type_check_level:
        flags.append("-l {}".format(ctx.attr.type_check_level))
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
      "{executable}" -o "{prg}" -y "{key}" -d {device_id} -f "{jungles}" {flags}
    """.format(
            settings_json = settings_json_file.path,
            fit_contributions_json = fit_contributions_json_file.path,
            executable = sdk_info.monkeyc_path,
            prg = prg_file.path,
            key = key_path,
            device_id = ctx.attr.device_id,
            jungles = ";".join([f.path for f in jungles_info.jungle_files]),
            flags = " ".join(flags),
        ),
    )
    return [
        sdk_info,
        manifest_info,
        DeviceBuildInfo(
            prg_file = prg_file,
            prg_debug_xml_file = prg_debug_xml_file,
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
        "sdk": attr.label(
            doc = "Connect IQ SDK to use for compilation.",
            default = Label("@local_ciq//sdk:current"),
            providers = [SdkInfo],
        ),
        "device_id": attr.string(
            doc = "Target device ID to build for (e.g., 'fenix6').",
            mandatory = True,
        ),
        "release": attr.bool(
            doc = "Build in release mode (optimized, no debug symbols).",
        ),
        "testing": attr.bool(
            doc = "Include tests in the build.",
        ),
        "profiling": attr.bool(
            doc = "Enable profiling in the build.",
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
