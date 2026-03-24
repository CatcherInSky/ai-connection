# 编译与开发命令

## 开发运行
```bash
cd src-tauri
cargo tauri dev
```

## 生产打包
```bash
cd src-tauri
cargo tauri build
```

- **产物位置**：`src-tauri/target/release/bundle/`
- **Linux**：当前默认产出 deb、rpm
- **Windows**：需在 Windows 上构建，产出 exe（及可选的 msi）
- **仅二进制、不打包**：`cargo tauri build --no-bundle`（产物在 `target/release/ai-connection-monitor`）
