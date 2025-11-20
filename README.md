# Osiris

**Osiris** 是 YGOPro 核心游戏逻辑 (`ocgcore`) 的纯 Rust 重写项目。
它的目标是提供一个高性能、内存安全且逻辑与原版 **像素级精确 (Pixel-Perfect)** 的卡牌游戏引擎。

## 🦁 核心特性

*   **纯 Rust 实现**：无 C++ 依赖，利用 Rust 的所有权机制管理复杂的卡牌关系。
*   **Lua 5.3 集成**：内置 Lua 虚拟机（基于 `mlua`），**完全兼容** 官方 YGOPro 脚本 (`constant.lua`,`utility.lua`, `procedure.lua` 及卡片脚本)。
*   **现代架构**：
    *   **Arena 内存模型**：使用 Entity-Component 风格的 ID 索引，避免指针地狱。
    *   **Unit-Based Processor**：基于队列的处理器架构，完美支持嵌套逻辑（如连锁处理）。
    *   **并发友好**：核心状态包裹在 `Arc<Mutex<DuelData>>` 中，支持多线程环境。
*   **生态支持**：
    *   支持加载 `.yrp` 回放文件（含 LZMA 解压与 STOC 协议解析）。
    *   支持读取 `cards.cdb` (SQLite) 数据库。

## 🛠️ 快速开始

### 运行测试
Osiris 包含大量单元测试来验证逻辑正确性：

```bash
cargo test -p osiris
```

### 核心组件

*   **`Duel`**: 外部接口壳，持有 Lua 虚拟机。
*   **`DuelData`**: 实际的游戏状态（卡片、场地、连锁），可被 Lua 回调访问。
*   **`Processor`**: 驱动游戏阶段流转的状态机。
*   **`Scripting`**: 负责加载和执行 Lua 脚本。

## 📜 项目状态

目前处于开发中。

---
*Osiris 是 Project Osiris 的核心部分。*