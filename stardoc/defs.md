<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Public API for rules_ciq.

This module re-exports rules and providers from the various sub-packages
(build, device, simulator, store) for convenience.

<a id="ciq_device_build"></a>


<pre>
ciq_device_build(<a href="#ciq_device_build-name">name</a>, <a href="#ciq_device_build-device_id">device_id</a>, <a href="#ciq_device_build-profiling">profiling</a>, <a href="#ciq_device_build-project">project</a>, <a href="#ciq_device_build-release">release</a>, <a href="#ciq_device_build-testing">testing</a>, <a href="#ciq_device_build-type_check_level">type_check_level</a>)
</pre>

Builds the application (.prg) for a specific device.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_device_build-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_device_build-device_id"></a>device_id |  Target device ID to build for (e.g., 'fenix6').  | STRING | true |    |
| <a id="ciq_device_build-profiling"></a>profiling |  Enable profiling in the build.  | BOOLEAN | false |  False  |
| <a id="ciq_device_build-project"></a>project |  The ciq_project target containing the manifest, jungle assets, and target SDK.  | LABEL | true |    |
| <a id="ciq_device_build-release"></a>release |  Build in release mode (optimized, no debug symbols).  | BOOLEAN | false |  False  |
| <a id="ciq_device_build-testing"></a>testing |  Include tests in the build.  | BOOLEAN | false |  False  |
| <a id="ciq_device_build-type_check_level"></a>type_check_level |  Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict).  | INT | false |  0  |


<a id="ciq_export"></a>


<pre>
ciq_export(<a href="#ciq_export-name">name</a>, <a href="#ciq_export-project">project</a>, <a href="#ciq_export-type_check_level">type_check_level</a>)
</pre>

Exports the application (.iq) for distribution.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_export-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_export-project"></a>project |  The ciq_project target containing the manifest, jungle assets, and target SDK.  | LABEL | true |    |
| <a id="ciq_export-type_check_level"></a>type_check_level |  Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict). Default is 0.  | INT | false |  0  |


<a id="ciq_field_screenshot"></a>


<pre>
ciq_field_screenshot(<a href="#ciq_field_screenshot-name">name</a>, <a href="#ciq_field_screenshot-device_id">device_id</a>, <a href="#ciq_field_screenshot-field_index">field_index</a>, <a href="#ciq_field_screenshot-layout">layout</a>, <a href="#ciq_field_screenshot-screenshot">screenshot</a>)
</pre>

Extracts a single data field rectangle from a simulator screenshot.

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

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_field_screenshot-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_field_screenshot-device_id"></a>device_id |  The device ID (e.g. 'fr945') used to look up layout metadata.  | STRING | true |    |
| <a id="ciq_field_screenshot-field_index"></a>field_index |  0-based index of the field within the named layout's 'fields' array.  | INT | true |    |
| <a id="ciq_field_screenshot-layout"></a>layout |  The layout name as it appears in simulator.json (e.g. '3 Fields A').  | STRING | true |    |
| <a id="ciq_field_screenshot-screenshot"></a>screenshot |  The simulator screenshot PNG to extract from.  | LABEL | true |    |


<a id="ciq_framed_screenshot"></a>


<pre>
ciq_framed_screenshot(<a href="#ciq_framed_screenshot-name">name</a>, <a href="#ciq_framed_screenshot-background_color">background_color</a>, <a href="#ciq_framed_screenshot-crop">crop</a>, <a href="#ciq_framed_screenshot-device_id">device_id</a>, <a href="#ciq_framed_screenshot-screenshots">screenshots</a>)
</pre>

Superimposes one or more simulator screenshots into a device background image.

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

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_framed_screenshot-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_framed_screenshot-background_color"></a>background_color |  Optional background fill colour as a hex string (#RRGGBB or #RRGGBBAA). Applied before screenshots are composited. If unset, unfilled areas are transparent.  | STRING | false |  ""  |
| <a id="ciq_framed_screenshot-crop"></a>crop |  If True, crops the output to the dimensions of the first screenshot with the background superimposed.  | BOOLEAN | false |  False  |
| <a id="ciq_framed_screenshot-device_id"></a>device_id |  The device ID to simulate (e.g. 'fenix6').  | STRING | true |    |
| <a id="ciq_framed_screenshot-screenshots"></a>screenshots |  One or more simulator screenshot image files to composite (in order, first = bottom-most layer).  | LABEL_LIST | true |    |


