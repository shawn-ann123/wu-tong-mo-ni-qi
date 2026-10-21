# Daedalus Simulator 2.0.0 Linux 性能口径

本文档定义当前可用的复现方式。`benchmarks/1.0.0/` 中的 Windows 结果只是历史证据，不是 2.0.0 Linux 基线，不得用于当前性能承诺。

## 固定条件

- Release 构建，Linux/Vulkan。
- RGB24/RGBA32 `1440×1080`。
- 物理 250 Hz，高性能采集上限 200 Hz。
- TCP `5602` latest-only 图像；UDP `5601` 云台；UDP `5603` 场景控制。
- 发射物理上限 20 Hz。

## 模式

```bash
./start-simulator.sh
./start-simulator.sh --target-only
./start-simulator.sh --visible
```

默认模式加载 `config.performance.toml`、禁用 UI、使用 TCP 原生 RGBA latest-only 传输，
并保留完整场馆图像。需要最大帧率且算法只需要当前目标时使用 `--target-only`；该模式保留
250 Hz 物理、真值和控制，但图像中不包含完整场馆。可见模式加载 `config.toml`。只有兼容
旧的文件 RGB/深度消费者时才使用 `--file-transport`，该路径会增加 GPU 深度复制和 CPU
文件映射开销。

性能模式不会创建主窗口、调试窗口或对应的 UI 相机。启动脚本显式设置图像传输和采集场景，
因此直接运行二进制时的兼容性默认值不代表正式高性能启动定义。

## 本机优化实测

2026-10-21 在 Intel Core Ultra 5 225H / Intel(R) Graphics (ARL)、Mesa 26.0.8、
1440×1080、物理 250 Hz、无 TCP 消费者条件下预热后测得：

- 旧启动实际落入文件 RGB/深度全场景路径：约 28 Hz。
- TCP 原生 RGBA 全场景：约 30 Hz。
- 优化后的 TCP 原生 RGBA 完整场景：约 95 Hz，物理约 249 Hz。
- 优化后的 TCP 原生 RGBA `--target-only`：约 124 Hz，物理约 249 Hz。

优化后的 1440 像素宽图像允许带行 padding 的 GPU 映射进入 UMA 快速路径，由 8 个有界
读回槽并行完成一次式去 padding；旧路径每帧需要两次整图复制，且这条快速路径实际未启用。
上述两组优化后实测的采集队列丢帧和 GPU map 错误均为 0。

这些数值是当前开发机的诊断结果，不是跨机器性能承诺。连接真实消费者后还必须同时记录
`talos_tcp_image_sent_hz`；无消费者时它为 0，只能用 `capture_copy_submit_hz` 衡量生产端。

## 报告要求

每次基线必须记录版本/提交、GPU 与驱动、配置哈希、预热时间、采样窗口和后台负载，并同时报告：

- `main_update_hz`
- `capture_copy_submit_hz`
- `talos_tcp_image_sent_hz`
- `preview_present_hz`
- 采集队列丢帧、GPU map 错误、TCP 错误
- 消费者完整视觉频率和曝光身份匹配情况

200 Hz 是配置上限，可见预览频率也不等于离屏采集频率。在 Linux 2.0.0 正式重测并写入带哈希的机器可读结果前，不声明新的数值基线。
