//! Semantic fingerprinting: hash only public API signatures using `syn`.
//!
//! When computing whether a crate's downstream dependents need to rebuild,
//! we only care about changes to the public API surface — not comments,
//! formatting, or private function bodies. This module extracts pub items
//! from Rust source files and produces a stable hash of just those signatures.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use xxhash_rust::xxh3::xxh3_128;

/// Extract a stable representation of the public API from a Rust source file.
/// Returns None if parsing fails (caller should fall back to content hash).
pub fn extract_public_api(source: &str) -> Option<String> {
    let file = syn::parse_file(source).ok()?;
    let mut api = String::new();

    for item in &file.items {
        extract_item(&mut api, item);
    }

    Some(api)
}

/// Hash only the public API surface of a Rust source file.
/// Falls back to hashing the full content if parsing fails.
pub fn semantic_hash_file(path: &Path) -> u128 {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    if let Some(api) = extract_public_api(&content) {
        if api.is_empty() {
            // No public items — hash full content as it might be a binary/test
            xxh3_128(content.as_bytes())
        } else {
            xxh3_128(api.as_bytes())
        }
    } else {
        // Parse failed — fall back to content hash
        xxh3_128(content.as_bytes())
    }
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn extract_item(out: &mut String, item: &syn::Item) {
    match item {
        syn::Item::Fn(f) if is_pub(&f.vis) => {
            extract_fn_sig(out, &f.sig);
            let _ = writeln!(out);
        }
        syn::Item::Struct(s) if is_pub(&s.vis) => {
            let _ = write!(out, "pub struct {}", s.ident);
            extract_generics(out, &s.generics);
            extract_struct_fields(out, &s.fields);
            let _ = writeln!(out);
        }
        syn::Item::Enum(e) if is_pub(&e.vis) => {
            let _ = write!(out, "pub enum {}", e.ident);
            extract_generics(out, &e.generics);
            let _ = write!(out, " {{");
            for variant in &e.variants {
                let _ = write!(out, " {}", variant.ident);
                extract_variant_fields(out, &variant.fields);
                let _ = write!(out, ",");
            }
            let _ = writeln!(out, " }}");
        }
        syn::Item::Trait(t) if is_pub(&t.vis) => {
            let _ = write!(out, "pub trait {}", t.ident);
            extract_generics(out, &t.generics);
            let _ = write!(out, " {{");
            for item in &t.items {
                if let syn::TraitItem::Fn(method) = item {
                    let _ = write!(out, " ");
                    extract_fn_sig(out, &method.sig);
                    let _ = write!(out, ";");
                }
                if let syn::TraitItem::Type(assoc) = item {
                    let _ = write!(out, " type {};", assoc.ident);
                }
            }
            let _ = writeln!(out, " }}");
        }
        syn::Item::Impl(imp) => {
            // Extract pub methods from impl blocks
            for impl_item in &imp.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    if is_pub(&method.vis) {
                        let _ = write!(out, "impl ");
                        extract_type(out, &imp.self_ty);
                        let _ = write!(out, " ");
                        extract_fn_sig(out, &method.sig);
                        let _ = writeln!(out);
                    }
                }
            }
        }
        syn::Item::Type(t) if is_pub(&t.vis) => {
            let _ = write!(out, "pub type {}", t.ident);
            extract_generics(out, &t.generics);
            let _ = write!(out, " = ");
            extract_type(out, &t.ty);
            let _ = writeln!(out, ";");
        }
        syn::Item::Const(c) if is_pub(&c.vis) => {
            let _ = write!(out, "pub const {}: ", c.ident);
            extract_type(out, &c.ty);
            let _ = writeln!(out, ";");
        }
        syn::Item::Static(s) if is_pub(&s.vis) => {
            let _ = write!(out, "pub static {}: ", s.ident);
            extract_type(out, &s.ty);
            let _ = writeln!(out, ";");
        }
        syn::Item::Mod(m) if is_pub(&m.vis) => {
            if let Some((_, items)) = &m.content {
                let _ = write!(out, "pub mod {} {{", m.ident);
                for item in items {
                    extract_item(out, item);
                }
                let _ = writeln!(out, "}}");
            } else {
                let _ = writeln!(out, "pub mod {};", m.ident);
            }
        }
        syn::Item::Use(u) if is_pub(&u.vis) => {
            let _ = writeln!(out, "pub use ...;");
        }
        _ => {}
    }
}

fn extract_fn_sig(out: &mut String, sig: &syn::Signature) {
    if sig.asyncness.is_some() {
        let _ = write!(out, "async ");
    }
    if sig.unsafety.is_some() {
        let _ = write!(out, "unsafe ");
    }
    let _ = write!(out, "fn {}", sig.ident);
    extract_generics(out, &sig.generics);
    let _ = write!(out, "(");
    for (i, arg) in sig.inputs.iter().enumerate() {
        if i > 0 {
            let _ = write!(out, ", ");
        }
        match arg {
            syn::FnArg::Receiver(r) => {
                if r.reference.is_some() {
                    let _ = write!(out, "&");
                    if r.mutability.is_some() {
                        let _ = write!(out, "mut ");
                    }
                }
                let _ = write!(out, "self");
            }
            syn::FnArg::Typed(pat_type) => {
                let _ = write!(out, "_: ");
                extract_type(out, &pat_type.ty);
            }
        }
    }
    let _ = write!(out, ")");
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        let _ = write!(out, " -> ");
        extract_type(out, ty);
    }
}