<a id="ciq_jungle"></a>


<pre>
ciq_jungle(<a href="#ciq_jungle-name">name</a>, <a href="#ciq_jungle-resources">resources</a>, <a href="#ciq_jungle-device_ids">device_ids</a>, <a href="#ciq_jungle-sources">sources</a>)
</pre>

Generates a jungle file mapping sources and resources for specific devices.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_jungle-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_jungle-resources"></a>resources |  List of resource files (e.g., .xml, images) to include in the jungle file.  | LABEL_LIST | false |  []  |
| <a id="ciq_jungle-device_ids"></a>device_ids |  List of device IDs to generate jungle entries for.  | STRING_LIST | false |  *All devices*  |
| <a id="ciq_jungle-sources"></a>sources |  List of source files (.mc) to include in the jungle file.  | LABEL_LIST | false |  []  |


<a id="ciq_log_test"></a>


<pre>
ciq_log_test(<a href="#ciq_log_test-name">name</a>, <a href="#ciq_log_test-expect_logs">expect_logs</a>, <a href="#ciq_log_test-forbid_logs">forbid_logs</a>, <a href="#ciq_log_test-log">log</a>)
</pre>

Verifies that an application log file (from ciq_automation) matches expected and forbidden substring/regex patterns.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_log_test-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_log_test-expect_logs"></a>expect_logs |  List of substrings or regex patterns that must appear in the log file.  | STRING_LIST | false |  []  |
| <a id="ciq_log_test-forbid_logs"></a>forbid_logs |  List of substrings or regex patterns that must not appear in the log file.  | STRING_LIST | false |  []  |
| <a id="ciq_log_test-log"></a>log |  The .log file (e.g., ':my_automation.log') or ciq_automation target to assert against.  | LABEL | true |    |


<a id="ciq_scaled_bmfont_jungle"></a>


<pre>
ciq_scaled_bmfont_jungle(<a href="#ciq_scaled_bmfont_jungle-name">name</a>, <a href="#ciq_scaled_bmfont_jungle-anti_alias">anti_alias</a>, <a href="#ciq_scaled_bmfont_jungle-chars">chars</a>, <a href="#ciq_scaled_bmfont_jungle-device_dependent_height">device_dependent_height</a>, <a href="#ciq_scaled_bmfont_jungle-device_ids">device_ids</a>, <a href="#ciq_scaled_bmfont_jungle-font">font</a>, <a href="#ciq_scaled_bmfont_jungle-font_resource_id">font_resource_id</a>, <a href="#ciq_scaled_bmfont_jungle-metrics_resource_id">metrics_resource_id</a>, <a href="#ciq_scaled_bmfont_jungle-percent">percent</a>, <a href="#ciq_scaled_bmfont_jungle-reference_chars">reference_chars</a>, <a href="#ciq_scaled_bmfont_jungle-snap">snap</a>, <a href="#ciq_scaled_bmfont_jungle-variations">variations</a>)
</pre>

