use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

pub fn generate_sbom(format: &str, output: Option<&str>) -> Result<()> {
    match format {
        "spdx" => generate_spdx(output),
        "cyclonedx" => generate_cyclonedx(output),
        other => {
            anyhow::bail!(
                "unknown SBOM format: {other}\n\
                 supported: spdx, cyclonedx"
            );
        }
    }
}

fn generate_spdx(output: Option<&str>) -> Result<()> {
    crate::output::info("generating SPDX SBOM...");

    // Get cargo metadata for dependency info
    let metadata = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .output()
        .context("failed to run cargo metadata")?;

    if !metadata.status.success() {
        anyhow::bail!("cargo metadata failed");
    }

    let meta: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).context("failed to parse cargo metadata")?;

    let packages = meta
        .get("packages")
        .and_then(|p| p.as_array())
        .context("no packages in metadata")?;

    let root_name = meta
        .get("resolve")
        .and_then(|r| r.get("root"))
        .and_then(|r| r.as_str())
        .unwrap_or("unknown");

    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let mut spdx = String::new();
    spdx.push_str("SPDXVersion: SPDX-2.3\n");
    spdx.push_str("DataLicense: CC0-1.0\n");
    spdx.push_str("SPDXID: SPDXRef-DOCUMENT\n");
    spdx.push_str(&format!("DocumentName: {root_name}\n"));
    spdx.push_str(&format!(
        "DocumentNamespace: https://spdx.org/spdxdocs/{root_name}\n"
    ));
    spdx.push_str("Creator: Tool: rx\n");
    spdx.push_str(&format!("Created: {timestamp}\n"));
    spdx.push('\n');

    for pkg in packages {
        let name = pkg.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        let license = pkg
            .get("license")
            .and_then(|l| l.as_str())
            .unwrap_or("NOASSERTION");

        let spdx_id = format!(
            "SPDXRef-Package-{}-{}",
            name.replace(['-', '.'], ""),
            version.replace('.', "")
        );

        spdx.push_str(&format!("PackageName: {name}\n"));
        spdx.push_str(&format!("SPDXID: {spdx_id}\n"));
        spdx.push_str(&format!("PackageVersion: {version}\n"));
        spdx.push_str("PackageDownloadLocation: https://crates.io\n");
        spdx.push_str(&format!("PackageLicenseConcluded: {license}\n"));
        spdx.push_str(&format!("PackageLicenseDeclared: {license}\n"));
        spdx.push_str("FilesAnalyzed: false\n");
        spdx.push('\n');
    }

    if let Some(path) = output {
        fs::write(path, &spdx)?;
        crate::output::success(&format!(
            "SPDX SBOM written to {path} ({} packages)",
            packages.len()
        ));
    } else {
        print!("{spdx}");
    }

    Ok(())
}

fn generate_cyclonedx(output: Option<&str>) -> Result<()> {
    crate::output::info("generating CycloneDX SBOM...");

    let metadata = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .output()
        .context("failed to run cargo metadata")?;

    if !metadata.status.success() {
        anyhow::bail!("cargo metadata failed");
    }

    let meta: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).context("failed to parse cargo metadata")?;

    let packages = meta
        .get("packages")
        .and_then(|p| p.as_array())
        .context("no packages in metadata")?;

    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let mut components = Vec::new();
    for pkg in packages {
        let name = pkg.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("?");
        let license = pkg.get("license").and_then(|l| l.as_str());
        let description = pkg.get("description").and_then(|d| d.as_str());

        let mut component = serde_json::json!({
            "type": "library",
            "name": name,
            "version": version,
            "purl": format!("pkg:cargo/{name}@{version}")
        });

        if let Some(lic) = license {
            component["licenses"] = serde_json::json!([{
                "license": { "id": lic }
            }]);
        }
        if let Some(desc) = description {
            component["description"] = serde_json::json!(desc);
        }

        components.push(component);
    }

    let bom = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "timestamp": timestamp,
            "tools": [{
                "name": "rx",
                "version": env!("CARGO_PKG_VERSION")
            }]
        },
        "components": components
    });

    let json = serde_json::to_string_pretty(&bom)?;

    if let Some(path) = output {
        fs::write(path, &json)?;
        crate::output::success(&format!(
            "CycloneDX SBOM written to {path} ({} components)",
            packages.len()
        ));
    } else {
        println!("{json}");
    }

    Ok(())
}
