# Apple Intelligence Build Setup

## Status

✅ Xcode installed at `/Applications/Xcode.app`
✅ FoundationModels.framework available
⚠️ Need to configure xcode-select to point to full Xcode

## Current Issue

The build is using Swift stubs because `FoundationModelsMacros` compiler plugin is only available in full Xcode, not CommandLineTools.

## Solution

To enable Apple Intelligence support with full compiler plugin:

### Step 1: Point xcode-select to Full Xcode

**Option A: Interactive (requires password)**

```bash
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
```

**Option B: Via GUI**

1. Open Xcode from Applications
2. Menu: Xcode → Preferences → Locations
3. Select `/Applications/Xcode.app/Contents/Developer` from Command Line Tools dropdown
4. Click Set

### Step 2: Verify Selection

```bash
xcode-select -p
# Should print: /Applications/Xcode.app/Contents/Developer
```

### Step 3: Build Without Stub Flag

Once xcode-select is configured, unset the stub flag:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:/opt/homebrew/bin:$PATH"
unset USE_AI_STUB  # Remove the stub flag
CMAKE_POLICY_VERSION_MINIMUM=3.5 cargo check --manifest-path src-tauri/Cargo.toml
```

Build output should show:

```
cargo:warning=Building with Apple Intelligence support.
```

### Step 4: Full Build

```bash
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri build
```

## What Changes

**With Stubs (current):**

- Apple Intelligence features: ❌ Disabled
- Binary size: ~30MB
- Build time: ~3m
- Compatible with: macOS 11+

**With Full Support:**

- Apple Intelligence features: ✅ Enabled (macOS 26+)
- Binary size: ~30-35MB (slightly larger)
- Build time: ~3-4m
- Compatible with: macOS 11+ (Apple Intelligence auto-detected at runtime)

## Build Script Logic

The build script checks:

1. Does `FoundationModels.framework` exist? (checks SDK)
2. Is `USE_AI_STUB=1` set? (env override)
3. If both pass: Use real Apple Intelligence Swift code
4. Else: Use stub implementation

```rust
let source_file = if has_foundation_models && std::env::var("USE_AI_STUB").is_err() {
    println!("cargo:warning=Building with Apple Intelligence support.");
    "swift/apple_intelligence.swift"  // Real implementation
} else {
    println!("cargo:warning=Apple Intelligence SDK not available or USE_AI_STUB set. Building with stubs.");
    "swift/apple_intelligence_stub.swift"  // Stub (no-op)
};
```

## Verifying Apple Intelligence in Build

After running build, check logs for:

```
cargo:warning=Building with Apple Intelligence support.
```

If you see this, Apple Intelligence is enabled. Otherwise:

```
cargo:warning=Apple Intelligence SDK not available or USE_AI_STUB set. Building with stubs.
```

## What Apple Intelligence Does

With Apple Intelligence enabled:

- Users on macOS 26+ can use Apple Intelligence for post-processing
- Feature gracefully degrades on older macOS (weak-linked framework)
- Settings UI shows "Apple Intelligence" option
- Transcription can be enhanced using on-device Apple Intelligence model

## Development Workflow

**Quick testing with stubs (no password needed):**

```bash
export USE_AI_STUB=1
bun run tauri dev
```

**Production build with full Apple Intelligence:**

```bash
# First time only: sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
unset USE_AI_STUB
bun run tauri build
```

## Troubleshooting

**"Building with stubs" after setting xcode-select?**
→ Rebuild might have cached build directory:

```bash
cargo clean
unset USE_AI_STUB
cargo build --manifest-path src-tauri/Cargo.toml
```

**xcode-select still shows CommandLineTools?**
→ Verify Xcode is fully installed:

```bash
ls /Applications/Xcode.app/Contents/Developer/usr/bin/swift
# Should exist
```

**FoundationModelsMacros still not found?**
→ Xcode needs to be fully installed and updated:

```bash
/Applications/Xcode.app/Contents/Developer/usr/bin/swift --version
# Should show Xcode Swift compiler, not CommandLineTools
```

## Next Steps

1. **Set xcode-select** (requires sudo password once)
2. **Test build** without stub flag
3. **Verify** "Building with Apple Intelligence support." in logs
4. **Rebuild app** with full Apple Intelligence support enabled

---

**Current Status:** Xcode installed ✅, ready to enable full Apple Intelligence
