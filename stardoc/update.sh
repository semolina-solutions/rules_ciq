#!/bin/bash
# Script to update documentation from Bazel-generated Stardoc outputs

set -e

echo "Building documentation..."
bazel build //stardoc:defs //stardoc:macros

echo "Copying generated documentation..."
cp -f bazel-bin/stardoc/defs.md stardoc/defs.md
cp -f bazel-bin/stardoc/macros.md stardoc/macros.md

echo "Documentation updated successfully!"
echo "Please review the changes and commit them to git."
