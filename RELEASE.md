# Daedalus Linux 发布流程

`VERSION`、`sdk/contract.json` 和 C++ SDK 的 `kSdkVersion` 必须一致。pitch 契约是 2.0.0 的主版本语义：`0°` 水平，正抬头，负低头。

## 构建与测试

```bash
cargo build --release
cargo test

cmake -S sdk/cpp -B build/sim-sdk -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=ON
cmake --build build/sim-sdk --parallel
ctest --test-dir build/sim-sdk --output-on-failure
```

## 生成目录包

```bash
./build-package.sh release
```

`package/` 包含 Linux 二进制、两套配置、资产、启动脚本、完整 C++ SDK 和 `release.json`。默认运行 `package/start-simulator.sh`；可见验收使用 `package/start-simulator.sh --visible`。

发布前还必须：

1. 检查 `git diff --check` 和工作树状态。
2. 校验 SDK 测试、UDP/ROS2/Talos pitch 语义和两种启动模式。
3. 为发布目录生成 SHA256 清单并记录源提交。
4. 只在用户明确授权后创建或推送 `simulator-v<version>` 标签。
