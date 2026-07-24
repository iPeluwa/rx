/// Curated database of helpful one-line hints for common Rust error codes,
/// surfaced next to cargo output.
struct HintEntry {
    code: &'static str,
    short: &'static str,
}

const HINT_DB: &[HintEntry] = &[
    HintEntry {
        code: "E0502",
        short: "Cannot borrow as mutable because it's also borrowed as immutable.",
    },
    HintEntry {
        code: "E0499",
        short: "Cannot borrow as mutable more than once at a time.",
    },
    HintEntry {
        code: "E0308",
        short: "Mismatched types.",
    },
    HintEntry {
        code: "E0382",
        short: "Use of moved value.",
    },
    HintEntry {
        code: "E0277",
        short: "Trait not implemented.",
    },
    HintEntry {
        code: "E0425",
        short: "Cannot find value or function in this scope.",
    },
    HintEntry {
        code: "E0433",
        short: "Failed to resolve path — module or type not found.",
    },
    HintEntry {
        code: "E0599",
        short: "No method found for this type.",
    },
    HintEntry {
        code: "E0061",
        short: "Wrong number of arguments.",
    },
    HintEntry {
        code: "E0106",
        short: "Missing lifetime specifier.",
    },
    HintEntry {
        code: "E0597",
        short: "Value does not live long enough.",
    },
    HintEntry {
        code: "E0507",
        short: "Cannot move out of borrowed content.",
    },
    HintEntry {
        code: "E0283",
        short: "Type annotations needed — cannot infer type.",
    },
    HintEntry {
        code: "E0271",
        short: "Type mismatch in trait implementation.",
    },
    HintEntry {
        code: "E0405",
        short: "Cannot find trait in this scope.",
    },
    HintEntry {
        code: "E0432",
        short: "Unresolved import.",
    },
    HintEntry {
        code: "E0603",
        short: "Item is private.",
    },
    HintEntry {
        code: "E0658",
        short: "Feature not stabilized yet.",
    },
    HintEntry {
        code: "E0015",
        short: "Cannot call non-const function in const context.",
    },
    HintEntry {
        code: "E0373",
        short: "Closure may outlive the current function, but it borrows a variable.",
    },
    HintEntry {
        code: "E0412",
        short: "Cannot find type name in this scope.",
    },
    HintEntry {
        code: "E0463",
        short: "Can't find crate.",
    },
    HintEntry {
        code: "E0609",
        short: "No field on this type.",
    },
    HintEntry {
        code: "E0614",
        short: "Cannot dereference this type.",
    },
    HintEntry {
        code: "E0728",
        short: "`await` is only allowed inside `async` functions and blocks.",
    },
];

/// Look up a hint for a given error code.
pub fn get_hint(code: &str) -> Option<&'static str> {
    HINT_DB.iter().find(|h| h.code == code).map(|h| h.short)
}
