"""
Public API for rules_ciq.

This module re-exports rules and providers from the various sub-packages
(build, device, simulator, store) for convenience.
"""

load("@local_ciq//:defs.bzl", _devices = "devices")
load(
    "//build:defs.bzl",
    _DeviceBuildInfo = "DeviceBuildInfo",
    _JunglesInfo = "JunglesInfo",
    _ManifestInfo = "ManifestInfo",
    _ciq_device_build = "ciq_device_build",
    _ciq_export = "ciq_export",
    _ciq_jungle = "ciq_jungle",
    _ciq_manifest = "ciq_manifest",
    _ciq_project = "ciq_project",
    _ciq_scaled_drawable_jungle = "ciq_scaled_drawable_jungle",
    _jungle_generator = "jungle_generator",
    _supports_app_type = "supports_app_type",
    _supports_min_sdk = "supports_min_sdk",
)
load(
    "//device:defs.bzl",
    _ciq_sideload_app = "ciq_sideload_app",
    _ciq_view_app_log = "ciq_view_app_log",
    _ciq_view_app_profiling = "ciq_view_app_profiling",
)
load(
    "//simulator:defs.bzl",
    _ciq_simulation = "ciq_simulation",
    _ciq_test = "ciq_test",
)
load(
    "//store:defs.bzl",
    _ciq_framed_screenshot = "ciq_framed_screenshot",
    _ciq_store_image = "ciq_store_image",
)

# @local_ciq//:defs.bzl
devices = _devices

# build/defs.bzl
JunglesInfo = _JunglesInfo
ManifestInfo = _ManifestInfo
DeviceBuildInfo = _DeviceBuildInfo
ciq_jungle = _ciq_jungle
ciq_scaled_drawable_jungle = _ciq_scaled_drawable_jungle
ciq_manifest = _ciq_manifest
ciq_project = _ciq_project
ciq_device_build = _ciq_device_build
ciq_export = _ciq_export
jungle_generator = _jungle_generator
supports_app_type = _supports_app_type
supports_min_sdk = _supports_min_sdk

# device/defs.bzl
ciq_sideload_app = _ciq_sideload_app
ciq_view_app_log = _ciq_view_app_log
ciq_view_app_profiling = _ciq_view_app_profiling

# simulator/defs.bzl
ciq_simulation = _ciq_simulation
ciq_test = _ciq_test

# store/defs.bzl
ciq_framed_screenshot = _ciq_framed_screenshot
ciq_store_image = _ciq_store_image