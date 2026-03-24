# AI Connection Monitor

跨平台桌面小工具，周期性检测多个 AI 服务（Claude、Cursor、Gemini 等）的连通性与延迟，并展示最近 48 小时内的历史图表。

## 功能

- **连通性检测**：定时探测各服务是否可达
- **延迟监控**：记录 HTTP 请求 RTT（毫秒）
- **历史图表**：折线图展示多服务延迟趋势
  - Ctrl + 滚轮缩放
  - 拖动 / Shift + 滚轮左右滑动
  - 应用退出后再打开，图表在时间空窗处自动断线
- **系统托盘**：关闭窗口后常驻托盘，后台继续探测

## 运行环境

| 平台 | 要求 |
|------|------|
| Windows | Windows 10/11，[WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)（Win11 已内置） |
| Linux | webkit2gtk、libappindicator 等，见下方依赖 |
| macOS | Xcode Command Line Tools |

### Linux 依赖（Ubuntu/Debian）

```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
# 若打包失败，可补充：
sudo apt install libayatana-appindicator3-dev
```

## 快速开始

### 开发运行

```bash
cd src-tauri
cargo tauri dev
```

### 生产构建

```bash
cd src-tauri
cargo tauri build
```

- **产物**：`src-tauri/target/release/bundle/`
- **Linux**：deb、rpm
- **Windows**：需在 Windows 上构建，产出 exe

> **WSL 路径构建**：若在 Windows 下对 WSL 仓库路径执行 build 报错，可先设置  
> `$env:CARGO_INCREMENTAL=0` 或 `$env:CARGO_TARGET_DIR="C:\temp\target"` 再构建。

## 技术栈

- [Tauri](https://tauri.app/) + Rust
- 前端：原生 HTML/JS
- 数据：SQLite 持久化

## 文档

- [打包与分发](docs/packaging.md)
- [命令速查](docs/commands.md)

## License

MIT
