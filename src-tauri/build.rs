fn main() {
    // 生成 Tauri 所需的正方形图标（AppImage 至少需 32x32）
    let icons_dir = std::path::Path::new("icons");
    let _ = std::fs::create_dir_all(icons_dir);

    let (r, g, b) = (0x22u8, 0xc5u8, 0x5eu8);
    for (size, name) in [(32, "32x32.png"), (128, "128x128.png"), (128, "icon.png")] {
        let path = icons_dir.join(name);
        let mut img = image::RgbaImage::new(size, size);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([r, g, b, 255]);
        }
        let _ = image::DynamicImage::ImageRgba8(img).save(&path);
    }

    // Windows 需要 icon.ico（tauri-build 的 Windows 资源文件）
    let ico_path = icons_dir.join("icon.ico");
    let ico_img = image::RgbaImage::from_fn(32, 32, |_, _| image::Rgba([r, g, b, 255]));
    let _ = image::DynamicImage::ImageRgba8(ico_img).save(&ico_path);

    // 兼容：若 icon.png 仍不存在，写最小占位（1x1）
    let icon_path = icons_dir.join("icon.png");
    if !icon_path.exists() {
        let minimal_png: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        let _ = std::fs::write(&icon_path, minimal_png);
    }

    tauri_build::build();
}

