fn main() {
    println!("cargo:rerun-if-changed=shaders/main.glsl");
    println!("cargo:rerun-if-changed=shaders/shader.glsl");
    println!("cargo:rerun-if-changed=shaders/geometry.glsl");
    println!("cargo:rerun-if-changed=shaders/rng.glsl");
}
