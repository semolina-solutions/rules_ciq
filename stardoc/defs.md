<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Bazel rules and providers for building Garmin Connect IQ applications.

This module defines custom Bazel rules for building Connect IQ projects, including:
- Jungle file generation for device-specific resources and sources
- Manifest generation with device filtering based on API level and app type
- Device builds and exports using the MonkeyC compiler
- Simulator support for testing applications

<a id="ciq_device_build"></a>


<pre>
ciq_device_build(<a href="#ciq_device_build-name">name</a>, <a href="#ciq_device_build-device_id">device_id</a>, <a href="#ciq_device_build-include_tests">include_tests</a>, <a href="#ciq_device_build-project">project</a>, <a href="#ciq_device_build-release">release</a>, <a href="#ciq_device_build-sdk">sdk</a>, <a href="#ciq_device_build-type_check_level">type_check_level</a>)
</pre>

Builds the application (.prg) for a specific device.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_device_build-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_device_build-device_id"></a>device_id |  Target device ID to build for (e.g., 'fenix6').  | STRING | true |    |
| <a id="ciq_device_build-include_tests"></a>include_tests |  Include test code in the build.  | BOOLEAN | false |  False  |
| <a id="ciq_device_build-project"></a>project |  The ciq_project target containing the manifest and jungle assets.  | LABEL | true |    |
| <a id="ciq_device_build-release"></a>release |  Build in release mode (optimized, no debug symbols).  | BOOLEAN | false |  False  |
| <a id="ciq_device_build-sdk"></a>sdk |  Connect IQ SDK to use for compilation.  | LABEL | false |  "@local_ciq//sdk:current"  |
| <a id="ciq_device_build-type_check_level"></a>type_check_level |  Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict).  | INT | false |  0  |


<a id="ciq_device_log_cat"></a>


<pre>
ciq_device_log_cat(<a href="#ciq_device_log_cat-name">name</a>, <a href="#ciq_device_log_cat-device_build">device_build</a>)
</pre>

Downloads and outputs the log file from a connected physical Garmin device.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_device_log_cat-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_device_log_cat-device_build"></a>device_build |  The ciq_device_build target to retrieve the log file for.  | LABEL | true |    |


<a id="ciq_device_upload"></a>


<pre>
ciq_device_upload(<a href="#ciq_device_upload-name">name</a>, <a href="#ciq_device_upload-device_build">device_build</a>)
</pre>

Uploads the application (.prg) to a connected physical Garmin device via MTP.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_device_upload-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_device_upload-device_build"></a>device_build |  The ciq_device_build target to upload.  | LABEL | true |    |


<a id="ciq_export"></a>


<pre>
ciq_export(<a href="#ciq_export-name">name</a>, <a href="#ciq_export-project">project</a>, <a href="#ciq_export-sdk">sdk</a>, <a href="#ciq_export-type_check_level">type_check_level</a>)
</pre>

Exports the application (.iq) for distribution.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_export-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_export-project"></a>project |  The ciq_project target containing the manifest and jungle assets.  | LABEL | true |    |
| <a id="ciq_export-sdk"></a>sdk |  Connect IQ SDK to use for compilation.  | LABEL | false |  "@local_ciq//sdk:current"  |
| <a id="ciq_export-type_check_level"></a>type_check_level |  Type checking level: 0 (Silent), 1 (Gradual), 2 (Informative), or 3 (Strict). Default is 0.  | INT | false |  0  |


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


<a id="ciq_manifest"></a>


<pre>
ciq_manifest(<a href="#ciq_manifest-name">name</a>, <a href="#ciq_manifest-device_ids">device_ids</a>, <a href="#ciq_manifest-entry">entry</a>, <a href="#ciq_manifest-id">id</a>, <a href="#ciq_manifest-launcher_icon_drawable_resource_id">launcher_icon_drawable_resource_id</a>, <a href="#ciq_manifest-min_api_level">min_api_level</a>, <a href="#ciq_manifest-name_string_resource_id">name_string_resource_id</a>, <a href="#ciq_manifest-permissions">permissions</a>, <a href="#ciq_manifest-type">type</a>)
</pre>