Generates a BMFont (.fnt and .png) and fonts.xml from a TrueType/OpenType font for specific devices.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_scaled_bmfont_jungle-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_scaled_bmfont_jungle-anti_alias"></a>anti_alias |  Enable anti-aliasing.  | BOOLEAN | false |  False  |
| <a id="ciq_scaled_bmfont_jungle-chars"></a>chars |  Characters to include in the font. Defaults to a standard ASCII set if unspecified.  | STRING | false |  ""  |
| <a id="ciq_scaled_bmfont_jungle-device_dependent_height"></a>device_dependent_height |  Metric target to use for height scaling.  | LABEL | true |    |
| <a id="ciq_scaled_bmfont_jungle-device_ids"></a>device_ids |  List of device IDs to generate font resources for.  | STRING_LIST | false |  *All devices*  |
| <a id="ciq_scaled_bmfont_jungle-font"></a>font |  Input font file (.ttf or .otf).  | LABEL | true |    |
| <a id="ciq_scaled_bmfont_jungle-font_resource_id"></a>font_resource_id |  Resource ID to use in the generated fonts.xml file.  | STRING | true |    |
| <a id="ciq_scaled_bmfont_jungle-metrics_resource_id"></a>metrics_resource_id |  Resource ID to use in the generated metrics jsonData entry.  | STRING | false |  ""  |
| <a id="ciq_scaled_bmfont_jungle-percent"></a>percent |  Percentage of the base size to scale to (100 = original size).  | INT | false |  100  |
| <a id="ciq_scaled_bmfont_jungle-reference_chars"></a>reference_chars |  String of characters to use as a height reference for scaling. If unspecified, no additional scaling is applied.  | STRING | false |  ""  |
| <a id="ciq_scaled_bmfont_jungle-snap"></a>snap |  Pixel multiple to snap scaled font to.  | INT | false |  1  |
| <a id="ciq_scaled_bmfont_jungle-variations"></a>variations |  Dictionary of font variation axis tags to values (e.g. {'FILL': '1', 'wght': '700'}).  | STRING_DICT | false |  {}  |


<a id="ciq_scaled_drawable_jungle"></a>


<pre>
ciq_scaled_drawable_jungle(<a href="#ciq_scaled_drawable_jungle-name">name</a>, <a href="#ciq_scaled_drawable_jungle-src">src</a>, <a href="#ciq_scaled_drawable_jungle-automatic_palette">automatic_palette</a>, <a href="#ciq_scaled_drawable_jungle-device_dependent_height">device_dependent_height</a>, <a href="#ciq_scaled_drawable_jungle-device_dependent_width">device_dependent_width</a>, <a href="#ciq_scaled_drawable_jungle-device_ids">device_ids</a>, <a href="#ciq_scaled_drawable_jungle-percent">percent</a>, <a href="#ciq_scaled_drawable_jungle-resource_id">resource_id</a>)
</pre>

Generates a jungle file for a scaled drawable resource.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_scaled_drawable_jungle-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_scaled_drawable_jungle-src"></a>src |  Source image file to scale for different devices.  | LABEL | true |    |
| <a id="ciq_scaled_drawable_jungle-automatic_palette"></a>automatic_palette |  Whether to enable automatic palette reduction in the generated drawables.xml.  | BOOLEAN | false |  True  |
| <a id="ciq_scaled_drawable_jungle-device_dependent_height"></a>device_dependent_height |  Metric target to use for height scaling.  | LABEL | false |  None  |
| <a id="ciq_scaled_drawable_jungle-device_dependent_width"></a>device_dependent_width |  Metric target to use for width scaling.  | LABEL | false |  None  |
| <a id="ciq_scaled_drawable_jungle-device_ids"></a>device_ids |  List of device IDs to generate scaled resources for.  | STRING_LIST | false |  *All devices*  |
| <a id="ciq_scaled_drawable_jungle-percent"></a>percent |  Percentage of the base size to scale to (100 = original size).  | INT | false |  100  |
| <a id="ciq_scaled_drawable_jungle-resource_id"></a>resource_id |  Resource ID to use in the generated drawables.xml file.  | STRING | true |    |


<a id="ciq_screenshot_test"></a>


<pre>
ciq_screenshot_test(<a href="#ciq_screenshot_test-name">name</a>, <a href="#ciq_screenshot_test-expected">expected</a>, <a href="#ciq_screenshot_test-max_diff_pixels">max_diff_pixels</a>, <a href="#ciq_screenshot_test-pixel_tolerance">pixel_tolerance</a>, <a href="#ciq_screenshot_test-screenshot">screenshot</a>)
</pre>

