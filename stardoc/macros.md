<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Macros for generating Connect IQ build, simulation, and upload targets for multiple devices.

<a id="device_targets_macro"></a>


<pre>
device_targets_macro(<a href="#device_targets_macro-name">name</a>, <a href="#device_targets_macro-visibility">visibility</a>, <a href="#device_targets_macro-project">project</a>, <a href="#device_targets_macro-device_ids">device_ids</a>, <a href="#device_targets_macro-type_check_level">type_check_level</a>)
</pre>

Generates debug and release build, simulation, and upload targets for multiple devices.

For example, if `name` is "my_app" and `device_ids` includes "fenix6",
the following debug and release targets will be generated:
- `//path/to/package:my_app_fenix6_debug_build`
- `//path/to/package:my_app_fenix6_debug_profiling_build`
- `//path/to/package:my_app_fenix6_debug_build_for_testing`
- `//path/to/package:my_app_fenix6_debug_log_cat`
- `//path/to/package:my_app_fenix6_debug_simulation`
- `//path/to/package:my_app_fenix6_debug_upload`
- `//path/to/package:my_app_fenix6_debug_profiling_upload`
- `//path/to/package:my_app_fenix6_release_build`
- `//path/to/package:my_app_fenix6_release_profiling_build`
- `//path/to/package:my_app_fenix6_release_log_cat`
- `//path/to/package:my_app_fenix6_release_simulation`
- `//path/to/package:my_app_fenix6_release_profiling_simulation`
- `//path/to/package:my_app_fenix6_release_upload`
- `//path/to/package:my_app_fenix6_release_profiling_upload`
- `//path/to/package:my_app_fenix6_test`

These targets can be built or run using `bazel build` or `bazel run`.
For example:
- `bazel build //path/to/package:my_app_fenix6_debug_build`
- `bazel run //path/to/package:my_app_fenix6_debug_simulation`
- `bazel run //path/to/package:my_app_fenix6_debug_upload`
- `bazel test //path/to/package:my_app_fenix6_test`

Profiling note: The *_profiling_simulation targets ensure profiling is
started along with the simulation, capturing startup execution. Profiling
may otherwise be started manually when using any *_simulation target.


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="device_targets_macro-name"></a>name |  Base name for the generated targets.  |    |
| <a id="device_targets_macro-visibility"></a>visibility |  Visibility specification for the build targets.  |  None  |
| <a id="device_targets_macro-project"></a>project |  The Connect IQ project to build.  |  None  |
| <a id="device_targets_macro-device_ids"></a>device_ids |  List of device IDs to generate targets for. Defaults to all available devices.  |  *All devices*  |
| <a id="device_targets_macro-type_check_level"></a>type_check_level |  Type check level for the build (0=Silent, 1=Gradual, 2=Informative, 3=Strict).  |  None  |

**RETURNS**




