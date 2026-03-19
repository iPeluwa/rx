use std::fs;
use tempfile::TempDir;

#[test]
fn fingerprint_deterministic() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    let fp1 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    let fp2 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    assert_eq!(fp1, fp2);
}

#[test]
fn fingerprint_changes_with_profile() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    let debug = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    let release = rx::cache::compute_build_fingerprint(root, "release", None).unwrap();
    assert_ne!(debug, release);
}

#[test]
fn fingerprint_changes_with_rustflags() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    let no_flags = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    let with_flags =
        rx::cache::compute_build_fingerprint(root, "debug", Some("-Clinker=mold")).unwrap();
    assert_ne!(no_flags, with_flags);
}

#[test]
fn fingerprint_changes_with_source() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    let fp1 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();

    fs::write(root.join("src/main.rs"), "fn main() { println!(\"hi\"); }").unwrap();

    let fp2 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    assert_ne!(fp1, fp2);
}

#[test]
fn fingerprint_changes_with_cargo_toml() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

    let fp1 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();

    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    let fp2 = rx::cache::compute_build_fingerprint(root, "debug", None).unwrap();
    assert_ne!(fp1, fp2);
}

#[test]
fn store_and_restore_build() {
    let dir = TempDir::new().unwrap();
    let src_dir = dir.path().join("src_artifacts");
    let target_dir = dir.path().join("restored");
    fs::create_dir_all(&src_dir).unwrap();

    // Create fake artifacts
    fs::write(src_dir.join("mybin"), b"binary content here").unwrap();
    fs::write(src_dir.join("libfoo.rlib"), b"rlib content").unwrap();

    let artifacts = vec![
        ("mybin".to_string(), src_dir.join("mybin")),
        ("libfoo.rlib".to_string(), src_dir.join("libfoo.rlib")),
    ];

    let fingerprint = "aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233";
    let cached = rx::cache::store_build(fingerprint, &artifacts).unwrap();
    assert!(cached.exists());

    // Restore to a new directory
    let count = rx::cache::restore_build(&cached, &target_dir).unwrap();
    assert_eq!(count, 2);
    assert_eq!(
        fs::read_to_string(target_dir.join("mybin")).unwrap(),
        "binary content here"
    );
    assert_eq!(
        fs::read_to_string(target_dir.join("libfoo.rlib")).unwrap(),
        "rlib content"
    );
}

#[test]
fn lookup_build_miss() {
    let result =
        rx::cache::lookup_build("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
    assert!(result.is_none());
}

#[test]
fn lookup_build_hit_after_store() {
    let dir = TempDir::new().unwrap();
    let src_dir = dir.path().join("art");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("bin"), b"test").unwrap();

    let fingerprint = "ff11223344556677ff11223344556677ff11223344556677ff11223344556677";
    let artifacts = vec![("bin".to_string(), src_dir.join("bin"))];

    rx::cache::store_build(fingerprint, &artifacts).unwrap();
    let result = rx::cache::lookup_build(fingerprint).unwrap();
    assert!(result.is_some());
}
