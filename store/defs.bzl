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
            default = Label("//store:frame_screenshot"),
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