Generates a manifest.xml file for the application.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_manifest-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_manifest-device_ids"></a>device_ids |  List of device IDs to include in the manifest. Devices are filtered by app type and API level support.  | STRING_LIST | false |  *All devices*  |
| <a id="ciq_manifest-entry"></a>entry |  Entry point class name for the application (e.g., 'MyApp').  | STRING | true |    |
| <a id="ciq_manifest-id"></a>id |  Unique application ID (as a UUID).  | STRING | true |    |
| <a id="ciq_manifest-launcher_icon_drawable_resource_id"></a>launcher_icon_drawable_resource_id |  Drawable resource ID for the launcher icon.  | STRING | true |    |
| <a id="ciq_manifest-min_api_level"></a>min_api_level |  Minimum Connect IQ API level required (e.g., '3.0.0').  | STRING | true |    |
| <a id="ciq_manifest-name_string_resource_id"></a>name_string_resource_id |  String resource ID for the application name (e.g., 'AppName').  | STRING | true |    |
| <a id="ciq_manifest-permissions"></a>permissions |  List of permissions required by the application (e.g., ['Positioning', 'Communications']).  | STRING_LIST | false |  []  |
| <a id="ciq_manifest-type"></a>type |  Application type: 'audio_content_provider', 'data_field', 'watch_face', 'widget', or 'watchApp'.  | STRING | true |    |


<a id="ciq_project"></a>


<pre>
ciq_project(<a href="#ciq_project-name">name</a>, <a href="#ciq_project-jungles">jungles</a>, <a href="#ciq_project-manifest">manifest</a>)
</pre>

Defines a Connect IQ project, linking a manifest file with jungle files and resources.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_project-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_project-jungles"></a>jungles |  List of jungle assets (typically from ciq_jungle or ciq_scaled_drawable_jungle) containing sources and resources.  | LABEL_LIST | true |    |
| <a id="ciq_project-manifest"></a>manifest |  The manifest.xml file for the project (typically from ciq_manifest).  | LABEL | true |    |


<a id="ciq_scaled_drawable_jungle"></a>


<pre>
ciq_scaled_drawable_jungle(<a href="#ciq_scaled_drawable_jungle-name">name</a>, <a href="#ciq_scaled_drawable_jungle-src">src</a>, <a href="#ciq_scaled_drawable_jungle-device_ids">device_ids</a>, <a href="#ciq_scaled_drawable_jungle-font_name">font_name</a>, <a href="#ciq_scaled_drawable_jungle-font_set">font_set</a>, <a href="#ciq_scaled_drawable_jungle-mode">mode</a>, <a href="#ciq_scaled_drawable_jungle-percent">percent</a>, <a href="#ciq_scaled_drawable_jungle-resource_id">resource_id</a>)
</pre>

Generates a jungle file for a scaled drawable resource.

**ATTRIBUTES**

| Name | Description | Type | Mandatory | Default |
| :--- | :--- | :--- | :--- | :--- |
| <a id="ciq_scaled_drawable_jungle-name"></a>name |  A unique name for this target.  | NAME | true |    |
| <a id="ciq_scaled_drawable_jungle-src"></a>src |  Source image file to scale for different devices.  | LABEL | true |    |
| <a id="ciq_scaled_drawable_jungle-device_ids"></a>device_ids |  List of device IDs to generate scaled resources for.  | STRING_LIST | false |  *All devices*  |
| <a id="ciq_scaled_drawable_jungle-font_name"></a>font_name |  Font name to use for 'font_height' mode. Required when mode is 'font_height'.  | STRING | false |  ""  |
| <a id="ciq_scaled_drawable_jungle-font_set"></a>font_set |  Font set to use for 'font_height' mode (e.g., 'ww' for worldwide).  | STRING | false |  "ww"  |
| <a id="ciq_scaled_drawable_jungle-mode"></a>mode |  Scaling mode: 'icon' (launcher icon size), 'screen_width', 'screen_height', 'screen_fill' (full screen), or 'font_height' (based on font metrics).  | STRING | true |    |
| <a id="ciq_scaled_drawable_jungle-percent"></a>percent |  Percentage of the base size to scale to (100 = original size).  | INT | false |  100  |
| <a id="ciq_scaled_drawable_jungle-resource_id"></a>resource_id |  Resource ID to use in the generated drawables.xml file.  | STRING | true |    |


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


<a id="DeviceBuildInfo"></a>

## DeviceBuildInfo

<pre>
load("@rules_ciq//:defs.bzl", "DeviceBuildInfo")

DeviceBuildInfo(<a href="#DeviceBuildInfo-prg_file">prg_file</a>, <a href="#DeviceBuildInfo-settings_json_file">settings_json_file</a>, <a href="#DeviceBuildInfo-device_id">device_id</a>)
</pre>

Provider for Garmin Connect IQ device build outputs.

**FIELDS**

| Name  | Description |
| :------------- | :------------- |
| <a id="DeviceBuildInfo-prg_file"></a>prg_file |  The compiled .prg file that can be run on the device or simulator.    |
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


