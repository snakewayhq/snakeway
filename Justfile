#!/usr/bin/env just --justfile

# List recipes
list:
    just --list

# Default recipe
default: list

# Import modular justfiles

import "dev/just/benchmark.just"
import "dev/just/build.just"
import "dev/just/code_quality.just"
import "dev/just/debug.just"
import "dev/just/package.just"
import "dev/just/test.just"
import "dev/just/tools_and_docs.just"