Compares a captured screenshot PNG (from ciq_automation, ciq_field_screenshot, or ciq_framed_screenshot) against a golden reference image.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_screenshot_test-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_screenshot_test-expected"></a>expected |  The golden reference .png image to compare against.  | LABEL | true |    |
| <a id="ciq_screenshot_test-max_diff_pixels"></a>max_diff_pixels |  Maximum number of pixels allowed to exceed pixel_tolerance.  | INT | false |  0  |
| <a id="ciq_screenshot_test-pixel_tolerance"></a>pixel_tolerance |  Maximum per-channel (R, G, B, A) absolute difference (0-255) treated as matching for a given pixel.  | INT | false |  0  |
| <a id="ciq_screenshot_test-screenshot"></a>screenshot |  The screenshot .png file (e.g., ':my_automation.png') or ciq_automation target to compare.  | LABEL | true |    |


<a id="ciq_sideload_app"></a>


<pre>
ciq_sideload_app(<a href="#ciq_sideload_app-name">name</a>, <a href="#ciq_sideload_app-device_build">device_build</a>)
</pre>

Sideloads the application (.prg) to a connected physical Garmin device via MTP.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_sideload_app-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_sideload_app-device_build"></a>device_build |  The ciq_device_build target to sideload.  | LABEL | true |    |


<a id="ciq_simulation"></a>


<pre>
ciq_simulation(<a href="#ciq_simulation-name">name</a>, <a href="#ciq_simulation-device_build">device_build</a>)
</pre>

Creates a script to run the application in the Connect IQ Simulator.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_simulation-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_simulation-device_build"></a>device_build |  The ciq_device_build target to run in the simulator.  | LABEL | true |    |


<a id="ciq_test"></a>


<pre>
ciq_test(<a href="#ciq_test-name">name</a>, <a href="#ciq_test-device_build">device_build</a>)
</pre>

Creates a script to run the application tests in the Connect IQ Simulator.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_test-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_test-device_build"></a>device_build |  The ciq_device_build target (with include_tests=True) to test in the simulator.  | LABEL | true |    |


<a id="ciq_view_app_log"></a>


<pre>
ciq_view_app_log(<a href="#ciq_view_app_log-name">name</a>, <a href="#ciq_view_app_log-device_build">device_build</a>)
</pre>

Downloads and outputs the log file from a connected physical Garmin device.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_view_app_log-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_view_app_log-device_build"></a>device_build |  The ciq_device_build target to retrieve the log file for.  | LABEL | true |    |


<a id="ciq_view_app_profiling"></a>


<pre>
ciq_view_app_profiling(<a href="#ciq_view_app_profiling-name">name</a>, <a href="#ciq_view_app_profiling-device_build">device_build</a>)
</pre>

Downloads the profiling log (.PRF) from the device and analyzes it using the debug XML.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_view_app_profiling-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_view_app_profiling-device_build"></a>device_build |  The ciq_device_build target to retrieve the profiling log for.  | LABEL | true |    |


<a id="AutomationInfo"></a>

## AutomationInfo

<pre>
load("@rules_ciq//:defs.bzl", "AutomationInfo")

AutomationInfo(<a href="#AutomationInfo-screenshot_file">screenshot_file</a>, <a href="#AutomationInfo-log_file">log_file</a>, <a href="#AutomationInfo-device_id">device_id</a>)
</pre>

Provider for Garmin Connect IQ simulator automation outputs.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="AutomationInfo-screenshot_file"></a>screenshot_file |  The captured simulator display screenshot (.png).    |
| <a id="AutomationInfo-log_file"></a>log_file |  The captured application stdout log (.log).    |
| <a id="AutomationInfo-device_id"></a>device_id |  The device ID simulated (e.g., 'fenix6').    |


<a id="DeviceBuildInfo"></a>

## DeviceBuildInfo

<pre>
load("@rules_ciq//:defs.bzl", "DeviceBuildInfo")

DeviceBuildInfo(<a href="#DeviceBuildInfo-prg_file">prg_file</a>, <a href="#DeviceBuildInfo-prg_debug_xml_file">prg_debug_xml_file</a>, <a href="#DeviceBuildInfo-settings_json_file">settings_json_file</a>, <a href="#DeviceBuildInfo-device_id">device_id</a>)
</pre>

