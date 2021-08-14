use change_detection::ChangeDetection;
use std::convert::TryFrom;
use std::path::Path;
use std::process::Command;

fn main() {
    let debug = std::env::var("PROFILE").unwrap() != "release";
    if !debug {
        let assetdir = "../res/assets-build";
        let assetdir = std::path::PathBuf::try_from(assetdir).unwrap();
        let assetdir = assetdir.canonicalize().unwrap();
        println!(
            "cargo:warning=Deleting old asset build dir first: {}",
            assetdir.display()
        );
        std::fs::remove_dir_all(assetdir).unwrap();
    }
    let builddir = "../res/assets";
    let builddir = std::path::PathBuf::try_from(builddir).unwrap();
    let builddir = builddir.canonicalize().unwrap();
    ChangeDetection::path_exclude("../res/assets", |x: &Path| {
        x.starts_with("../res/assets/node_modules")
    })
    .generate();
    println!("cargo:warning=Building in {}", builddir.display());
    let mut cmd = Command::new("yarn");
    let out = {
        if debug {
            cmd.arg("devbuild").env("NODE_ENV", "development")
        } else {
            println!("cargo:warning=Production Asset Build");
            cmd.arg("deploy").env("NODE_ENV", "production")
        }
    }
    .current_dir(builddir)
    .output()
    .unwrap();
    if !out.status.success() {
        panic!(
            " --- Asset Build Failed: --- \nStdout:\n{}\n---\nStderr:\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    println!("cargo:warning=Building Tiberius Core Assets complete");
}
