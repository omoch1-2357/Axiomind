// Documentation validation test
// This test ensures all required documentation files exist and contain necessary content

use std::fs;
use std::path::Path;

#[test]
fn required_documentation_exists() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));

    // README.md must exist
    let readme = base.join("README.md");
    assert!(readme.exists(), "README.md must exist");

    let readme_content = fs::read_to_string(&readme).expect("Failed to read README.md");
    assert!(
        readme_content.contains("# axm_web"),
        "README must have title"
    );
    assert!(
        readme_content.contains("Setup"),
        "README must have setup section"
    );
    assert!(
        readme_content.contains("Usage"),
        "README must have usage section"
    );

    // API.md must exist
    let api_doc = base.join("API.md");
    assert!(api_doc.exists(), "API.md must exist");

    let api_content = fs::read_to_string(&api_doc).expect("Failed to read API.md");
    assert!(api_content.contains("API"), "API.md must document API");
    assert!(
        api_content.contains("/api/sessions"),
        "API.md must document session endpoints"
    );
    assert!(
        api_content.contains("POST"),
        "API.md must document POST methods"
    );
    assert!(
        api_content.contains("GET"),
        "API.md must document GET methods"
    );

    // DEPLOYMENT.md must exist
    let deployment = base.join("DEPLOYMENT.md");
    assert!(deployment.exists(), "DEPLOYMENT.md must exist");

    let deployment_content = fs::read_to_string(&deployment).expect("Failed to read DEPLOYMENT.md");
    assert!(
        deployment_content.contains("Configuration"),
        "DEPLOYMENT.md must have configuration section"
    );
    assert!(
        deployment_content.contains("port"),
        "DEPLOYMENT.md must document port configuration"
    );

    // TROUBLESHOOTING.md must exist
    let troubleshooting = base.join("TROUBLESHOOTING.md");
    assert!(troubleshooting.exists(), "TROUBLESHOOTING.md must exist");

    let troubleshooting_content =
        fs::read_to_string(&troubleshooting).expect("Failed to read TROUBLESHOOTING.md");
    assert!(
        troubleshooting_content.contains("Server Issues")
            || troubleshooting_content.contains("Common Issues"),
        "TROUBLESHOOTING.md must document common issues"
    );
}

#[test]
fn workspace_integration_is_correct() {
    // Verify Cargo.toml is properly configured
    let cargo_toml = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).expect("Failed to read Cargo.toml");

    assert!(content.contains("axm_web"), "Package name must be axm_web");
    assert!(content.contains("axm-engine"), "Must depend on axm-engine");
    assert!(content.contains("warp"), "Must have warp dependency");
    assert!(content.contains("tokio"), "Must have tokio dependency");
}

#[test]
fn root_workspace_includes_web_crate() {
    // Verify root workspace includes rust/web
    let workspace_cargo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Cargo.toml");

    if workspace_cargo.exists() {
        let content =
            fs::read_to_string(&workspace_cargo).expect("Failed to read workspace Cargo.toml");
        assert!(
            content.contains("rust/web"),
            "Workspace must include rust/web member"
        );
    }
}
