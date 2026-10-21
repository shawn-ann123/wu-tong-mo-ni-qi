#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

BINARY="target/release/daedalus"
BUILD_MODE="release"
VISIBLE=0
TARGET_ONLY=0
FILE_TRANSPORT=0
IPC_DIR="${TALOS_IPC_DIR:-$SCRIPT_DIR/runtime/talos-ipc}"

usage() {
    cat <<'EOF'
Usage: ./start-simulator.sh [OPTIONS]

Options:
  --debug         Use debug build instead of release
  --visible       Enable visible window mode (default: performance)
  --target-only   Render only the active target in performance mode (maximum FPS)
  --file-transport
                  Use compatibility file RGB/depth transport instead of TCP
  --ipc-dir PATH  Set TALOS IPC directory (default: ./runtime/talos-ipc)
  --skip-build    Skip build step, run existing binary
  --help, -h      Show this help

GPU selection is automatic: NVIDIA > Intel > CPU
Intel GPUs use Vulkan (ANV) with UMA optimization.

Debug window controls:
  ` (backtick)    Toggle debug panel window
  F1              Spawn point selection
  F2              Screenshot
  F3              Camera mode cycle
  F4              Telemetry panel
  F5              Auto-aim toggle
  F6              Shooting range control
  F8/F9           Scene mode shortcuts
EOF
}

SKIP_BUILD=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug) BUILD_MODE="debug"; BINARY="target/debug/daedalus"; shift ;;
        --visible) VISIBLE=1; shift ;;
        --target-only) TARGET_ONLY=1; shift ;;
        --file-transport) FILE_TRANSPORT=1; shift ;;
        --ipc-dir)
            [[ $# -ge 2 ]] || { echo "--ipc-dir requires a value" >&2; exit 2; }
            IPC_DIR="$2"; shift 2 ;;
        --skip-build) SKIP_BUILD=1; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ "$SKIP_BUILD" -eq 0 ]]; then
    echo "=== Building Daedalus Simulator (Intel iGPU Optimized) ==="
    echo "Build mode: $BUILD_MODE"

    source ~/.cargo/env 2>/dev/null || true
    if ! command -v cargo &> /dev/null; then
        echo "Error: cargo not found. Install Rust from https://rustup.rs/" >&2
        exit 1
    fi

    if [ "$BUILD_MODE" = "release" ]; then
        cargo build --release
    else
        cargo build
    fi
    echo "Build complete."
fi

[[ -x "$BINARY" ]] || { echo "Error: binary not found at $BINARY" >&2; exit 1; }

mkdir -p "$IPC_DIR"
export TALOS_IPC_DIR="$IPC_DIR"
export WGPU_POWER_PREF="high"
export RUST_LOG=info

if [[ "$VISIBLE" == 1 ]]; then
    export DAEDALUS_CONFIG="$SCRIPT_DIR/config.toml"
    unset DAEDALUS_PERF_DISABLE_UI
    unset DAEDALUS_CAPTURE_SCENE_PROFILE
    RUN_MODE="visible"
else
    export DAEDALUS_CONFIG="$SCRIPT_DIR/config.performance.toml"
    export DAEDALUS_PERF_DISABLE_UI=1
    if [[ "$TARGET_ONLY" == 1 ]]; then
        export DAEDALUS_CAPTURE_SCENE_PROFILE=aim
        RUN_MODE="performance/aim"
    else
        export DAEDALUS_CAPTURE_SCENE_PROFILE=full
        RUN_MODE="performance/full-scene"
    fi
fi

if [[ "$FILE_TRANSPORT" == 1 ]]; then
    export DAEDALUS_TALOS_IMAGE_TRANSPORT=file
    IMAGE_TRANSPORT="file-rgb-depth"
else
    export DAEDALUS_TALOS_IMAGE_TRANSPORT=tcp
    IMAGE_TRANSPORT="tcp-native-rgba"
fi

if command -v vulkaninfo &> /dev/null; then
    echo "=== Vulkan Devices ==="
    vulkaninfo --summary 2>/dev/null | grep -A5 "GPU" || true
    echo ""
fi

echo "=== Daedalus Simulator (Intel iGPU Optimized) ==="
echo "GPU auto-selection: NVIDIA > Intel > CPU"
if [[ "$VISIBLE" == 1 ]]; then
    echo "Debug window: press \` (backtick) to toggle"
fi
echo "Mode=$RUN_MODE  Transport=$IMAGE_TRANSPORT"
echo "Config=$DAEDALUS_CONFIG  IPC=$TALOS_IPC_DIR"
echo ""

exec "$BINARY"
