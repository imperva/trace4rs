# Change Log

## [UNRELEASED]

* Externalize utc-offset crate

### Changed

* Migrated all path manipulation to use `camino`.

## [0.5.0]

### Added

* Add utc-offset as a last resort to obtain the local utc offset

## [0.4.6]

### Fixed

* Fix format str deserialization.

## [0.4.5]

### Fixed

* Rm a print.

## [0.4.4]

### Fixed

* Actually apply custom format fix.

## [0.4.3]

### Fixed

* Fixed custom log format serialization schema.

## [0.4.2]

### Fixed

* Compile err with schemars and serde features enabled.
* Move `allow(clippy::pub_use)` to the root.

## [0.4.1]

### Added

* Added utc time formatting within custom format strings

## [0.3.1]

### Fixed

Bugfix

## [0.3.0]

* Fix schemars [#5](https://github.com/imperva/trace4rs/pull/5)

### Fixed

* Fixed config serialization issue [#3](https://github.com/imperva/trace4rs/pull/4)

## [0.2.1]

### Fixed

* Fixed wrong trace4rs linked packages.

## [0.2.0]

### Changed

* Change custom log formatting config: [#3](https://github.com/imperva/trace4rs/pull/3)

## [0.1.0]

Initial release
