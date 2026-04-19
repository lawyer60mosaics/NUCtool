# 笔记本温控软件功能需求总览

## 一、硬件实时监控

**代表软件**：HWiNFO64、AIDA64、GPU-Z、CPU-Z、Open Hardware Monitor

### 核心功能需求

**CPU 监控**
- 各核心独立温度采样（Core Temp）
- 实时频率 / Turbo 状态
- 功耗（Package Power）
- 核心电压

**GPU 监控**
- GPU 核心温度 / 显存温度
- 显卡频率 / 显存频率
- GPU 占用率 / 功耗
- 风扇转速

**其他硬件**
- 硬盘 S.M.A.R.T. 温度
- 主板传感器（VRM、电源相温度）
- 内存占用 / 带宽
- 电池电量 / 充电功率

**展示方式**
- 桌面悬浮窗（低占用、可自定义显示项）
- 任务栏 / 系统托盘实时数值
- OSD 游戏内覆盖层（如 MSI Afterburner）
- 传感器列表支持隐藏 / 排序

---

## 二、风扇转速控制

**代表软件**：FanControl、NoteBook FanControl（NBFC）、G-Helper、各厂商原生软件

### 核心功能需求

**调速策略**
- 自定义风扇曲线（温度为横轴，转速为纵轴）
- PWM 精准调速（0–100% 占空比）
- 预设模式：安静 / 标准 / 性能 / 全速
- 混合策略（Multi-sensor Mix，取多传感器最高值）

**多风扇管理**
- CPU 风扇与 GPU 风扇独立控制
- 支持多个风扇头独立配置
- 风扇加速 / 减速斜率设置（避免转速突变）
- 极静音模式（低负载时完全停转）

**兼容性**
- 支持 PWM（4 针）和 DC（3 针）两种协议
- 通过 EC（嵌入式控制器）直接读写寄存器
- 插件扩展机制（如 FanControl + HWiNFO64 联动）

---

## 三、性能 / 功耗调节

**代表软件**：ThrottleStop、Intel XTU、Ryzen Controller、MSI Afterburner

### 核心功能需求

**功耗墙调节**
- PL1（长期功耗限制）/ PL2（短期功耗限制）设置
- Tau（PL2 持续时间）配置
- 解除 BIOS / 厂商默认的节流限制（Throttle）
- BD PROCHOT 信号控制（防止 GPU 高温联动拉低 CPU 频率）

**电压 / 频率控制**
- CPU 降压（Undervolting）：在不降频情况下降温降噪
- GPU 核心 / 显存降压
- Turbo Boost 开关
- SpeedStep / C-State 省电状态管理

**场景配置文件**
- 多套配置方案切换（如：轻办公 / 游戏 / 高性能）
- 开机自启并自动加载指定配置
- 按交流 / 电池状态自动切换方案

---

## 四、温度预警 / 日志记录

**代表软件**：HWiNFO64、AIDA64、Open Hardware Monitor

### 核心功能需求

**报警机制**
- 用户自定义温度阈值
- 触发动作：声音报警 / 弹窗通知 / 记录日志 / 强制关机
- 支持对多个传感器分别设定不同阈值

**历史数据**
- 温度 / 功耗 / 转速历史曲线图
- 压力测试期间完整数据采集
- 数据导出（CSV / 日志文件）
- 可设定采样间隔

---

## 五、厂商生态 / 整机联调

**代表软件**：华硕 Armoury Crate / G-Helper、联想 Legion Toolkit / Vantage、微星 MSI Center、戴尔 Alienware Command Center、通用第三方 NBFC / EC Fan Control

### 核心功能需求

**性能档位联动**
- 与 BIOS 内置性能模式同步（静音 / 均衡 / 性能 / 睿频）
- 插电与拔电状态自动切换档位
- 显卡工作模式切换（核显 Eco / 混合 / 独显直连）

**整机协同**
- 风扇曲线、功耗墙、RGB 灯效联动配置
- 屏幕刷新率 / 色彩配置随场景切换
- 一键应用场景预设（游戏 / 创作 / 省电）

**轻量替代方案**
- 开源第三方软件（G-Helper）以极低资源占用实现原生软件全部核心功能
- 无需安装厂商臃肿服务进程，通过 EC 寄存器直接控制硬件

---

## 各软件定位速查

| 软件 | 主要平台 | 核心定位 |
|---|---|---|
| HWiNFO64 | Windows | 最全面的硬件传感器监控 |
| AIDA64 | Windows | 监控 + 压力测试 + 日志 |
| ThrottleStop | Windows | Intel CPU 功耗 / 降压调节 |
| Intel XTU | Windows | Intel 官方超频 / 降压工具 |
| Ryzen Controller | Windows | AMD CPU 功耗调节 |
| FanControl | Windows | 最强自定义风扇曲线控制 |
| NBFC | Windows | 笔记本专用 EC 风扇控制 |
| G-Helper | Windows | 华硕笔记本轻量整机管理 |
| MSI Afterburner | Windows | GPU 频率 / 电压 / OSD |
| Armoury Crate | Windows | 华硕原生整机管理（较重） |
| Legion Toolkit | Windows | 联想拯救者第三方管理工具 |