Provider for Garmin Connect IQ device build outputs.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="DeviceBuildInfo-prg_file"></a>prg_file |  The compiled .prg file that can be run on the device or simulator.    |
| <a id="DeviceBuildInfo-prg_debug_xml_file"></a>prg_debug_xml_file |  The debug XML file generated during compilation.    |
| <a id="DeviceBuildInfo-settings_json_file"></a>settings_json_file |  The settings JSON file generated during compilation.    |
| <a id="DeviceBuildInfo-device_id"></a>device_id |  The device ID this build was compiled for (e.g., 'fenix6').    |


<a id="JunglesInfo"></a>

## JunglesInfo

<pre>
load("@rules_ciq//:defs.bzl", "JunglesInfo")

JunglesInfo(<a href="#JunglesInfo-jungle_files">jungle_files</a>)
</pre>

Provider for jungle files used in Garmin Connect IQ projects.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="JunglesInfo-jungle_files"></a>jungle_files |  List of jungle files that define source and resource paths per device.    |


<a id="LanguagesInfo"></a>

## LanguagesInfo

<pre>
load("@rules_ciq//:defs.bzl", "LanguagesInfo")

LanguagesInfo(<a href="#LanguagesInfo-iso_639_2_codes">iso_639_2_codes</a>)
</pre>

Provider for language and internationalization metadata in Garmin Connect IQ projects.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="LanguagesInfo-iso_639_2_codes"></a>iso_639_2_codes |  List of supported ISO 639-2 language codes.    |


<a id="ManifestInfo"></a>

## ManifestInfo

<pre>
load("@rules_ciq//:defs.bzl", "ManifestInfo")

ManifestInfo(<a href="#ManifestInfo-manifest_file">manifest_file</a>)
</pre>

Provider for Garmin Connect IQ manifest file.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="ManifestInfo-manifest_file"></a>manifest_file |  The manifest.xml file that defines the application metadata and supported devices.    |


<a id="ciq_automation"></a>


<pre>
ciq_automation(<a href="#ciq_automation-name">name</a>, <a href="#ciq_automation-screenshot_out">screenshot_out</a>, <a href="#ciq_automation-log_out">log_out</a>, <a href="#ciq_automation-kwargs">kwargs</a>)
</pre>

Runs a ciq_device_build target in a containerized Connect IQ Simulator and produces two outputs: `<name>.png` and `<name>.log`.

Downstream rules can reference the predeclared output files directly via `:<name>.png` and `:<name>.log`,
or pass `:<name>` to rules that accept `AutomationInfo`.


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_automation-name"></a>name |  The target name.  |    |
| <a id="ciq_automation-screenshot_out"></a>screenshot_out |  Optional custom filename for the generated screenshot PNG (defaults to `<name>.png`).  |  None  |
| <a id="ciq_automation-log_out"></a>log_out |  Optional custom filename for the generated application log (defaults to `<name>.log`).  |  None  |
| <a id="ciq_automation-kwargs"></a>kwargs |  Arguments forwarded to the underlying automation rule (`device_build`, `layout`, `background_color`, `settings`, `actions`, `timer_actions`, `simulate_activity`, `fit_file`, `language`, `glance_mode`, `time_display`, `units_display`, `wait_for_log`, `delay_ms`, `timeout_secs`, etc.).  |    |

**RETURNS**




<a id="ciq_localized_strings_jungle"></a>


<pre>
ciq_localized_strings_jungle(<a href="#ciq_localized_strings_jungle-name">name</a>, <a href="#ciq_localized_strings_jungle-translations">translations</a>, <a href="#ciq_localized_strings_jungle-base">base</a>, <a href="#ciq_localized_strings_jungle-device_ids">device_ids</a>, <a href="#ciq_localized_strings_jungle-kwargs">kwargs</a>)
</pre>

Generates a jungle file mapping localized string resources (and optional base strings).

This rule isolates each language's XML string files into its own resource
directory, avoiding filename collisions and emitting the appropriate
`{device}.lang.<language>` qualifiers.

