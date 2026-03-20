use anyhow::Result;
use owo_colors::OwoColorize;

/// Curated database of helpful hints for common Rust error codes.
/// These supplement rustc's own explanations with practical fixes.
struct HintEntry {
    code: &'static str,
    short: &'static str,
    detail: &'static str,
}

const HINT_DB: &[HintEntry] = &[
    HintEntry {
        code: "E0502",
        short: "Cannot borrow as mutable because it's also borrowed as immutable.",
        detail: "Common fixes:\n\
                 1. Move the immutable borrow into its own block/scope\n\
                 2. Clone the value to avoid shared references\n\
                 3. Use RefCell<T> for interior mutability\n\
                 4. Restructure to avoid overlapping borrows",
    },
    HintEntry {
        code: "E0499",
        short: "Cannot borrow as mutable more than once at a time.",
        detail: "Common fixes:\n\
                 1. Split into separate scopes so borrows don't overlap\n\
                 2. Use indices instead of references for collections\n\
                 3. Use Cell/RefCell for interior mutability",
    },
    HintEntry {
        code: "E0308",
        short: "Mismatched types.",
        detail: "Common fixes:\n\
                 1. Check return type annotation matches actual return value\n\
                 2. Use .into() or From::from() for type conversions\n\
                 3. Add/remove & or * for reference/dereference\n\
                 4. Use as for numeric casts (e.g., x as u64)",
    },
    HintEntry {
        code: "E0382",
        short: "Use of moved value.",
        detail: "Common fixes:\n\
                 1. Clone the value before the move: value.clone()\n\
                 2. Use a reference (&value) instead of moving\n\
                 3. Implement Copy if the type is small and simple\n\
                 4. Use Rc<T> or Arc<T> for shared ownership",
    },
    HintEntry {
        code: "E0277",
        short: "Trait not implemented.",
        detail: "Common fixes:\n\
                 1. Derive the trait: #[derive(Debug, Clone, ...)]\n\
                 2. Implement the trait manually with impl Trait for Type\n\
                 3. Check if you need a trait bound: fn foo<T: Trait>(x: T)\n\
                 4. Use a wrapper type that implements the trait",
    },
    HintEntry {
        code: "E0425",
        short: "Cannot find value or function in this scope.",
        detail: "Common fixes:\n\
                 1. Add a use statement to import the item\n\
                 2. Check for typos in the name\n\
                 3. Ensure the module is declared with mod in lib.rs/main.rs\n\
                 4. Check visibility — the item may need pub",
    },
    HintEntry {
        code: "E0433",
        short: "Failed to resolve path — module or type not found.",
        detail: "Common fixes:\n\
                 1. Add the crate to [dependencies] in Cargo.toml\n\
                 2. Check the use path matches the module structure\n\
                 3. Ensure the module file exists (e.g., src/foo.rs or src/foo/mod.rs)",
    },
    HintEntry {
        code: "E0599",
        short: "No method found for this type.",
        detail: "Common fixes:\n\
                 1. Import the trait that provides the method: use Trait;\n\
                 2. Check the receiver type — you may need &self vs self\n\
                 3. Check feature flags — some methods are behind cargo features\n\
                 4. The method may exist on a different type — check docs",
    },
    HintEntry {
        code: "E0061",
        short: "Wrong number of arguments.",
        detail: "Common fixes:\n\
                 1. Check the function signature for required parameters\n\
                 2. Some parameters may have been added in a newer version\n\
                 3. Check if you meant to call a different function/method",
    },
    HintEntry {
        code: "E0106",
        short: "Missing lifetime specifier.",
        detail: "Common fixes:\n\
                 1. Add a lifetime parameter: fn foo<'a>(x: &'a str) -> &'a str\n\
                 2. Use owned types instead (String vs &str, Vec<T> vs &[T])\n\
                 3. Use 'static if the reference lives for the whole program\n\
                 4. Let the compiler elide lifetimes — simplify the signature",
    },
    HintEntry {
        code: "E0597",
        short: "Value does not live long enough.",
        detail: "Common fixes:\n\
                 1. Move the value to a higher scope (declare it earlier)\n\
                 2. Use owned types instead of references\n\
                 3. Use Arc/Rc for shared ownership across scopes\n\
                 4. Restructure to avoid returning references to local data",
    },
    HintEntry {
        code: "E0507",
        short: "Cannot move out of borrowed content.",
        detail: "Common fixes:\n\
                 1. Clone the value: value.clone()\n\
                 2. Use .take() for Option<T> fields\n\
                 3. Use std::mem::replace() to swap with a default\n\
                 4. Match on a reference: match &value { ... }",
    },
    HintEntry {
        code: "E0283",
        short: "Type annotations needed — cannot infer type.",
        detail: "Common fixes:\n\
                 1. Add explicit type: let x: Type = ...\n\
                 2. Use turbofish: iter.collect::<Vec<_>>()\n\
                 3. Specify the type in the function call context",
    },
    HintEntry {
        code: "E0271",
        short: "Type mismatch in trait implementation.",
        detail: "Common fixes:\n\
                 1. Check the associated type in your impl matches the trait definition\n\
                 2. Ensure generic parameters align between trait and impl\n\
                 3. Check if you're implementing the right version of the trait",
    },
    HintEntry {
        code: "E0405",
        short: "Cannot find trait in this scope.",
        detail: "Common fixes:\n\
                 1. Add a use statement: use crate_name::TraitName;\n\
                 2. Check that the trait is pub in its defining module\n\
                 3. Ensure the dependency is in Cargo.toml with the right features",
    },
    HintEntry {
        code: "E0432",
        short: "Unresolved import.",
        detail: "Common fixes:\n\
                 1. Check the crate is in [dependencies] in Cargo.toml\n\
                 2. Verify the path: crate:: for local, or crate_name:: for deps\n\
                 3. Check visibility — the item may not be pub\n\
                 4. Run `cargo update` if you just added the dependency",
    },
    HintEntry {
        code: "E0603",
        short: "Item is private.",
        detail: "Common fixes:\n\
                 1. Make the item pub in its defining module\n\
                 2. Use pub(crate) for crate-internal visibility\n\
                 3. Access through a public re-export if available",
    },
    HintEntry {
        code: "E0658",
        short: "Feature not stabilized yet.",
        detail: "Common fixes:\n\
                 1. Use #![feature(name)] on nightly (not recommended for production)\n\
                 2. Find a stable alternative in the standard library\n\
                 3. Use a crate that provides similar functionality\n\
                 4. Check if a newer Rust version has stabilized it",
    },
    HintEntry {
        code: "E0015",
        short: "Cannot call non-const function in const context.",
        detail: "Common fixes:\n\
                 1. Use const fn if the function can be made const\n\
                 2. Move the call out of the const context\n\
                 3. Use lazy_static! or std::sync::LazyLock for lazy initialization",
    },
    HintEntry {
        code: "E0373",
        short: "Closure may outlive the current function, but it borrows a variable.",
        detail: "Common fixes:\n\
                 1. Use move || { ... } to capture by value\n\
                 2. Clone the value before the closure: let val = val.clone();\n\
                 3. Use Arc<T> for thread-safe shared ownership",
    },
    HintEntry {
        code: "E0412",
        short: "Cannot find type name in this scope.",
        detail: "Common fixes:\n\
                 1. Add a use statement to import the type\n\
                 2. Check for typos in the type name\n\
                 3. Ensure the crate is in [dependencies] in Cargo.toml\n\
                 4. Check that the type is pub in its defining module",
    },
    HintEntry {
        code: "E0463",
        short: "Can't find crate.",
        detail: "Common fixes:\n\
                 1. Add the crate to [dependencies] in Cargo.toml\n\
                 2. Run `cargo update` to refresh the index\n\
                 3. Check that the crate name is spelled correctly\n\
                 4. Ensure `extern crate` is not needed (edition 2018+)",
    },
    HintEntry {
        code: "E0609",
        short: "No field on this type.",
        detail: "Common fixes:\n\
                 1. Check the field name for typos\n\
                 2. Ensure the field is pub if accessing from another module\n\
                 3. Use a getter method if the field is private\n\
                 4. Check if you're using the right variant for enums",
    },
    HintEntry {
        code: "E0614",
        short: "Cannot dereference this type.",
        detail: "Common fixes:\n\
                 1. Implement Deref for your type\n\
                 2. Use .as_ref() or .borrow() instead of *\n\
                 3. Check if you need & instead of *\n\
                 4. Use pattern matching to destructure",
    },
    HintEntry {
        code: "E0728",
        short: "`await` is only allowed inside `async` functions and blocks.",
        detail: "Common fixes:\n\
                 1. Mark the function as async: async fn foo()\n\
                 2. Wrap the call in an async block: async { ... }.await\n\
                 3. Use block_on() from your async runtime at the top level\n\
                 4. Use #[tokio::main] or #[async_std::main] for main()",
    },
];

