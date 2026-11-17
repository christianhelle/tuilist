fn main() {
    // Lets `todo update` pick the release archive built for this target.
    println!(
        "cargo:rustc-env=TUILIST_TARGET={}",
        std::env::var("TARGET").expect("Cargo sets TARGET for build scripts")
    );

    // Embed the application icon in the Windows executable. macOS and Linux
    // have no mechanism for embedding an icon in a plain binary.
    println!("cargo:rerun-if-changed=images/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("images/icon.ico")
            .compile()
            .expect("failed to embed the Windows icon");
    }
}
