//! Benchmarks for performance-sensitive operations.

use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

fn bench_xxhash_fingerprint() {
    let data = "a".repeat(10_000);
    let start = Instant::now();
    for _ in 0..1000 {
        let hash = xxhash_rust::xxh3::xxh3_128(black_box(data.as_bytes()));
        black_box(hash);
    }
    let elapsed = start.elapsed();
    println!(
        "xxh3_128 (10KB x 1000): {:.2}ms ({:.0} MB/s)",
        elapsed.as_secs_f64() * 1000.0,
        (10_000.0 * 1000.0) / elapsed.as_secs_f64() / 1_000_000.0
    );
}

fn bench_semantic_hash() {
    let source = r#"
pub fn hello(name: &str) -> String {
    format!("hello {name}")
}

pub struct Config {
    pub name: String,
    pub value: u64,
}

pub trait Service {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>>;
}
"#;

    let start = Instant::now();
    for _ in 0..100 {
        let api = rx::semantic_hash::extract_public_api(black_box(source));
        black_box(api);
    }
    let elapsed = start.elapsed();
    println!(
        "semantic_hash parse (100 iterations): {:.2}ms ({:.0} us/iter)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1_000_000.0 / 100.0
    );
}

fn bench_cache_fingerprint() {
    // Create a temporary project structure
    let tmp = std::env::temp_dir().join("rx-bench-fp");
    std::fs::create_dir_all(tmp.join("src")).ok();

    // Write a Cargo.toml
    std::fs::write(
        tmp.join("Cargo.toml"),
        r#"[package]
name = "bench-project"
version = "0.1.0"
edition = "2024"
"#,
    )
    .ok();

    // Write some source files
    for i in 0..10 {
        let mut f = std::fs::File::create(tmp.join("src").join(format!("file_{i}.rs"))).unwrap();
        writeln!(f, "pub fn func_{i}() -> u64 {{ {i} }}").unwrap();
    }
    std::fs::write(tmp.join("src/main.rs"), "fn main() {}").ok();

    let start = Instant::now();
    for _ in 0..50 {
        let fp = rx::cache::compute_build_fingerprint(black_box(&tmp), "debug", None);
        black_box(fp.ok());
    }
    let elapsed = start.elapsed();
    println!(
        "cache fingerprint (10 files x 50): {:.2}ms ({:.0} us/iter)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1_000_000.0 / 50.0
    );

    // Cleanup
    std::fs::remove_dir_all(&tmp).ok();
}

fn main() {
    println!("rx benchmarks");
    println!("{}", "─".repeat(60));
    bench_xxhash_fingerprint();
    bench_semantic_hash();
    bench_cache_fingerprint();
}
