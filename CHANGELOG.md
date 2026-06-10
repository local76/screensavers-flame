# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2026.7.0] - 2026-06-10

### Added
- **4.2 Screensaver Host Loop**: Standardized on library::screensaver_runtime to drive the screensaver execution, supporting CLI flags for preview mode (/p), configuration mode (/c), run mode (/s), and help (-h).

### Changed
- **Workspace Collapse**: Replaced local screensaver logic files with a clean 20-line shim importing the consolidated scene from the shared library.
- **Repository Rename**: Renamed repository and local directory to screensavers-flame for cleaner ecosystem taxonomy.