IMPORTANT: To avoid duplicate resource ID collisions in Monkey C, ensure
that any localized string files passed to this rule are NOT included in
generic `ciq_jungle` resource globs. For example:
    ciq_jungle(
        name = "static_resources",
        resources = glob(
            include = ["resources/common/**/*"],
            exclude = ["resources/common/strings/**/*"],
        ),
    )


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_localized_strings_jungle-name"></a>name |  A unique target name.  |    |
| <a id="ciq_localized_strings_jungle-translations"></a>translations |  Dictionary mapping ISO 639-2 language codes (e.g., 'deu', 'fin') to a label, file, or list of labels/files containing localized string XML resources.  |    |
| <a id="ciq_localized_strings_jungle-base"></a>base |  Optional label, file, or list of labels/files for base (fallback) string resources registered on the default resourcePath.  |  None  |
| <a id="ciq_localized_strings_jungle-device_ids"></a>device_ids |  List of device IDs to generate jungle entries for. Defaults to all devices.  |  *All devices*  |
| <a id="ciq_localized_strings_jungle-kwargs"></a>kwargs |  Additional rule arguments.  |    |

**RETURNS**




<a id="ciq_manifest_metadata"></a>


<pre>
ciq_manifest_metadata(<a href="#ciq_manifest_metadata-id">id</a>, <a href="#ciq_manifest_metadata-entry">entry</a>, <a href="#ciq_manifest_metadata-type">type</a>, <a href="#ciq_manifest_metadata-name_string_resource_id">name_string_resource_id</a>, <a href="#ciq_manifest_metadata-launcher_icon_drawable_resource_id">launcher_icon_drawable_resource_id</a>, <a href="#ciq_manifest_metadata-min_api_level">min_api_level</a>, <a href="#ciq_manifest_metadata-permissions">permissions</a>, <a href="#ciq_manifest_metadata-languages">languages</a>, <a href="#ciq_manifest_metadata-device_ids">device_ids</a>)
</pre>

Encapsulates metadata for a Connect IQ application manifest to be generated by ciq_project.

**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_manifest_metadata-id"></a>id |  Unique application ID (as a UUID string).  |    |
| <a id="ciq_manifest_metadata-entry"></a>entry |  Entry point class name for the application (e.g., 'MyApp').  |    |
| <a id="ciq_manifest_metadata-type"></a>type |  Application type ('audio_content_provider', 'data_field', 'watch_face', 'widget', or 'watchApp').  |    |
| <a id="ciq_manifest_metadata-name_string_resource_id"></a>name_string_resource_id |  String resource ID for the application name.  |    |
| <a id="ciq_manifest_metadata-launcher_icon_drawable_resource_id"></a>launcher_icon_drawable_resource_id |  Drawable resource ID for the launcher icon.  |    |
| <a id="ciq_manifest_metadata-min_api_level"></a>min_api_level |  Minimum Connect IQ API level required (default: '1.0.0').  |  "1.0.0"  |
| <a id="ciq_manifest_metadata-permissions"></a>permissions |  List of permissions required by the application.  |  []  |
| <a id="ciq_manifest_metadata-languages"></a>languages |  Optional explicit list of ISO 639-2 language codes (if omitted, derived from LanguagesInfo in project jungles).  |  []  |
| <a id="ciq_manifest_metadata-device_ids"></a>device_ids |  Optional list of device IDs to include in the manifest.  |  *All devices*  |

**RETURNS**

A struct containing the manifest metadata.


<a id="ciq_project"></a>


<pre>
ciq_project(<a href="#ciq_project-name">name</a>, <a href="#ciq_project-jungles">jungles</a>, <a href="#ciq_project-manifest">manifest</a>, <a href="#ciq_project-sdk">sdk</a>, <a href="#ciq_project-kwargs">kwargs</a>)
</pre>

Defines a Connect IQ project, linking a manifest file with jungle files, resources, and a target SDK.

