# 模拟器公共 SDK 与坐标契约

- 契约版本：`DaedalusSimSdk 2.0.0`
- 生产分支：`main`
- 平台：Linux
- IPC：`SHM v7 / ABI revision 2`
- 图像：RGB24/RGBA32，`1440×1080`

消费者必须使用模拟器发布包中的 SDK，不得复制 `talos_v1.hpp`。

```bash
cmake -S sdk/cpp -B build/sim-sdk -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build/sim-sdk --parallel
ctest --test-dir build/sim-sdk --output-on-failure
cmake --install build/sim-sdk --prefix build/sim-sdk-install
```

```cmake
find_package(DaedalusSimSdk 2 REQUIRED CONFIG)
target_link_libraries(my_consumer PRIVATE DaedalusSimSdk::DaedalusSimSdk)
```

## 坐标系

所有坐标系均为右手系。内部 Bevy 空间与对外 ROS/SDK 空间不直接混用。

| 坐标系 | +X | +Y | +Z | 备注 |
| --- | --- | --- | --- | --- |
| Bevy 世界/节点 | 右 | 上 | 后 | 默认前方为 `-Z` |
| 对外 world/odom | 前 | 左 | 上 | ROS/REP-103 式 FLU |
| 车体 body/chassis | 前 | 左 | 上 | 线速度、角速度、RPY 均用此系 |
| 云台逻辑系 | 前 | 左 | 上 | 与 body 同轴，受 yaw/pitch 旋转 |
| 相机 optical | 右 | 下 | 前 | OpenCV/ROS optical 习惯，深度沿 `+Z` |
| 图像像素 | 右 | 下 | — | 原点在左上角 |

Bevy 向对外 FLU 的向量映射是：

```text
x_flu = -z_bevy
y_flu = -x_bevy
z_flu =  y_bevy
```

位置、线速度和角速度都使用该映射；旋转使用同一对齐矩阵做基变换。SDK 中的 `world_frame` 是 ROS odom/FLU，不是 Bevy 原生轴。

## 云台角

- `yaw_deg`：绝对偏航角，绕逻辑 `+Z` 轴；从上向下看，正值向左转。
- `pitch_deg`：绝对俯仰角，`0°` 是光轴水平，正值抬头，负值低头。
- 当前配置 `gimbal_pitch_limit=0.785 rad`，所以有效范围约为 `[-45°,+45°]`，包含端点。
- UDP、ROS2、Talos 命令、`readGimbalState()` 和曝光同步状态全部使用该定义。
- 超范围命令被对称 clamp；不做旧版偏移兼容。

## 公共 ABI 与传输

- `ShmHeader`：magic、版本、结构大小、生产者 epoch。
- `ImageMeta`、`CameraInfo`、曝光位姿、`GroundTruthBatch`、`RuntimeState`、`GimbalCommand`。
- 元数据、真值与状态：`TALOS_IPC_DIR` 指定的 SDK IPC。
- 图像：TCP `5602`，latest-only。
- 实时云台/发射命令：UDP `5601`。
- 场景控制：UDP `5603`，协议 `daedalus.scene-control/2`。

消费者打开 IPC 后必须检查兼容性。跨流关联必须同时校验 `producer_epoch`、`frame_seq` 和 `timestamp_ns`。

## 发射语义

`fire_advice=true` 是开火请求，不是发射成功回执。所有入口共用 `ProjectileCooldown`；最大物理射频 20 Hz，即最短冷却 0.05 s。
