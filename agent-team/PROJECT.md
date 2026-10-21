# 模拟器模块上下文

- 上下文版本：`CTX-SIM-REPO-2026.10-v3`
- 仓库：`daedalus-intel-optimized`
- 分支：`main`
- 平台：Linux
- 模拟器版本：`Daedalus Simulator 2.0.0`
- 公共 SDK：`DaedalusSimSdk 2.0.0`
- IPC 布局：`SHM v7 / ABI revision 2`
- 固定图像规格：RGB24/RGBA32，`1440×1080`

本仓库是渲染、物理、场景、采集、真值、控制通道、性能、SDK 和发布包的唯一权威实现。消费者只能依赖带版本的发布包和公共 SDK，不得复制 IPC 结构或模拟器源码。

2.0.0 的破坏性语义变更是统一云台俯仰角：水平为 `pitch=0°`，正值抬头，负值低头，配置的 `gimbal_pitch_limit=0.785 rad` 对应约 `[-45°,+45°]`。1.x 的俯仰语义已废止，不得在转换层、SDK 或启动器中恢复偏移。

当前只维护 Linux 启动与发布链路。Windows/PowerShell/WSL 集成启动器不属于当前支持范围。

公共入口：

- SDK 契约：`sdk/contract.json`
- C++ 头文件：`sdk/cpp/include/daedalus_sim_sdk/`
- 接口与坐标系：`agent-team/SIMULATOR_INTERFACE.md`
- 场景控制：`agent-team/SCENARIO_CONTROL.md`
- 构建与性能：`SIMULATOR_PERFORMANCE.md`
- 故障排查：`SIMULATOR_TROUBLESHOOTING.md`
- 发布：`RELEASE.md`
