# Daedalus Simulator - Intel iGPU 优化版

基于 `daedalus-simulator` 源码，针对 Intel 核显 (iGPU) 进行优化。

## 主要改动

### 1. 自动 GPU 适配器选择

启动时自动检测系统中所有 GPU，按以下优先级选择渲染器：

| 优先级 | GPU 类型 | 说明 |
|--------|----------|------|
| 0 | NVIDIA 独显 | GeForce / RTX / GTX / Quadro |
| 1 | Intel 独显 | Arc 系列 |
| 2 | AMD 独显 | Radeon / RX |
| 3 | 其他独显 | - |
| 4 | **Intel 核显** | Arc / Iris / UHD / HD Graphics |
| 5 | 其他核显 | - |
| 6 | CPU 软渲染 | 最后兜底 |

同一优先级内，Vulkan 后端优先。

### 2. Intel Vulkan 强制

检测到 Intel GPU 时，自动强制使用 Vulkan 后端（ANV 驱动），避免 wgpu 选到 OpenGL 或其他次优后端。

### 3. UMA 共享内存优化

Intel 核显使用统一内存架构 (UMA)，CPU 和 GPU 共享同一物理内存。优化措施：

- **`MemoryHints::MemoryUsage`**：提示 wgpu 优化内存分配策略
- **增加 readback 缓冲区数量**：UMA 下从 3 个增加到 5 个 fast readback buffer，减少同步等待
- **零拷贝 map_async**：UMA 下 staging buffer 的 map 操作本质上是内存映射，无 PCIe 传输开销

### 4. 启动日志

启动时会打印所有检测到的 GPU 候选及最终选择：

```
GPU candidate: Intel(R) Arc(TM) Pro 140T (vendor=IntegratedGpu, tier=IntelIntegrated, vulkan=true, uma=true)
Selected GPU: Intel(R) Arc(TM) Pro 140T (tier=IntelIntegrated, uma=true)
Intel GPU detected (Intel(R) Arc(TM) Pro 140T), forcing Vulkan backend for optimal UMA performance
UMA adapter detected (Intel(R) Arc(TM) Pro 140T)): readback path will avoid unnecessary copies
```

## 编译

需要 Rust 工具链。安装：https://rustup.rs/

```bash
cd daedalus-intel-optimized

# Release 编译
cargo build --release

# 或者使用打包脚本
./build-package.sh release
```

## 运行

```bash
# 直接运行
cargo run --release

# 或使用打包后的启动脚本
cd package && ./start-simulator.sh
```

## 环境变量

| 变量 | 说明 |
|------|------|
| `DAEDALUS_WINDOW_WIDTH` | 窗口宽度 |
| `DAEDALUS_WINDOW_HEIGHT` | 窗口高度 |
| `DAEDALUS_CAPTURE_SCENE_PROFILE` | `full`（默认）或 `aim`（仅渲染目标，性能更好）|
| `DAEDALUS_PERF_DISABLE_UI` | `1` 禁用 UI 窗口（纯 headless） |

## 与完整版的区别

| 特性 | 完整版 | Intel 优化版 |
|------|--------|--------------|
| GPU 选择 | 默认（可能选到软渲染） | 自动优先级选择 |
| Intel Vulkan | 不保证 | 强制 |
| UMA 优化 | 无 | readback 缓冲区增加、内存提示 |
| 启动日志 | 无 GPU 信息 | 详细 GPU 检测和选择日志 |

## 源码结构

```
src/
├── gpu_selector.rs    # 新增：GPU 检测和适配器选择
├── capture/
│   └── driver.rs      # 修改：UMA 感知的 readback 缓冲区池
└── main.rs            # 修改：使用 gpu_selector 替代硬编码的渲染插件
```
