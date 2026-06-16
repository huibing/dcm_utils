# DCM Utils

[English](README.md) | 中文

一款基于 Rust 的命令行工具，用于解析、操作和比较汽车 ECU 标定中使用的 DCM（DAMOS Calibration Memory）文件。

## 概述

DCM Utils 是专为汽车工程师设计的标定数据工具，全面支持 ASAM DAMOS 格式 DCM 文件的读取、合并、更新、过滤和比较。

## 功能特性

- **📖 解析 DCM 文件**：读取和解析 DCM 文件，支持所有标准块类型
- **🔀 合并**：将多个 DCM 文件合并为一个，以第一个文件为基础
- **🔄 更新**：将一个或多个 DCM 文件的标定变更应用到基础文件
- **🔍 过滤**：使用正则表达式包含或排除变量
- **📊 对比**：比较两个 DCM 文件的差异
- **🌐 Diff-Base Web UI**：交互式多源对比网页，支持 MAP 合并/分离视图和跨源单元格高亮
- **🔧 生成**：从 A2L 标定描述和 Intel HEX 刷写镜像生成 DCM 文件
- **📝 输出**：使用 Handlebars 模板生成格式规范的 DCM 文件

## 支持的 DCM 块类型

| 块类型 | 说明 |
|--------|------|
| `FESTWERT` | 单个常量值 |
| `FESTWERTEBLOCK` | 常量数组（一维） |
| `GRUPPENKENNLINIE` | 一维查找表/曲线 |
| `STUETZSTELLENVERTEILUNG` | 轴断点分布 |
| `GRUPPENKENNFELD` | 二维 MAP/表格 |

## 安装

### 前置条件

- Rust 工具链（1.94.0 或更高版本）
- Cargo 包管理器

### 从源码构建

```bash
# 克隆仓库
git clone git@github.com:huibing/dcm_utils.git
cd dcm_utils

# Debug 模式构建
cargo build

# Release 模式构建（优化后）
cargo build --release
```

编译后的二进制文件位于：
- Debug：`target/debug/dcm_utils`
- Release：`target/release/dcm_utils`

## 使用方法

### 命令概览

```bash
dcm_utils <COMMAND> [OPTIONS]
```

可用命令：
- `merge` - 合并多个 DCM 文件
- `update` - 使用其他文件的数据更新 DCM 文件
- `filter` - 使用正则表达式过滤变量
- `diff` - 比较两个 DCM 文件
- `diff-base` - 将基准源与多个其他源进行对比
- `gen` - 从 A2L 和 HEX 标定文件生成 DCM 文件
- `help` - 显示帮助信息

### merge 命令

将多个 DCM 文件合并为一个，以第一个文件为基础。

```bash
dcm_utils merge file1.DCM file2.DCM file3.DCM -o merged.DCM
```

**行为：**
- 第一个文件中的变量作为基础保留
- 后续文件中不存在于基础的变量会被添加
- 已有变量不会被覆盖（如需覆盖请使用 `update`）

**选项：**
- `-o, --output <OUTPUT>` - 输出文件路径（默认：`merged.dcm`）

### update 命令

使用其他 DCM 文件的数据更新第一个 DCM 文件。

```bash
dcm_utils update base.DCM updates.DCM -o updated.DCM
```

**行为：**
- 只更新基础文件中已存在的变量
- 更新文件中的新变量会被忽略
- 可以指定多个更新文件

**选项：**
- `-o, --output <OUTPUT>` - 输出文件路径（默认：`updated.dcm`）

### filter 命令

使用包含或排除正则表达式过滤 DCM 变量。

```bash
# 仅包含匹配模式的变量
dcm_utils filter input.DCM --include "VAR.*" "CFG.*" -o filtered.DCM

# 排除匹配模式的变量
dcm_utils filter input.DCM --exclude "Temp.*" "Test.*" -o filtered.DCM
```

**选项：**
- `-i, --include <PATTERNS>` - 仅包含匹配的变量
- `-e, --exclude <PATTERNS>` - 排除匹配的变量
- `-o, --output <OUTPUT>` - 输出文件路径（默认：`filtered.dcm`）

**注意：** 必须提供 `--include` 或 `--exclude` 之一，但不能同时使用。

### diff-base 命令

将基准标定源的变量与多个其他源进行对比。仅对比基准源中存在的变量，不在基准中的变量会被忽略。所有源之间的变量比较使用 f32 字节级对比。

```bash
# 将基准 DCM 与另外两个 DCM 文件对比
dcm_utils diff-base --base base.DCM --other source1.DCM --other source2.DCM

# 将 A2L+HEX 基准与 DCM 文件对比
dcm_utils diff-base --base --a2l cal.a2l -x flash.hex --other modified.DCM
```

**输出：**
- 控制台摘要，列出变量总数和有差异的变量
- JSON 文件，包含每个变量、每个源的值
- 可选网页（`--web`），带标注的源名称（Base、Source 1、Source 2...）

