// CascLib, compilee ici plutot que prise sur crates.io -- il n'y en a pas d'equivalent Rust qui
// traite MNDX, le format racine de StarCraft II.
//
// Elle porte `sources-c.c` et `sources-cpp.cpp`, deux fichiers de compilation unifiee qui
// incluent tout le reste : deux appels a `cc` suffisent, et le CMakeLists du depot n'a pas a
// etre utilise. C'est le meme parti pris que `src-tauri\build.rs` pour signalsmith.

fn main() {
    let casclib = "vendor/CascLib";
    println!("cargo:rerun-if-changed=src/casc_shim.cpp");
    println!("cargo:rerun-if-changed={casclib}/sources-cpp.cpp");

    if !std::path::Path::new(casclib).join("sources-cpp.cpp").exists() {
        panic!(
            "CascLib absente. Depuis outils\\fabriquer :\n  \
             git clone --depth 1 https://github.com/ladislav-zezula/CascLib.git vendor\\CascLib"
        );
    }

    // La bibliotheque de codebooks d'aoTuV, que `wwise.rs` embarque pour rendre au doublage de
    // Cyberpunk les tables que Wwise lui a retirees. Meme parti pris que CascLib : un depot
    // tiers se clone, il ne se versionne pas ici.
    let codebooks = "vendor/ww2ogg/packed_codebooks_aoTuV_603.bin";
    println!("cargo:rerun-if-changed={codebooks}");
    if !std::path::Path::new(codebooks).exists() {
        panic!(
            "packed_codebooks_aoTuV_603.bin absent. Depuis outils\\fabriquer :\n  \
             git clone --depth 1 https://github.com/hcs64/ww2ogg.git vendor\\ww2ogg"
        );
    }

    // Le C et le C++ de CascLib ne se compilent pas dans le meme appel : `cc` choisit le
    // compilateur pour toute la fournee.
    cc::Build::new()
        .file(format!("{casclib}/sources-c.c"))
        .include(format!("{casclib}/src"))
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .define("CASCLIB_NO_AUTO_LINK_LIBRARY", None)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .warnings(false)
        .compile("casclib_c");

    cc::Build::new()
        .file(format!("{casclib}/sources-cpp.cpp"))
        .file("src/casc_shim.cpp")
        .include(format!("{casclib}/src"))
        .cpp(true)
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .define("CASCLIB_NO_AUTO_LINK_LIBRARY", None)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .warnings(false)
        .compile("casclib_cpp");

    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=ws2_32");
}
