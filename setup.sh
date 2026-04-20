#!/bin/bash
# OBINexus semverx Setup Script
# Part of the polyglot toolchain: riftlang.exe -> .so.a -> rift.exe -> gosilang

set -e

echo "Setting up semverx for OBINexus..."

# Ensure proper directory structure
mkdir -p src/bin
mkdir -p schemas
mkdir -p scripts

# Initialize cargo project if needed
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found"
    exit 1
fi

# Build the project
cargo build --release

echo "semverx setup complete"
