"""
Defines provider and rules for device-dependent metrics.
"""

load("@local_ciq//:defs.bzl", "devices")

DeviceDependentMetricInfo = provider(
    "Provider for device-dependent metrics.",
    fields = {
        "map": "Dictionary of device_id to metric value (int or shell expression string)",
    },
)

def _metric_impl(_ctx, value_map):
    return [
        DeviceDependentMetricInfo(map = value_map),
        DefaultInfo(),
    ]

def _lookup_nested(dictionary, key):
    parts = key.split(".")
    current = dictionary
    for part in parts:
        if type(current) != "dict" or part not in current:
            return None
        current = current[part]
    return current

def _lookup_metric_impl(ctx):
    value_map = {}
    for device_id, device_metadata in devices.items():
        if device_id not in ctx.attr.device_ids:
            continue
        val = _lookup_nested(device_metadata, ctx.attr.key)
        if val != None:
            value_map[device_id] = val
    return _metric_impl(ctx, value_map)

lookup_metric = rule(
    implementation = _lookup_metric_impl,
    attrs = {
        "key": attr.string(mandatory = True),
        "device_ids": attr.string_list(default = devices.keys()),
    },
)

def _min_screen_dimension_metric_impl(ctx):
    value_map = {}
    for device_id, device_metadata in devices.items():
        if device_id not in ctx.attr.device_ids:
            continue
        width = _lookup_nested(device_metadata, "compiler.resolution.width")
        height = _lookup_nested(device_metadata, "compiler.resolution.height")
        if width != None and height != None:
            value_map[device_id] = min(width, height)
    return _metric_impl(ctx, value_map)

min_screen_dimension_metric = rule(
    implementation = _min_screen_dimension_metric_impl,
    attrs = {
        "device_ids": attr.string_list(default = devices.keys()),
    },
)

def _filename_without_extension(file):
    return file.basename[0:file.basename.rindex(".")]

def _get_sdk_font_height(ctx, device_metadata):
    simulator = device_metadata["simulator"]
    matched_font_sets = [
        font
        for font in simulator["fonts"]
        if font["fontSet"] == ctx.attr.sdk_font_set
    ]
    if len(matched_font_sets) != 1:
        # It's possible for a device to not have the requested font set.
        # In this case we just don't provide a metric for this device.
        return None

    fonts = matched_font_sets[0]["fonts"]
    matched_fonts = [
        font
        for font in fonts
        if font["name"] == ctx.attr.sdk_font_name
    ]
    if len(matched_fonts) != 1:
        # Similarly, the font might not exist in the set.
        return None
    font = matched_fonts[0]

    if "type" in font and font["type"] == "ttf":
        return int(font["size"] * simulator["ppi"] / 72)
    else:
        # Need to find the font file in _fonts
        matched_font_files = [
            file
            for file in ctx.attr._fonts.files.to_list()
            if _filename_without_extension(file) == font["filename"]
        ]
        if len(matched_font_files) == 0:
            # Skip generation for devices with broken font references.
            return None
        font_file_ref = matched_font_files[0]
        return """$({tool} "{font_path}")""".format(
            tool = ctx.executable._measure_cft_tool.path,
            font_path = font_file_ref.path,
        )

def _sdk_font_metric_impl(ctx):
    value_map = {}
    for device_id, device_metadata in devices.items():
        if device_id not in ctx.attr.device_ids:
            continue
        val = _get_sdk_font_height(ctx, device_metadata)
        if val != None:
            value_map[device_id] = val

    return _metric_impl(ctx, value_map)

sdk_font_metric = rule(
    implementation = _sdk_font_metric_impl,
    attrs = {
        "sdk_font_name": attr.string(mandatory = True),
        "sdk_font_set": attr.string(default = "ww"),
        "device_ids": attr.string_list(default = devices.keys()),
        "_fonts": attr.label(default = Label("@local_ciq//:fonts")),
        "_measure_cft_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//build:measure_cft"),
        ),
    },
)
