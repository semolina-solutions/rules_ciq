<!-- Generated with Stardoc: http://skydoc.bazel.build -->

Macros for generating Connect IQ build, simulation, and upload targets for multiple devices.

<a id="ciq_device_targets_macro"></a>


<pre>
ciq_device_targets_macro(<a href="#ciq_device_targets_macro-name">name</a>, <a href="#ciq_device_targets_macro-visibility">visibility</a>, <a href="#ciq_device_targets_macro-project">project</a>, <a href="#ciq_device_targets_macro-device_ids">device_ids</a>, <a href="#ciq_device_targets_macro-type_check_level">type_check_level</a>)
</pre>

Generates debug and release build, simulation, and device interaction targets for multiple devices.

For example, if `name` is "my_app" and `device_ids` includes "fenix6",
the following targets will be generated:
- `//path/to/package:my_app_fenix6_debug_build` (build)
- `//path/to/package:my_app_fenix6_debug_build_for_testing` (build)
- `//path/to/package:my_app_fenix6_debug_profiling_build` (build)
- `//path/to/package:my_app_fenix6_debug_profiling_sideload_app` (run)
- `//path/to/package:my_app_fenix6_debug_profiling_simulation` (run)
- `//path/to/package:my_app_fenix6_debug_sideload_app` (run)
- `//path/to/package:my_app_fenix6_debug_simulation` (run)
- `//path/to/package:my_app_fenix6_debug_view_app_log` (run)
- `//path/to/package:my_app_fenix6_debug_view_app_profiling` (run)
- `//path/to/package:my_app_fenix6_release_build` (build)
- `//path/to/package:my_app_fenix6_release_profiling_build` (build)
- `//path/to/package:my_app_fenix6_release_profiling_sideload_app` (run)
- `//path/to/package:my_app_fenix6_release_profiling_simulation` (run)
- `//path/to/package:my_app_fenix6_release_sideload_app` (run)
- `//path/to/package:my_app_fenix6_release_simulation` (run)
- `//path/to/package:my_app_fenix6_release_view_app_log` (run)
- `//path/to/package:my_app_fenix6_release_view_app_profiling` (run)
- `//path/to/package:my_app_fenix6_test` (test)

These targets can be built or run using `bazel build` or `bazel run`.
For example:
- `bazel build //path/to/package:my_app_fenix6_debug_build`
- `bazel run //path/to/package:my_app_fenix6_debug_simulation`
- `bazel run //path/to/package:my_app_fenix6_debug_sideload_app`
- `bazel test //path/to/package:my_app_fenix6_test`

Profiling note: The *_profiling_simulation targets ensure profiling is
started along with the simulation, capturing startup execution. Profiling
may otherwise be started manually when using any *_simulation target.


**PARAMETERS**

| Name | Description | Default Value |
| :--- | :--- | :--- |
| <a id="ciq_device_targets_macro-name"></a>name |  Base name for the generated targets.  |    |
| <a id="ciq_device_targets_macro-visibility"></a>visibility |  Visibility specification for the build targets.  |  None  |
| <a id="ciq_device_targets_macro-project"></a>project |  The Connect IQ project to build.  |  None  |
| <a id="ciq_device_targets_macro-device_ids"></a>device_ids |  List of device IDs to generate targets for. Defaults to all available devices.  |  *All devices*  |
| <a id="ciq_device_targets_macro-type_check_level"></a>type_check_level |  Type check level for the build (0=Silent, 1=Gradual, 2=Informative, 3=Strict).  |  None  |

**RETURNS**