The `manifest` parameter can either be:
- A manifest file label / target (e.g. `manifest = "manifest.xml"` or `manifest = ":my_manifest"`)
- A metadata struct created by `ciq_manifest_metadata(...)` or a metadata dictionary.
  When metadata is provided, the manifest is generated automatically and will
  auto-detect supported languages from `jungles`.


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_project-name"></a>name |  Target name for the project.  |    |
| <a id="ciq_project-jungles"></a>jungles |  List of jungle asset targets containing sources and resources.  |    |
| <a id="ciq_project-manifest"></a>manifest |  Either a manifest file target (e.g. "manifest.xml") or metadata from ciq_manifest_metadata(...).  |    |
| <a id="ciq_project-sdk"></a>sdk |  Connect IQ SDK version string (e.g., `"9.3.0"`).  |    |
| <a id="ciq_project-kwargs"></a>kwargs |  Additional arguments forwarded to the project rule.  |    |

**RETURNS**




<a id="ciq_store_image"></a>


<pre>
ciq_store_image(<a href="#ciq_store_image-name">name</a>, <a href="#ciq_store_image-images">images</a>, <a href="#ciq_store_image-max_size_kb">max_size_kb</a>, <a href="#ciq_store_image-transition_millis">transition_millis</a>, <a href="#ciq_store_image-kwargs">kwargs</a>)
</pre>

Generates a store image (GIF, PNG or JPG) from a list of files with size constraints.

**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_store_image-name"></a>name |  The name of the target.  |    |
| <a id="ciq_store_image-images"></a>images |  A list of image files to include. An animated GIF will be generated if more than one image is provided.  |    |
| <a id="ciq_store_image-max_size_kb"></a>max_size_kb |  The maximum allowed size for the output image in kilobytes.  |    |
| <a id="ciq_store_image-transition_millis"></a>transition_millis |  Optional. A single integer or a list of integers representing transition durations in milliseconds.  |  None  |
| <a id="ciq_store_image-kwargs"></a>kwargs |  Standard Bazel rule arguments (tags, visibility, etc.).  |    |

**RETURNS**




<a id="jungle_generator"></a>


<pre>
jungle_generator(<a href="#jungle_generator-ctx">ctx</a>, <a href="#jungle_generator-generator_func">generator_func</a>, <a href="#jungle_generator-device_ids">device_ids</a>)
</pre>

Generates a jungle file and associated resources/sources using a generator function.

`generator_func` is given the base directories for any generated source or
resource files; any files declared and generated should be prefixed with
the `sources_dir` or `resources_dir` respectively.


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="jungle_generator-ctx"></a>ctx |  The rule context.  |    |
| <a id="jungle_generator-generator_func"></a>generator_func |  A function that takes (`ctx`, `device_id`, `device_metadata`, `sources_dir`, `resources_dir`) and returns a list of generated files or `None` to skip.  |    |
| <a id="jungle_generator-device_ids"></a>device_ids |  A list of device IDs to generate for.  |  *All devices*  |

**RETURNS**

A list of providers [JunglesInfo, DefaultInfo].


<a id="supports_app_type"></a>


<pre>
supports_app_type(<a href="#supports_app_type-device_id">device_id</a>, <a href="#supports_app_type-app_type">app_type</a>)
</pre>

Checks if a device supports a specific app type.

**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="supports_app_type-device_id"></a>device_id |  The device ID to check.  |    |
| <a id="supports_app_type-app_type"></a>app_type |  The application type to check for (e.g. "data_field").  |    |

**RETURNS**

True if the device supports the specified app type, False otherwise.


<a id="supports_min_sdk"></a>


<pre>
supports_min_sdk(<a href="#supports_min_sdk-device_id">device_id</a>, <a href="#supports_min_sdk-min_api_level">min_api_level</a>)
</pre>

Checks if a device supports a minimum API level.

**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="supports_min_sdk-device_id"></a>device_id |  The device ID to check.  |    |
| <a id="supports_min_sdk-min_api_level"></a>min_api_level |  The minimum API level required (e.g., "3.0.0").  |    |

**RETURNS**

True if the device supports the specified minimum API level, False otherwise.


