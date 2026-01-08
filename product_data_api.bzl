"""Repository rule for fetching Garmin product data."""

BUILD_CONTENT = """
filegroup(
    name = "product_data",
    srcs = glob(["product_data/**/*.json"]),
    visibility = ["//visibility:public"],
)

filegroup(
    name = "products",
    srcs = glob(["products_*.json"]),
    visibility = ["//visibility:public"],
)
"""

LOCALES = ["en-US", "en-GB"]
GET_PRODUCT_DATA_API_BATCH_SIZE = 5

def _normalize_pn(pn):
    """Normalizes a part number by stripping the suffix and appending -00.

    This handles cases where the API returns a localized part number (e.g. -10, -11)
    but we want to match against the base part number (usually -00).

    Args:
        pn: The part number string to normalize.

    Returns:
        The normalized part number string ending in -00, or the original string if no dash is found.
    """
    last_dash = pn.rfind("-")
    if last_dash != -1:
        return pn[:last_dash] + "-00"
    return pn

def _product_data_api_impl(ctx):
    ctx.file("BUILD.bazel", BUILD_CONTENT)

    devices_json = ctx.attr.devices_json
    devices = json.decode(ctx.read(devices_json))

    # Map normalized hardwarePartNumber to dict of original HPNs and device IDs
    target_part_numbers = {}

    for device_id, device_data in devices.items():
        hpn = device_data["compiler"]["hardwarePartNumber"]
        norm_hpn = _normalize_pn(hpn)
        if norm_hpn not in target_part_numbers:
            target_part_numbers[norm_hpn] = []
        target_part_numbers[norm_hpn].append(device_id)

    product_id_map = {}

    ctx.report_progress("Searching for local devices across locales {}...".format(LOCALES))

    for locale in LOCALES:
        products_filename = "products_{}.json".format(locale)
        ctx.download(
            url = "https://www.garmin.com/compare/api/getDisplayableProducts?locale={}".format(locale),
            output = products_filename,
        )

        content = ctx.read(products_filename)
        products = json.decode(content)

        for p in products:
            pn = p["partNumber"]
            norm_pn = _normalize_pn(pn)

            if norm_pn in target_part_numbers:
                if locale not in product_id_map:
                    product_id_map[locale] = {}
                product_id_map[locale][norm_pn] = p["productId"]

    # Calculate total unique devices found for progress reporting
    found_unique = {}
    for locale_map in product_id_map.values():
        for hpn in locale_map:
            found_unique[hpn] = True

    total_found = len(found_unique)
    ctx.report_progress("Fetching detailed data for {} found devices...".format(total_found))

    for locale, device_map in product_id_map.items():
        pids = device_map.values()

        # Chunk products into batches.
        total_pids = len(pids)
        for i in range(0, total_pids, GET_PRODUCT_DATA_API_BATCH_SIZE):
            batch_ids = pids[i:i + GET_PRODUCT_DATA_API_BATCH_SIZE]
            query_params = ["pId={}".format(pid) for pid in batch_ids]

            url = "https://www.garmin.com/compare/api/getProductDataByPids?locale={}&{}".format(
                locale,
                "&".join(query_params),
            )

            output_file = "product_data/{}_batch_{}.json".format(locale, i // GET_PRODUCT_DATA_API_BATCH_SIZE)
            ctx.download(
                url = url,
                output = output_file,
            )

product_data_api = repository_rule(
    implementation = _product_data_api_impl,
    attrs = {
        "devices_json": attr.label(
            allow_single_file = True,
            doc = "The devices.json file from local_ciq containing the devices dict.",
        ),
    },
    # This rule downloads content, so it shouldn't be local.
    local = False,
)
