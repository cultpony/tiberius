use change_detection::ChangeDetection;
use std::convert::TryFrom;
use std::path::Path;
use std::process::Command;

fn main() {
    if std::env::var("TIBERIUS_PREBUILT_ASSETS") == Ok("YES".to_string()) {
        return;
    }
    let debug = std::env::var("PROFILE").expect("need rust compile profile") != "release";
    if !debug {
        let assetdir = "../res/assets-build";
        let assetdir =
            std::path::PathBuf::try_from(assetdir).expect("release asset path not readable");
        let assetdir = assetdir
            .canonicalize()
            .expect("release could not canonicalize asset path");
        println!(
            "cargo:warning=Deleting old asset build dir first: {}",
            assetdir.display()
        );
        std::fs::remove_dir_all(assetdir).unwrap();
    }
    let builddir = "../res/assets";
    let builddir = std::path::PathBuf::try_from(builddir).expect("asset path not readable");
    let builddir = builddir
        .canonicalize()
        .expect("could not canonicalize asset path");
    ChangeDetection::path_exclude("../res/assets", |x: &Path| {
        x.starts_with("../res/assets/node_modules")
    })
    .generate();
    println!("cargo:warning=Building in {}", builddir.display());
    {
        println!("cargo:warning: yarn install");
        let out = Command::new("yarn")
            .arg("install")
            .current_dir(builddir.clone())
            .output()
            .expect("failed to run build command");
        if !out.status.success() {
            panic!(
                " --- Asset Build Failed: --- \nStdout:\n{}\n---\nStderr:\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    {
        println!("cargo:warning: yarn build");
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
        .expect("failed to run build command");
        if !out.status.success() {
            panic!(
                " --- Asset Build Failed: --- \nStdout:\n{}\n---\nStderr:\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    println!("cargo:warning=Building Tiberius Core Assets complete");
}