**Web UI 功能：**
- 默认仅显示有差异的变量；切换到"All"可查看所有变量
- 按变量名或块类型（FESTWERT、GRUPPENKENNFELD 等）过滤
- 点击变量名打开详情弹窗
- **MAP（二维）变量**提供两种视图：
  - **Merged（合并）**（默认）：所有源的值在一个网格中上下排列，按源着色并显示颜色图例；轴点（X/Y）同样合并显示，轴差异一目了然
  - **Separated（分离）**：每个源一个网格，纵向堆叠；悬停单元格时自动高亮其他源网格中对应位置的单元格
- 与基准源相同的值显示为暗色；不同的值加粗高亮显示

**选项：**
- `--base <PATH>` - 基准 DCM 文件
- `--base-a2l <PATH>` - 基准 A2L 文件（与 `--base-hex` 配对）
- `--base-hex <PATH>` - 基准 Intel HEX 镜像（与 `--base-a2l` 配对）
- `--other <PATH>` - 其他 DCM 文件（可重复）
- `--other-a2l <PATH>` - 其他 A2L 文件（与 `--other-hex` 配对，可重复）
- `--other-hex <PATH>` - 其他 Intel HEX 镜像（与 `--other-a2l` 配对）
- `-o, --output <OUTPUT>` - 输出 JSON 文件路径（默认：`diff-base.json`）
- `--show <MODE>` - 显示模式：`diff`（仅差异）或 `all`（所有基准变量，默认：`all`）
- `--web` - 以网页形式展示对比结果

### diff 命令

比较 DCM 文件和 A2L+HEX 对之间的标定数据。对比引擎将所有数据源统一归一化为 `DcmData` 后再进行比较，支持跨格式对比。

```bash
# DCM vs DCM
dcm_utils diff --dcm original.DCM --dcm modified.DCM -o diff.json

# DCM vs A2L+HEX（DCM 在前）
dcm_utils diff --dcm ref.DCM --a2l calibration.a2l -x flash.hex -o diff.json

# A2L+HEX vs A2L+HEX
dcm_utils diff --a2l v1.a2l -x v1.hex --a2l v2.a2l -x v2.hex -o diff.json
```

**浮点数比较：**
- 值被转换为单精度（f32）后按字节逐一比较
- 低于 f32 精度的差异（如 `1.0` 与 `1.0 + 1e-9`）视为相等
- 这与标定数据在 ECU Flash 中的实际存储方式一致

**输出：**
- 彩色控制台统计摘要
- JSON 文件包含详细差异：
  - `New` - 仅在右侧源中存在的变量
  - `Deleted` - 仅在左侧源中存在的变量
  - `Changed` - 值不同的变量
  - `ChangedMap` - 有差异的二维 MAP（完整 JSON 表示）

**选项：**
- `--dcm <PATH>` - DCM 文件数据源（可重复，每侧一个）
- `--a2l <PATH>` - A2L 标定描述文件（与同侧 `--hex` 配对）
- `-x, --hex <PATH>` - Intel HEX 刷写镜像（与同侧 `--a2l` 配对）
- `-o, --output <OUTPUT>` - 输出 JSON 文件路径（默认：`diff.json`）
- `--web` - 以网页形式展示对比结果

**输出示例：**
```
=== Calibration Diff Results ===
Left:  test-dcms/test1.DCM
Right: test-dcms/simple_test.a2l + test-dcms/simple_test.hex
Timestamp: 1779877997

New blocks: 4
Deleted blocks: 10
Changed blocks: 0
Total differences: 14

Diff details written to: diff.json
```

### gen 命令

从 A2L 标定描述和 Intel HEX 刷写镜像生成 DCM 文件。

```bash
dcm_utils gen --a2l calibration.a2l -x flash.hex -o all_cali.DCM
```

**行为：**
- 从 A2L 文件和 HEX 二进制中提取所有标定特征
- 将 A2L 类型映射为 DCM 块类型（VALUE→FESTWERT, CURVE→GRUPPENKENNLINIE+轴, MAP→GRUPPENKENNFELD+轴, VAL_BLK→FESTWERTEBLOCK, ASCII→FESTWERT）
- 提取失败的条目会跳过并在 stderr 输出摘要
- 包含非数值（verbal）值的 CURVE/MAP 块会跳过并输出警告
- 数值与非数值混合的 VAL_BLK 块整体转换为 TEXT 格式

**选项：**
- `-a, --a2l <A2L>` - A2L 标定描述文件路径（必填）
- `-x, --hex <HEX>` - Intel HEX 刷写镜像路径（必填）
- `-o, --output <OUTPUT>` - 输出 DCM 文件路径（默认：`generated.dcm`）

## DCM 文件格式

DCM（DAMOS Calibration Memory）文件遵循 ASAM 标准格式。以下是示例结构：

