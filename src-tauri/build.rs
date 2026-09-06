// Le decaleur de formants, compile ici plutot que pris sur crates.io.
//
// Le crate `signalsmith-stretch` fait la meme chose, mais genere ses liaisons avec bindgen, qui
// reclame libclang -- 2,5 Go de LLVM pour transcrire quinze signatures C triviales. Elles sont
// ecrites a la main dans src\stretch.rs ; il ne reste ici que le C++ a compiler, ce que `cc`
// fait avec le MSVC deja installe.
fn main() {
    println!("cargo:rerun-if-changed=vendor/signalsmith/wrapper.cpp");
    println!("cargo:rerun-if-changed=vendor/signalsmith/wrapper.h");

    cc::Build::new()
        .file("vendor/signalsmith/wrapper.cpp")
        .include("vendor/signalsmith")
        .cpp(true)
        .std("c++14")
        .compile("signalsmith_stretch");

    tauri_build::build();
}
