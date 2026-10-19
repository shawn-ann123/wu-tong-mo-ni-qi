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

cat > "$PACKAGE_DIR/start-simulator.sh" << 'EOF'
#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

export RUST_LOG=info

if command -v vulkaninfo &> /dev/null; then
    echo "=== Vulkan Devices ==="
    vulkaninfo --summary 2>/dev/null | grep -A5 "GPU" || true
    echo ""
fi

echo "Starting Daedalus Simulator (Intel iGPU Optimized)..."
echo "GPU selection is automatic: NVIDIA > Intel > CPU"
echo ""

exec ./bin/daedalus "$@"
EOF
chmod +x "$PACKAGE_DIR/start-simulator.sh"

cat > "$PACKAGE_DIR/release.json" << EOF
{
  "product": "daedalus-simulator-intel-optimized",
  "version": "1.4.0-intel-uma-r1",
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
