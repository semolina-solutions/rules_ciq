"""
Rules for generating assets for the Garmin Connect IQ Store.
"""

load("@local_ciq//:defs.bzl", "devices")

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
    
    screenshot_args = []
    for f in ctx.files.screenshots:
        screenshot_args += ["--screenshot", f.path]

    ctx.actions.run(
        inputs = ctx.files.screenshots + [bg_file],
        outputs = [output],
        executable = ctx.executable._frame_screenshot_tool,
        arguments = [
            bg_file.path,
            str(x),
            str(y),
            output.path,
        ] + screenshot_args +
        (["--background-color", ctx.attr.background_color] if ctx.attr.background_color else []) +
        (["--crop-to-screenshot"] if ctx.attr.crop else []),
    )
    
    return [DefaultInfo(files = depset([output]))]

ciq_framed_screenshot = rule(
    implementation = _ciq_framed_screenshot_impl,
    doc = """Superimposes one or more simulator screenshots into a device background image.

    Multiple screenshots are composited in list order (first = bottom-most) before the
    device frame is applied on top. This allows transparent-field-extracted PNGs from
    ciq_field_screenshot to be layered together into a single composite view.

    Example:
        ciq_framed_screenshot(
            name = "fr970_composite_framed",
            device_id = "fr970",
            screenshots = [
                ":fr970_field_z2",
                ":fr970_field_z4",
                ":fr970_field_z5",
            ],
        )
    """,
    attrs = {
        "screenshots": attr.label_list(
            doc = "One or more simulator screenshot image files to composite (in order, first = bottom-most layer).",
            mandatory = True,
            allow_files = True,
        ),
        "device_id": attr.string(
            doc = "The device ID to simulate (e.g. 'fenix6').",
            mandatory = True,
        ),
        "crop": attr.bool(
            doc = "If True, crops the output to the dimensions of the first screenshot with the background superimposed.",
            default = False,
        ),
        "background_color": attr.string(
            doc = "Optional background fill colour as a hex string (#RRGGBB or #RRGGBBAA). "
                  + "Applied before screenshots are composited. If unset, unfilled areas are transparent.",
            default = "",
        ),
        "_frame_screenshot_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//store:frame_screenshot"),
        ),
        "_device_files": attr.label(
            default = Label("@local_ciq//:devices"),
            allow_files = True,
        ),
    },
)

def _find_field_location(simulator, layout_name, field_index):
    """Returns the (x, y, width, height) of a field within a named layout.

    Walks simulator.layouts[*].datafields.datafields and finds the first
    layout whose "name" matches layout_name, then returns the location of
    the field at field_index (0-based) within that layout's "fields" list.

    Returns None if the layout or field index is not found.
    """
    for layout_set in simulator.get("layouts", []):
        for df in layout_set.get("datafields", {}).get("datafields", []):
            if df.get("name") != layout_name:
                continue
            fields = df.get("fields", [])
            if field_index < 0 or field_index >= len(fields):
                fail("field_index {} is out of range for layout '{}' which has {} fields".format(
                    field_index, layout_name, len(fields),
                ))
            loc = fields[field_index].get("location", {})
            return (
                loc.get("x", 0),
                loc.get("y", 0),
                loc.get("width"),
                loc.get("height"),
            )
    return None

def _ciq_field_screenshot_impl(ctx):
    device_id = ctx.attr.device_id
    device_metadata = devices[device_id]
    simulator = device_metadata["simulator"]

    loc = _find_field_location(simulator, ctx.attr.layout, ctx.attr.field_index)
    if loc == None:
        fail("Layout '{}' not found in simulator.json for device '{}'".format(
            ctx.attr.layout, device_id,
        ))

    x, y, w, h = loc
    if w == None or h == None:
        fail("Layout '{}' field {} has no location dimensions in simulator.json for device '{}'".format(
            ctx.attr.layout, ctx.attr.field_index, device_id,
        ))

    output = ctx.actions.declare_file(ctx.label.name + ".png")

    ctx.actions.run(
        inputs = [ctx.file.screenshot],
        outputs = [output],
        executable = ctx.executable._extract_field_tool,
        arguments = [
            ctx.file.screenshot.path,
            output.path,
            "--x", str(x),
            "--y", str(y),
            "--width", str(w),
            "--height", str(h),
        ],
    )

    return [DefaultInfo(files = depset([output]))]

ciq_field_screenshot = rule(
    implementation = _ciq_field_screenshot_impl,
    doc = """Extracts a single data field rectangle from a simulator screenshot.

    Looks up the field's pixel rectangle from the device's simulator.json layout
    metadata (matched by layout name and 0-based field index), then produces an
    output PNG of the same dimensions as the input with all pixels outside the
    field rectangle set to fully transparent.

    Example:
        ciq_field_screenshot(
            name = "fr945_zone2_field1",
            screenshot = ":fr945_3_Fields_A_zone_2.png",
            device_id = "fr945",
            layout = "3 Fields A",
            field_index = 1,
        )
    """,
    attrs = {
        "screenshot": attr.label(
            doc = "The simulator screenshot PNG to extract from.",
            mandatory = True,
            allow_single_file = True,
        ),
        "device_id": attr.string(
            doc = "The device ID (e.g. 'fr945') used to look up layout metadata.",
            mandatory = True,
        ),
        "layout": attr.string(
            doc = "The layout name as it appears in simulator.json (e.g. '3 Fields A').",
            mandatory = True,
        ),
        "field_index": attr.int(
            doc = "0-based index of the field within the named layout's 'fields' array.",
            mandatory = True,
        ),
        "_extract_field_tool": attr.label(
            executable = True,
            cfg = "exec",
            default = Label("//store:extract_field"),
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
            default = Label("//store:compose_store_image"),
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
