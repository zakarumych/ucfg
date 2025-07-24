Ultimate Configuration Library
==============================

This library provides a comprehensive solution for configuring applications from various sources, layering configurations, supporting overloads, extensions, and more.


Goal
----

To be one-stop for all configuration needs, regardless of the nature of the application or the complexity of the configuration.


Features
--------

- Using serde compatible types for leaf configurations.
- Supports sourcing configuration from:
  - Environment variables
  - Files (JSON, YAML, TOML)
  - Command line arguments
  - Custom sources via `Source` trait
- Supports multiple ways of updating configuration value from layered sources:
  - Recursive update (default)
  - Overriding (the only option for serde-deserializable types)
  - Extending (available for container types)
- Derive macros to declaratively define configuration structures.
