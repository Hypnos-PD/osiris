fn main() {
    // Compile gframe LZMA sources so we can use LzmaUncompress via FFI for full compatibility
    let src_dir_rel = "external/ygopro/gframe/lzma";
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir).parent().unwrap().to_path_buf();
    let src_dir = workspace_root.join(src_dir_rel);
    let mut build = cc::Build::new();
    build.include(src_dir.clone());
    let files = [
        "Alloc.c",
        "LzFind.c",
        "LzmaDec.c",
        "LzmaEnc.c",
        "LzmaLib.c",
    ];
    for f in &files {
        let fpath = src_dir.join(f);
        println!("cargo:warning=Adding C source: {:?}", fpath);
        build.file(fpath);
    }
    build.compile("lzma_gframe");
}
