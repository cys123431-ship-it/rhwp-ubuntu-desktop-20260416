use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");

    let attributes = tauri_build::Attributes::new()
        .windows_attributes(configure_windows_resources());

    tauri_build::try_build(attributes).expect("failed to run tauri build script");
}

fn configure_windows_resources() -> tauri_build::WindowsAttributes {
    let source_icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("icons/icon.ico");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("tauri-build-assets");

    fs::create_dir_all(&out_dir).expect("failed to create tauri build asset directory");

    let copied_icon = out_dir.join("icon.ico");
    fs::copy(&source_icon, &copied_icon).expect("failed to stage Windows icon for tauri build");

    tauri_build::WindowsAttributes::new().window_icon_path(copied_icon)
}
