# CHANGELOG

## Unreleased

- Add: `h3` header support (https://github.com/heroku-buildpacks/bullet_stream/pull/32)
- Add: `global::print::plain` to print out plain text like `println!`. It auto-flushes IO, redirects to the global writer (if you wanted to capture everything), and enables "paragraph detection" if it's followed by something like a warning or error ()

## v0.6.1 - 2024/02/11

- Fix: Relax the constraint of `fun_run` optional dependency. Now any version higher than `0.5` and less than `1.0` is will work. (https://github.com/heroku-buildpacks/bullet_stream/pull/30)

## v0.6.0 - 2025/02/05

- Added: `Print<T>::error()` now returns the original writer `W`, this allows for building error messages (`Vec<u8>`) with debug output above it using the stateful API. See the tests for an example (https://github.com/heroku-buildpacks/bullet_stream/pull/28)

## v0.5.0 - 2025/01/31

- A `fun_run` feature to provide optional interfaces when the `fun_run` crate is being used. PR: (https://github.com/heroku-buildpacks/bullet_stream/pull/25)
  - Added: `global::print::sub_stream_cmd` and `global::print::sub_time_cmd`
  - Added: `Print<SubBullet<W>>::stream_cmd()` and `Print<SubBullet<W>>::time_cmd()`
- Struct `_GlobalWriter` is deprecated, use `GlobalWriter` instead (https://github.com/heroku-buildpacks/bullet_stream/pull/26)

## v0.4.0 - 2025/01/21

- Fix: dropping `Print<Background<W>>` now emits a message of `(Error)` and a newline (https://github.com/heroku-buildpacks/bullet_stream/pull/20)
- Added: `bullet_stream::global::print` functions for writing formatted output without needing to preserve state (https://github.com/heroku-buildpacks/bullet_stream/pull/21)
- Added: `Print::global()` and `bullet_stream::global::set_writer`. Use these to preserve the newline indentation when handling dropped structs or errors (https://github.com/heroku-buildpacks/bullet_stream/pull/21)

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
