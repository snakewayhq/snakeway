---
title: Configuration Internals
---


The configuration subsystem is located in `snakeway-core/src/conf`.

The configuration loading process is broken down into the following steps:

1. Spec
    - The operator-facing config file specification.
    - Defined in `snakeway-core/src/conf/types/specification`
2. Parse
    - Ingest the config files and convert to an in-memory spec representation.
    - Fail early on parse errors.
3. Validate
    - Pass the spec representation through a series of validation steps.
    - Collect validation errors and warnings.
4. Lower
    - Transform the spec representation into a runtime representation.
    - Define in `snakeway-core/src/conf/types/runtime`
5. Runtime
    - Runtime representation is returned by the `load_config` function.
