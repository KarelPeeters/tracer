fn main() {
    println!("cargo:rerun-if-changed=src/main.glsl");
    println!("cargo:rerun-if-changed=src/shader.glsl");
    println!("cargo:rerun-if-changed=src/geometry.glsl");
}