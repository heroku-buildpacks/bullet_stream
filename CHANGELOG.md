# CHANGELOG

## Unreleased

- Fix: dropping `Print<Background<W>>` now emits a message of `(Error)` and a newline (https://github.com/heroku-buildpacks/bullet_stream/pull/20)
- Added: `Print::global()` and `bullet_stream::global::set_writer`. Use these to preserve the newline indentation when handling dropped structs or errors ()
- Added: `bullet_stream::global::print` functions for writing formatted output without needing to preserve state ()
- Added: `Print::global()` and `bullet_stream::global::set_writer`. Use these to preserve the newline indentation when handling dropped structs or errors ()

## v0.3.0 - 2024/08/14

- Added `bullet_stream::strip_ansi` (https://github.com/schneems/bullet_stream/pull/11)
- Added `Print<Background<W>>::cancel()` to stop a timer with a message instead of emitting timing information (https://github.com/schneems/bullet_stream/pull/10)

## v0.2.0 - 2024/06/06

- Added: `Print` struct. `Output` is now deprecated, use `Print` instead (https://github.com/schneems/bullet_stream/pull/5)
- Fix: Missing `must_use` attributes (https://github.com/schneems/bullet_stream/pull/5)

## v0.1.1 - 2024/06/03

- Fix double newlines for headers (https://github.com/schneems/bullet_stream/pull/2)

## v0.1.0 - 2024/06/03

- First