/// Look up a hint for a given error code.
pub fn get_hint(code: &str) -> Option<&'static str> {
    HINT_DB.iter().find(|h| h.code == code).map(|h| h.short)
}

/// Show the full explanation for an error code.
pub fn explain(code: &str) -> Result<()> {
    // Normalize: accept both "E0502" and "0502"
    let code = if code.starts_with('E') {
        code.to_string()
    } else {
        format!("E{code}")
    };

    if let Some(entry) = HINT_DB.iter().find(|h| h.code == code) {
        println!("{}", format!("{} — {}", entry.code, entry.short).bold());
        println!("{}", "─".repeat(60));
        println!();
        println!("{}", entry.detail);
        println!();

        // Also show rustc's official explanation
        println!("{}", "Official explanation:".dimmed());
        println!("  {}", format!("rustc --explain {}", entry.code).dimmed());
    } else {
        // Fall back to rustc --explain
        crate::output::info(&format!(
            "no rx hint for {code}, showing rustc explanation..."
        ));
        let status = std::process::Command::new("rustc")
            .args(["--explain", &code])
            .status();
        match status {
            Ok(s) if s.success() => {}
            _ => {
                anyhow::bail!(
                    "unknown error code: {code}\n\
                     hint: error codes look like E0502, E0308, etc."
                );
            }
        }
    }

    Ok(())
}