fn extract_generics(out: &mut String, generics: &syn::Generics) {
    if generics.params.is_empty() {
        return;
    }
    let _ = write!(out, "<");
    for (i, param) in generics.params.iter().enumerate() {
        if i > 0 {
            let _ = write!(out, ", ");
        }
        match param {
            syn::GenericParam::Type(t) => {
                let _ = write!(out, "{}", t.ident);
                if !t.bounds.is_empty() {
                    let _ = write!(out, ": ...");
                }
            }
            syn::GenericParam::Lifetime(l) => {
                let _ = write!(out, "'{}", l.lifetime.ident);
            }
            syn::GenericParam::Const(c) => {
                let _ = write!(out, "const {}: ", c.ident);
                extract_type(out, &c.ty);
            }
        }
    }
    let _ = write!(out, ">");
}

fn extract_type(out: &mut String, ty: &syn::Type) {
    // Use token stream for a stable text representation of the type
    let _ = write!(out, "{}", quote::quote!(#ty));
}

fn extract_struct_fields(out: &mut String, fields: &syn::Fields) {
    match fields {
        syn::Fields::Named(named) => {
            let _ = write!(out, " {{");
            for field in &named.named {
                if is_pub(&field.vis) {
                    if let Some(ident) = &field.ident {
                        let _ = write!(out, " pub {}: ", ident);
                        extract_type(out, &field.ty);
                        let _ = write!(out, ",");
                    }
                }
            }
            let _ = write!(out, " }}");
        }
        syn::Fields::Unnamed(unnamed) => {
            let _ = write!(out, "(");
            for (i, field) in unnamed.unnamed.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                if is_pub(&field.vis) {
                    let _ = write!(out, "pub ");
                }
                extract_type(out, &field.ty);
            }
            let _ = write!(out, ")");
        }
        syn::Fields::Unit => {}
    }
}

fn extract_variant_fields(out: &mut String, fields: &syn::Fields) {
    match fields {
        syn::Fields::Named(named) => {
            let _ = write!(out, " {{");
            for field in &named.named {
                if let Some(ident) = &field.ident {
                    let _ = write!(out, " {}: ", ident);
                    extract_type(out, &field.ty);
                    let _ = write!(out, ",");
                }
            }
            let _ = write!(out, " }}");
        }
        syn::Fields::Unnamed(unnamed) => {
            let _ = write!(out, "(");
            for (i, field) in unnamed.unnamed.iter().enumerate() {
                if i > 0 {
                    let _ = write!(out, ", ");
                }
                extract_type(out, &field.ty);
            }
            let _ = write!(out, ")");
        }
        syn::Fields::Unit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pub_fn() {
        let src = r#"
            pub fn greet(name: &str) -> String { format!("hi {name}") }
            fn private_helper() {}
        "#;
        let api = extract_public_api(src).unwrap();
        assert!(api.contains("fn greet"));
        assert!(!api.contains("private_helper"));
    }

    #[test]
    fn extracts_pub_struct() {
        let src = r#"
            pub struct Foo {
                pub x: i32,
                secret: String,
            }
        "#;
        let api = extract_public_api(src).unwrap();
        assert!(api.contains("pub struct Foo"));
        assert!(api.contains("pub x"));
        assert!(!api.contains("secret"));
    }

    #[test]
    fn extracts_pub_enum() {
        let src = r#"
            pub enum Color { Red, Green, Blue }
            enum Private { A, B }
        "#;
        let api = extract_public_api(src).unwrap();
        assert!(api.contains("pub enum Color"));
        assert!(api.contains("Red"));
        assert!(!api.contains("Private"));
    }

    #[test]
    fn ignores_fn_body_changes() {
        let src1 = r#"pub fn add(a: i32, b: i32) -> i32 { a + b }"#;
        let src2 = r#"pub fn add(a: i32, b: i32) -> i32 { let result = a + b; result }"#;
        let api1 = extract_public_api(src1).unwrap();
        let api2 = extract_public_api(src2).unwrap();
        assert_eq!(api1, api2);
    }

    #[test]
    fn detects_signature_change() {
        let src1 = r#"pub fn add(a: i32, b: i32) -> i32 { a + b }"#;
        let src2 = r#"pub fn add(a: i64, b: i64) -> i64 { a + b }"#;
        let api1 = extract_public_api(src1).unwrap();
        let api2 = extract_public_api(src2).unwrap();
        assert_ne!(api1, api2);
    }

    #[test]
    fn ignores_comments() {
        let src1 = r#"pub fn foo() -> bool { true }"#;
        let src2 = r#"
            /// This is a doc comment
            // Regular comment
            pub fn foo() -> bool { true }
        "#;
        let api1 = extract_public_api(src1).unwrap();
        let api2 = extract_public_api(src2).unwrap();
        assert_eq!(api1, api2);
    }

    #[test]
    fn extracts_pub_trait() {
        let src = r#"
            pub trait Greeter {
                fn greet(&self) -> String;
                type Output;
            }
        "#;
        let api = extract_public_api(src).unwrap();
        assert!(api.contains("pub trait Greeter"));
        assert!(api.contains("fn greet"));
        assert!(api.contains("type Output"));
    }
}
