fn main() {
    // Lets `todo update` pick the release archive built for this target.
    println!(
        "cargo:rustc-env=TUILIST_TARGET={}",
        std::env::var("TARGET").expect("Cargo sets TARGET for build scripts")
    );
}
