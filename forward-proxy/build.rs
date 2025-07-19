fn main() {
    #[cfg(debug_assertions)]
    {
        use std::fs;
        if !std::path::Path::new(".env").exists() {
            // if we're in dev mode make sure the .env.dev file is copied to a new .env file if none exists
            fs::copy(".env.dev", ".env").expect("Failed to copy .env.dev to .env");
        }
    }

    // let dst = cmake::build("libsrp6");
    //
    // println!("cargo:rustc-link-search=native={}", dst.display());
    // println!("cargo:rustc-link-search=libsrp6");
    // println!("cargo:rustc-link-lib=dylib=srp6");
    // println!("cargo:rustc-link-lib=dylib=ssl");
    // println!("cargo:rustc-link-lib=dylib=crypto");
    // println!("cargo:rustc-link-lib=dylib=stdc++");
}
