# Daedalus 模拟器

RoboMaster 视觉算法验证模拟器。本仓库维护 Linux 原生渲染、物理、场景、采集、真值、控制、SDK 和发布链路。

本项目基于 [Blackjack200/bevy_robomaster_simulator](https://github.com/Blackjack200/bevy_robomaster_simulator) 发展，原始作者、许可证和派生声明见 [NOTICE.md](NOTICE.md)。

## 当前契约

- 模拟器与 SDK：`2.0.0`
- 图像：RGB24/RGBA32 `1440×1080`
- 物理：250 Hz；高性能采集上限 200 Hz；物理发射上限 20 Hz
- pitch：`0°` 水平，正值抬头，负值低头，默认约 `[-45°,+45°]`
- 平台：Linux；Windows/PowerShell/WSL 集成启动链路已退役

坐标系、接口和角度语义见 [SIMULATOR_INTERFACE.md](agent-team/SIMULATOR_INTERFACE.md)。

## 启动

```bash
./start-simulator.sh
./start-simulator.sh --target-only
./start-simulator.sh --visible
```

默认是完整场景的无窗口高性能模式，加载 `config.performance.toml`，禁用 UI，并显式使用
TCP 原生 RGBA 图像链路。`--target-only` 只渲染当前目标以获得更高帧率；`--visible`
加载 `config.toml` 并创建交互窗口。兼容旧文件 RGB/深度链路时可显式加
`--file-transport`。

## 构建和打包

```bash
cargo test
./build-package.sh release
```

发布流程见 [RELEASE.md](RELEASE.md)，SDK 见 [sdk/README.md](sdk/README.md)，性能口径见 [SIMULATOR_PERFORMANCE.md](SIMULATOR_PERFORMANCE.md)，排障见 [SIMULATOR_TROUBLESHOOTING.md](SIMULATOR_TROUBLESHOOTING.md)。
