# 模拟器场景控制接口

- 上下文版本：`CTX-SIM-2026.10-v3`
- SDK：`DaedalusSimSdk 2.0.0`
- 协议：`daedalus.scene-control/2`
- 默认端点：`127.0.0.1:5603/udp`

消费者运行时控制统一使用 `SceneControlClient`，不得直接修改模拟器内部资源。端点可由 `DAEDALUS_SCENE_CONTROL_BIND` 覆盖。请求和响应均携带 `command_id`、`session_id`；响应返回状态、应用帧、时间戳和消息。

公共操作包括：

- `ping / create_session / status`
- `set_scene / reset_scene`
- `set_range_target_motion / set_range_target_geometry`
- `set_rune_state / set_rune_scenario / get_big_rune_score`

具体可用场景和能力以 `sdk/contract.json` 及运行时能力文件为准。所有 World 修改在 Bevy 主线程执行；场景切换和 reset 的 ACK 在实体重建完成后发送。非法参数、缺少会话、会话不匹配和实体未就绪都必须返回明确错误。

初始状态可通过稳定启动参数设置，包括场景、确定性种子、自动生成和靶场参数。云台命令仍使用独立的 `GimbalCommand`，pitch 定义见 `SIMULATOR_INTERFACE.md`。

任意实体生成/销毁、暂停/单步、运行时相机和光照变更尚未进入公共契约。新能力必须先由模拟器升级协议和 SDK，不得由消费者建立私有旁路。
