"""Macros for the device parametrics sample."""

load("@bazel_skylib//lib:paths.bzl", "paths")
load("//:defs.bzl", "ciq_scaled_drawable_jungle", "jungle_generator")

def build_font_height_scaled_symbol_jungles():
    """
    Generates ciq_scaled_drawable_jungle targets for a predefined list of fonts.

    Returns:
        A list of the generated target names to be included in the project.
    """
    targets = []
    for font in ["tiny", "large"]:
        name = "font_height_scaled_symbol_{}".format(font)
        targets.append(name)
        ciq_scaled_drawable_jungle(
            name = name,
            src = ":symbol.png",
            font_name = font,
            mode = "font_height",
            resource_id = "FontHeightScaledSymbol_{}".format(font),
        )
    return targets

def _device_parameterized_source_generator(ctx, _device_id, device_metadata, sources_dir, _resources_dir):
    # Extract something that might be useful from the device's metadata.
    display_type = device_metadata["compiler"]["displayType"]

    # Declare (for Bazel's analysis phase) a file to be generated.
    mc_file = ctx.actions.declare_file(paths.join(sources_dir, "generated.mc"))

    # Generate the file (done during Bazel's execution phase).
    ctx.actions.write(
        output = mc_file,
        content = """module GeneratedCode {{ const DISPLAY_TYPE = "{}"; }}""".format(display_type),
    )

    return [mc_file]

def _device_parameterized_source_jungle_impl(ctx):
    return jungle_generator(ctx, _device_parameterized_source_generator)

device_parameterized_source_jungle = rule(
    implementation = _device_parameterized_source_jungle_impl,
)
