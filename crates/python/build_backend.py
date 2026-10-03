"""PEP 517 hooks that build the bundled renderer before delegating to maturin."""

import os
import struct

import maturin

from build_native_viewer import build_native_viewer


def _prepare_renderer(config_settings, editable=False):
    arguments = maturin.get_maturin_pep517_args(config_settings)
    profile = "dev" if editable else "release"
    target = os.environ.get("CARGO_BUILD_TARGET")
    if target is None and os.name == "nt" and struct.calcsize("P") == 4:
        target = "i686-pc-windows-msvc"
    for index, argument in enumerate(arguments):
        if argument == "--release":
            profile = "release"
        elif argument in {"--profile", "--target"}:
            value = arguments[index + 1]
            if argument == "--profile":
                profile = value
            else:
                target = value
        elif argument.startswith("--target="):
            target = argument.split("=", 1)[1]
        elif argument.startswith("--profile="):
            profile = argument.split("=", 1)[1]
    build_native_viewer(profile=profile, target=target)


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    _prepare_renderer(config_settings)
    return maturin.build_wheel(wheel_directory, config_settings, metadata_directory)


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    _prepare_renderer(config_settings, editable=True)
    return maturin.build_editable(wheel_directory, config_settings, metadata_directory)


build_sdist = maturin.build_sdist
get_requires_for_build_wheel = maturin.get_requires_for_build_wheel
get_requires_for_build_editable = maturin.get_requires_for_build_editable
get_requires_for_build_sdist = maturin.get_requires_for_build_sdist
prepare_metadata_for_build_wheel = maturin.prepare_metadata_for_build_wheel
prepare_metadata_for_build_editable = maturin.prepare_metadata_for_build_editable
