# Contributing

Thanks for stopping by to help out. This is a lean little app, so contributing
is straightforward.

## Getting started

1. **Fork** the repo on github: `dp1x/handy-groq`.
2. **Clone** your fork locally and add upstream:
   ```sh
   git clone https://github.com/<you>/handy-groq.git
   cd handy-groq
   git remote add upstream https://github.com/dp1x/handy-groq.git
   ```
3. **Create a branch** for your change:
   ```sh
   git checkout -b fix-something-here
   ```

## Making changes

- Write tests. There are existing unit tests across `settings.rs`, `audio.rs`,
  `shortcut.rs`, `groq_client.rs`, and `paste.rs` — follow the same `#[cfg(test)]`
  style. New behavior needs coverage.
- Keep the code formatted and lint-free. Before you commit, run:
  ```sh
  cargo fmt
  cargo clippy --no-deps
  ```
- Make sure the project still compiles cleanly:
  ```sh
  cargo check
  ```
- **Speed up local builds + save your SSD:** point `CARGO_TARGET_DIR` at a
  ramdisk (this is what CI effectively does for free):
  ```sh
  export CARGO_TARGET_DIR=/tmp/ramdisk/handy-groq
  cargo check   # or cargo build / cargo test
  ```

## Pushing and opening a PR

1. Commit your work with a short, honest message.
2. Push to your fork:
   ```sh
   git push origin fix-something-here
   ```
3. Open a PR against `dp1x/handy-groq` on github. Fill in what you changed and
   why.

## CI

The CI runs automatically on every PR (and push to `main`): it lints with
`cargo fmt` + `clippy`, runs the test suite, and builds release binaries for
windows, macos, and linux. Merge once it's green.

## Notes

- No code signing is set up or expected. If you add a platform-specific bit,
  just make sure CI builds it — don't try to sign anything locally.
- There's no Tauri or webview in this project. If a dependency would pull in a
  browser engine, don't add it.

Keep it lean. Keep it rusty.
