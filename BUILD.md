# BUILD

All official builds happen on GitHub Actions. You should too — here's why and
how.

## Why CI builds (and not locally)

A clean release build of this app pulls in a lot of rust crates and compiles
them down, which means lots of small file writes. On an SSD that adds up fast,
and honestly rust compilation is one of the quickest ways to torch your drive's
write budget for not much gain. So the rule is simple: **don't build locally
unless you have to.** Let the CI server sweat for you.

As a bonus, CI builds every target from a clean, reproducible environment and
produces ready-to-ship binaries for all supported platforms.

## What the CI does

The workflow lives at `.github/workflows/build.yml` and has three jobs:

- **lint** — runs `cargo fmt --check` and `cargo clippy` on ubuntu.
- **test** — runs `cargo test` on ubuntu.
- **build** — a matrix that produces a release binary for each platform:

  | runner                | target                       | artifact (release .zip)          |
  | --------------------- | ---------------------------- | -------------------------------- |
  | `ubuntu-22.04`        | `x86_64-unknown-linux-gnu`   | `handy-groq-linux-amd64`         |
  | `ubuntu-24.04`        | `aarch64-unknown-linux-gnu`  | `handy-groq-linux-arm64`         |
  | `macos-14`            | `x86_64-apple-darwin`        | `handy-groq-macos-x64`           |
  | `macos-14-large`      | `aarch64-apple-darwin`       | `handy-groq-macos-arm64`         |
  | `windows-2022`        | `x86_64-pc-windows-msvc`     | `handy-groq-windows-x64`         |

Each matrix entry runs `cargo build --release --target <target>`, then zips the
binary up and uploads it as a GitHub artifact named after the table above.

## Toolchain

- **Rust:** stable 1.99.0 (`dtolnay/rust-toolchain@stable`).
- **Caching:** `Swatinem/rust-cache@v2` is used on every job so builds aren't
  compiling from scratch every time.

## Dependencies (the ones that matter for building)

| crate          | version | why it's here                                  |
| -------------- | ------- | ---------------------------------------------- |
| `eframe`       | 0.36    | the gui (pure rust, no webkit)                 |
| `cpal`         | 0.16    | mic audio capture                              |
| `hound`        | 3.5     | wav encoding                                   |
| `reqwest`      | 0.12    | http to groq's api (uses `native-tls`)         |
| `tokio`        | 1       | async runtime for the transcription request    |
| `global-hotkey`| 0.8     | the system-wide push-to-talk shortcut          |
| `enigo`        | 0.6     | simulating the paste keystroke                 |
| `arboard`      | 3       | clipboard access                               |
| `serde` / `serde_json` | 1 / 1 | reading/writing the settings file      |
| `dirs`         | 6       | finding the config directory per-os            |

Plus the usual helpers: `anyhow`, `log`, `env_logger`.

On linux the CI job installs a few system libs that `cpal` and `eframe`
need to talk to alsa / x11:

```
sudo apt-get update
sudo apt-get install -y libasound2-dev libxcb-shape0-dev libxcb1-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libxkbcommon-x11-0 pkg-config
```

## Running the CI workflow

It triggers automatically on every push and pull request to `main`. To run it
manually:

1. Open the repo on github, go to **Actions** → **Build**.
2. Click **Run workflow** → pick the `main` branch.
3. Wait for the **build** job to finish for your platform.
4. Download the artifact (e.g. `handy-groq-windows-x64.zip`) from the bottom of
   the run.

Artifacts stick around on github for ~90 days after a run.

## Building locally (only if you really need to)

This is not recommended for normal use — it's for development and debugging.

**What you need installed:**

- the rust stable toolchain
- a C compiler + linker — `reqwest` uses `native-tls`, so it needs to link
  against the system's openssl/tls libs.
  - **windows:** a "Desktop development with C++" workload (MSVC) and openssl
    via vcpkg, or switch `reqwest` to `rustls-tls`.
  - **macos:** xcode command line tools (`xcode-select --install`).
  - **linux:** gcc/make + `libssl-dev` and the x11/alsa libs listed above.

**The actual build:**

```sh
# release (like CI does)
cargo build --release

# or for a specific target
cargo build --release --target x86_64-pc-windows-msvc
```

**Speeding up local builds without killing your SSD:** point
`CARGO_TARGET_DIR` at a ramdisk so all the write churn lands in memory instead
of on your drive. This is the same trick CI gets for free (ephemeral
disk). Example on linux:

```sh
mkdir /tmp/ramdisk
mount -t tmpfs -o size=4G tmpfs /tmp/ramdisk
export CARGO_TARGET_DIR=/tmp/ramdisk/handy-groq
cargo build --release
```

On macOS you can use a `tmpfs`-style ram disk too; on windows, a `imdisk` or
built-in ram disk works the same idea.

> **tl;dr** — Use the CI. It builds every platform, ships a zip, and doesn't
> write to your SSD. Local builds need a C toolchain (for native-tls) and are
> only worth it if you're hacking on the code.
