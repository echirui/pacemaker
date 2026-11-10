fn main() {
    println!("cargo:rerun-if-changed=../ui");
    println!("cargo:rerun-if-changed=../ui/index.html");
    println!("cargo:rerun-if-changed=../ui/app.js");
    println!("cargo:rerun-if-changed=../ui/style.css");
    tauri_build::build()
}
