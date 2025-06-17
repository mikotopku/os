static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn main() {
    println!("cargo::rustc-check-cfg=cfg(LOG_ERROR)");
    println!("cargo::rustc-check-cfg=cfg(LOG_WARN)");
    println!("cargo::rustc-check-cfg=cfg(LOG_INFO)");
    println!("cargo::rustc-check-cfg=cfg(LOG_DEBUG)");
    println!("cargo::rustc-check-cfg=cfg(LOG_TRACE)");
    println!("cargo:rerun-if-changed=../user/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
}