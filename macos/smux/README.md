# smux native shell (macOS)

Native macOS terminal shell using libghostty for terminal rendering.

## Build

```bash
swift build --package-path macos/smux
```

## Run

```bash
swift run --package-path macos/smux
```

## Architecture

- Swift/AppKit + libghostty (via prebuilt xcframework from libghostty-spm)
- Communicates with smux-daemon over Unix socket IPC
- Rust core handles orchestration, consensus, policy
