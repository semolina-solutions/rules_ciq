"""Demonstrates how to generate custom fonts.

Assuming a "fenix6" device:

Simulate:
`bazel run @rules_ciq//samples/custom_font:sample_fenix6_debug_simulation`
"""

load(
    "@rules_ciq//:defs.bzl",
    "ciq_jungle",
    "ciq_manifest",
    "ciq_project",
    "ciq_scaled_bmfont_jungle",
    "ciq_scaled_drawable_jungle",
    "devices",
    "supports_app_type",
)
load("@rules_ciq//:macros.bzl", "ciq_device_targets_macro")

# A custom font sized to 50% of the screen width or height, whichever is smaller.
ciq_scaled_bmfont_jungle(
    name = "screen_scaled_font",
    anti_alias = True,
    chars = "Hi",
    device_dependent_height = "//metrics:screen_minimum_dimension",
    font = "Micro5/Micro5-Regular.ttf",
    percent = 50,
    reference_chars = "A",
    resource_id = "ScreenScaledFont",
)

# A custom font sized to SDK font "xtiny" but snapped to a multiple that matches the font.
ciq_scaled_bmfont_jungle(
    name = "xtiny_snapped_font",
    anti_alias = False,
    chars = "SNAPPED",
    device_dependent_height = "//metrics:xtiny_font_height",
    font = "Micro5/Micro5-Regular.ttf",
    reference_chars = "A",
    resource_id = "XtinySnappedFont",
    snap = 5,
)

LAUNCHER_ICON_RESOURCE_ID = "LauncherIcon"

ciq_manifest(
    name = "sample_manifest",
    entry = "SampleApp",
    id = "00000000-0000-0000-0000-000000000001",
    launcher_icon_drawable_resource_id = LAUNCHER_ICON_RESOURCE_ID,
    min_api_level = "1.0.0",
    name_string_resource_id = "SampleAppName",
    type = "widget",
)

ciq_jungle(
    name = "sample_jungle",
    resources = glob(["*.xml"]),
    sources = glob(["*.mc"]),
)

ciq_scaled_drawable_jungle(
    name = "scaled_icon",
    src = "//samples:icon.png",
    device_dependent_height = "//metrics:launcher_icon_height",
    device_dependent_width = "//metrics:launcher_icon_width",
    resource_id = LAUNCHER_ICON_RESOURCE_ID,
)

ciq_project(
    name = "sample_project",
    jungles = [
        ":sample_jungle",
        ":scaled_icon",
        ":screen_scaled_font",
        ":xtiny_snapped_font",
    ],
    manifest = ":sample_manifest",
)

DEVICE_IDS_SUPPORTING_WIDGET = [
    device_id
    for device_id in devices
    if supports_app_type(device_id, "widget")
]

ciq_device_targets_macro(
    name = "sample",
    device_ids = DEVICE_IDS_SUPPORTING_WIDGET,
    project = ":sample_project",
)
