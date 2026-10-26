use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=resources/windows/icon.rc");
    println!("cargo:rerun-if-changed=../../assets/vibe-assembly-icon.ico");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=WindowsSdkDir");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("crate directory"));
    let resource = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"))
        .join("vibe-assembly-icon.res");
    let compiler = resource_compiler().expect(
        "Windows resource compiler not found. Install the Windows SDK or set RC to rc.exe's path.",
    );
    let status = Command::new(compiler)
        .current_dir(manifest)
        .args(["/nologo", "/fo"])
        .arg(&resource)
        .arg("resources/windows/icon.rc")
        .status()
        .expect("run Windows resource compiler");
    assert!(status.success(), "Windows app icon compilation failed");
    println!(
        "cargo:rustc-link-arg-bin=vibe-assembly-desktop={}",
        resource.display()
    );
}

fn resource_compiler() -> Option<PathBuf> {
    if let Some(compiler) = env::var_os("RC") {
        return Some(compiler.into());
    }
    if let Some(path) = env::var_os("PATH") {
        if let Some(compiler) = env::split_paths(&path)
            .map(|directory| directory.join("rc.exe"))
            .find(|compiler| compiler.is_file())
        {
            return Some(compiler);
        }
    }

    let mut roots = Vec::new();
    if let Some(sdk) = env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(sdk).join("bin"));
    }
    if let Some(programs) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(programs).join("Windows Kits/10/bin"));
    }
    let host = env::var("HOST").unwrap_or_default();
    let architecture = if host.starts_with("aarch64") {
        "arm64"
    } else if host.starts_with("i686") {
        "x86"
    } else {
        "x64"
    };
    for root in roots {
        let mut versions: Vec<_> = fs::read_dir(root)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        versions.sort();
        for version in versions.into_iter().rev() {
            let compiler = version.join(architecture).join("rc.exe");
            if compiler.is_file() {
                return Some(compiler);
            }
        }
    }
    None
}
