use std::env;
use std::process::Command;

fn main() {
    // TODO: add error checking
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();

    let git_hash: String = String::from_utf8(output.stdout).unwrap();
    let mut git_short_hash = git_hash.clone();
    git_short_hash.truncate(8);
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    match git_short_hash.len() {
        0 => println!(
            "cargo:rustc-env=GIT_SHORT_HASH={}",
            env::var("GIT_COMMIT_ID").unwrap_or_else(|_| String::from(""))
        ),
        _ => println!("cargo:rustc-env=GIT_SHORT_HASH={}", git_short_hash),
    };
}
