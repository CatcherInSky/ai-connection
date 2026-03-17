# AI Connection Monitor - 打包与分发指南

## 1. 打包命令

### 开发运行
```bash
cd src-tauri
cargo tauri dev
```
或（若项目根目录有 package.json 配置了 tauri）：
```bash
npm run tauri dev
```

### 生产打包
```bash
cd src-tauri
cargo tauri build
```

打包产物位于 `src-tauri/target/release/bundle/` 下。

---

## 2. 各平台打包方式

### Windows
- **环境要求**：Windows 10/11、Visual Studio Build Tools（或完整 VS）
- **WebView2 必需**：Tauri 依赖 WebView2。Win11 已内置；Win10 需安装 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)。若出现闪退，先安装 WebView2 再试。
- **调试闪退**：在 CMD 中 `cd` 到 exe 所在目录，执行 exe，可看到崩溃时的错误输出。
- **命令**：在 Windows 上执行 `cargo tauri build`
- **输出格式**：
  - `.msi` 安装包（需 WiX Toolset）
  - `.exe` 可执行文件
- **注意**：跨平台时 Windows 包需在 Windows 上构建

### macOS
- **环境要求**：macOS、Xcode Command Line Tools
- **命令**：在 macOS 上执行 `cargo tauri build`
- **输出格式**：
  - `.app` 应用包
  - `.dmg` 磁盘镜像（可选）
- **注意**：macOS 包需在 macOS 上构建；上架或分发通常需代码签名与公证

### Linux
- **环境要求**：常见发行版、webkit2gtk、libappindicator 等（Tauri 会提示缺失依赖）
- **命令**：在 Linux 上执行 `cargo tauri build`
- **输出格式**：
  - `.AppImage`
  - `.deb`（Debian/Ubuntu）
  - `.rpm`（Fedora/RHEL）
- **Ubuntu 依赖示例**：
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
  ```
- **若打包失败**（`Can't detect any appindicator library`）：
  - 安装：`sudo apt install libayatana-appindicator3-dev`
  - 或仅构建二进制、不打包：`cargo tauri build --no-bundle`（产物在 `target/release/ai-connection-monitor`）
- **当前默认**：仅生成 deb 和 rpm，跳过 AppImage（避免 linuxdeploy 依赖问题）。如需 AppImage，可执行 `cargo tauri build --bundles appimage`

---

## 3. 跨平台构建（CI/CD）

无法在单机上直接构建所有平台包，需使用对应系统或 CI：

- **GitHub Actions**：为 Windows/macOS/Linux 各建一个 job 或 matrix
- **自托管**：在各自平台机器上执行 `cargo tauri build`
- **参考**：[tauri-action](https://github.com/tauri-apps/tauri-action) 可自动处理多平台构建

---

## 4. 图标配置

在 `tauri.conf.json` 的 `bundle.icon` 中配置图标路径。将图标文件置于 `src-tauri/icons/` 目录：

- `icon.png`（至少 32×32，推荐 512×512）
- 或 `icon.ico`（Windows）
- 或 `icon.icns`（macOS）

当前 `build.rs` 会在缺失时生成最小 1×1 占位图标；正式分发前请替换为正式图标。

---

## 5. 版本号

版本号来自 `tauri.conf.json` 的 `version` 字段，未配置时回退到 `Cargo.toml` 中 `package.version`。