```text
* encoding="UTF-8"
* DAMOS format
* Created by CDM V7.2.17 Build 86
* Creation date: 2025/2/20 19:34:19
*
* Project: Example_Project
* Dataset: example.DCM

KONSERVIERUNG_FORMAT 2.0

* no memory layouts specified

FESTWERT VAR_0001
   LANGNAME "Control enable flag"
   EINHEIT_W "unitless"
   WERT 1.0000000000000000
END

GRUPPENKENNLINIE VAR_0002 9
   LANGNAME "Calibration lookup table"
   EINHEIT_X "percent"
   EINHEIT_W "mA"
*SSTX	VAR_0003
   ST/X   0.0000000000000000   12.5000000000000000   25.0000000000000000
   ST/X   37.5000000000000000   50.0000000000000000   62.5000000000000000
   WERT   320.0000000000000000   480.0000000000000000   640.0000000000000000
   WERT   800.0000000000000000   960.0000000000000000   1120.0000000000000000
END
```

### 块结构

每个标定参数定义为一个块，包含：
- **块头**：块类型和名称
- **LANGNAME**：描述/标签
- **EINHEIT_W/X/Y**：值和轴的单位
- **值**：WERT（数值）或 TEXT（字符串）
- **END**：块结束标记

## 架构

### 模块结构

```
src/
├── main.rs          # CLI 入口
├── lib.rs           # 库核心（DcmData、I/O 操作）
├── block.rs         # Block 枚举（统一接口）
├── value.rs         # Value 枚举（WERT/TEXT）
├── diff.rs          # 对比功能
├── gen.rs           # A2L+HEX 转 DCM 生成
├── blocks/          # 块类型实现
│   ├── festwert.rs
│   ├── festwerteblock.rs
│   ├── gruppenkennlinie.rs
│   ├── stuetzstellenverteilung.rs
│   └── gruppenkennfeld.rs
└── attr/            # 属性解析
    ├── string_attr.rs
    ├── value_attr.rs
    └── attr_arbitor.rs
```

### 核心组件

- **DcmData**：主数据结构，使用 `IndexMap` 存储所有标定块
- **Block 枚举**：所有块类型的统一接口，提供通用操作
- **Value 枚举**：表示数值（`WERT`）或文本（`TEXT`）值
- **Handlebars 模板**：用于格式化输出 DCM 文件

### 数据流

1. **解析**：`DcmData::new(path)` 读取并解析 DCM 文件为块
2. **存储**：块存储在 `IndexMap<String, Block>` 中（保持插入顺序）
3. **操作**：merge、update、filter 等操作修改块映射
4. **渲染**：`write_dcm_data()` 使用模板生成输出

## 开发

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行指定测试
cargo test test_festwert

# 运行测试并显示输出
cargo test -- --nocapture
```

### 代码质量

```bash
# 运行 clippy（通过 .clippy.toml 配置）
cargo clippy

# 格式化代码
cargo fmt
```

### 项目配置

- **Rust 版本**：1.94.0 或更高
- **Clippy**：配置 `too-many-arguments-threshold = 12`

### 核心依赖

| Crate | 用途 |
|-------|------|
| `indexmap` | 有序映射，保持块顺序一致 |
| `handlebars` | 模板引擎，用于 DCM 文件生成 |
| `clap` | 使用 derive 宏的 CLI 参数解析 |
| `serde_json` | JSON 序列化，用于对比输出 |
| `regex` | 正则匹配，用于过滤命令 |
| `a2ldeser` | A2L+HEX 标定数据提取 |
| `a2lfile` | A2L 文件解析器 |
| `colored` | 终端彩色输出 |
| `rstest` | 参数化测试框架 |

## 示例

### 工作流：使用新数据更新标定

```bash
# 1. 比较基准与新数据，查看变更
dcm_utils diff base.DCM new_data.DCM -o changes.json

# 2. 使用新值更新基准文件
dcm_utils update base.DCM new_data.DCM -o updated.DCM

# 3. 验证更新结果
dcm_utils diff base.DCM updated.DCM
```

### 工作流：合并多个标定文件

```bash
# 合并多个文件，以 base.DCM 为基础
dcm_utils merge base.DCM additions1.DCM additions2.DCM -o complete.DCM
```

### 工作流：提取特定标定集

```bash
# 提取所有匹配模式的参数
dcm_utils filter large_dataset.DCM --include "VAR.*" -o var_only.DCM

# 排除临时/测试参数
dcm_utils filter dataset.DCM --exclude ".*Temp.*" ".*Test.*" -o clean.DCM
```

## 测试

项目包含全面的测试：

- **单元测试**：块解析、值处理、属性解析
- **集成测试**：完整的文件读写周期
- **冒烟测试**：解析所有测试 DCM 文件，确保无 panic

测试数据位于 `./test-dcms/` 目录。

## 许可证

[MIT]

## 贡献

欢迎贡献！请确保：

1. 代码遵循 Rust 规范（`cargo fmt`、`cargo clippy`）
2. 测试通过（`cargo test`）
3. 新功能包含相应测试
4. 及时更新文档

## 致谢

- ASAM e.V. 的 DAMOS 标准
- Rust 社区提供的优秀工具和库
