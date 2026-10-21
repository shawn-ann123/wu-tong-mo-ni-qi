# Daedalus Simulator 2.0.0 Linux 排障

本文只面向当前 Linux 启动链路。Windows/PowerShell/WSL 流程已退役。

## 先确认启动模式

```bash
./start-simulator.sh
./start-simulator.sh --visible
```

启动器会打印 `Mode`、`Config` 和 `IPC`。默认高性能模式没有窗口是正常现象，应根据采集计数、TCP 连接和消费者输入判断是否工作。

## 常见问题

| 现象 | 检查 | 处理 |
| --- | --- | --- |
| 默认模式无窗口 | `DAEDALUS_CONFIG`、`DAEDALUS_PERF_DISABLE_UI` | 这是预期行为；要看画面时用 `--visible` |
| `5602/5603` 绑定失败 | `ss -ltnp` / `ss -lunp` | 查明端口所有者，只停止已确认的旧实例 |
| TCP 有监听但没有图像 | 采集计数、TCP 连接、发送计数 | 按采集→连接→发送→消费者的顺序隔离 |
| 云台俯仰严重偏移 | 消费者是否仍做旧版偏移补偿 | 删除补偿；2.0.0 中 `0°` 就是水平 |
| 俯仰命令超过约 45° 不再增加 | `vehicle.gimbal_pitch_limit` | 这是对称限位 clamp |
| 看到约 60 FPS 就判定采集不足 | 预览与离屏采集指标 | 分别报告 `preview_present_hz` 和 `capture_copy_submit_hz` |

## pitch 快速自检

1. 发送 `pitch_deg=0`，光轴应水平。
2. 发送小正值，光轴应抬高。
3. 发送小负值，光轴应降低。
4. UDP、ROS2、Talos 与 SDK 状态反馈应同号、同零点。

如果消费者调试指向模拟器缺陷，先提交可复现步骤、版本、日志、影响和建议公共接口；获得用户明确批准后再修改模拟器。
