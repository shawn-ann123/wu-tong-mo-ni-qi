#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

source ~/.cargo/env 2>/dev/null || true

echo "=== Building Daedalus Simulator (Intel iGPU Optimized) ==="

if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

BUILD_TYPE="${1:-release}"
VERSION_VALUE="$(tr -d '[:space:]' < VERSION)"

if [ "$BUILD_TYPE" = "release" ]; then
    echo "Building in release mode..."
    cargo build --release
    BINARY="target/release/daedalus"
else
    echo "Building in debug mode..."
    cargo build
    BINARY="target/debug/daedalus"
fi

if [ ! -f "$BINARY" ]; then
    echo "Error: Build failed, binary not found at $BINARY"
    exit 1
fi

echo "Build successful: $BINARY"

PACKAGE_DIR="package"
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR/bin"
mkdir -p "$PACKAGE_DIR/runtime/talos-ipc"
mkdir -p "$PACKAGE_DIR/assets"

cp "$BINARY" "$PACKAGE_DIR/bin/daedalus"
cp -r assets/* "$PACKAGE_DIR/assets/" 2>/dev/null || true

if [ -f "config.toml" ]; then
    cp config.toml "$PACKAGE_DIR/"
fi

if [ -f "config.performance.toml" ]; then
    cp config.performance.toml "$PACKAGE_DIR/"
fi

rm -rf "$PACKAGE_DIR/sdk"
cmake -S sdk/cpp -B build/sim-sdk -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build/sim-sdk --parallel
ctest --test-dir build/sim-sdk --output-on-failure
cmake --install build/sim-sdk --prefix "$PACKAGE_DIR/sdk"
cp sdk/contract.json "$PACKAGE_DIR/sdk/"
cp sdk/README.md "$PACKAGE_DIR/sdk/"

cat > "$PACKAGE_DIR/start-simulator.sh" << 'EOF'
#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

export RUST_LOG=info
export TALOS_IPC_DIR="${TALOS_IPC_DIR:-$SCRIPT_DIR/runtime/talos-ipc}"
mkdir -p "$TALOS_IPC_DIR"

if [[ "${1:-}" == "--visible" ]]; then
    export DAEDALUS_CONFIG="$SCRIPT_DIR/config.toml"
    unset DAEDALUS_PERF_DISABLE_UI
    shift
    RUN_MODE="visible"
else
    export DAEDALUS_CONFIG="$SCRIPT_DIR/config.performance.toml"
    export DAEDALUS_PERF_DISABLE_UI=1
    RUN_MODE="performance"
fi

if command -v vulkaninfo &> /dev/null; then
    echo "=== Vulkan Devices ==="
    vulkaninfo --summary 2>/dev/null | grep -A5 "GPU" || true
    echo ""
fi

echo "Starting Daedalus Simulator (Intel iGPU Optimized)..."
echo "GPU selection is automatic: NVIDIA > Intel > CPU"
echo "Mode=$RUN_MODE  Config=$DAEDALUS_CONFIG  IPC=$TALOS_IPC_DIR"
echo ""

exec ./bin/daedalus "$@"
EOF
chmod +x "$PACKAGE_DIR/start-simulator.sh"

cat > "$PACKAGE_DIR/release.json" << EOF
{
  "product": "daedalus-simulator-intel-optimized",
  "version": "$VERSION_VALUE",
  "sdk_version": "$VERSION_VALUE",
  "platform": "linux",
  "gimbal_pitch": {
    "unit": "degree",
    "level_deg": 0.0,
    "positive": "up",
    "negative": "down",
    "range_deg": [-45.0, 45.0]
  },
  "gpu_optimization": {
    "auto_adapter_selection": true,
    "priority_order": ["nvidia_discrete", "intel_discrete", "amd_discrete", "intel_integrated", "cpu"],
    "intel_vulkan_forced": true,
    "uma_optimization": true,
    "memory_hints": "MemoryUsage"
  },
  "image": {
    "default_transport": "tcp",
    "format": "rgba32",
    "channels": 4,
    "width": 1440,
    "height": 1080
  },
  "physics_hz": 250
}
EOF

echo ""
echo "=== Package created at: $PACKAGE_DIR ==="
echo "Run with: cd $PACKAGE_DIR && ./start-simulator.sh"
