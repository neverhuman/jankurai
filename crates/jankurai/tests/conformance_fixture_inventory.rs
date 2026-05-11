use std::fs;
use std::path::Path;

#[test]
fn conformance_fixture_inventory_matches_paper_seed_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");
    let fixtures = root.join("conformance/fixtures");
    let expected = root.join("conformance/expected");

    let fixture_count = fs::read_dir(&fixtures)
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().path().is_dir())
        .count();
    assert_eq!(fixture_count, 10);

    for name in [
        "hl3-pass-minimal",
        "ownerless-path-fail",
        "unmapped-proof-fail",
        "generated-zone-mutation-fail",
        "secret-sprawl-fail",
        "destructive-migration-fail",
        "authz-isolation-fail",
        "input-boundary-xss-fail",
        "overbroad-agency-fail",
        "rendered-ux-gap-fail",
    ] {
        assert!(fixtures.join(name).exists(), "missing fixture {name}");
        assert!(
            fixtures.join(name).join("jankurai-fixture.toml").exists(),
            "missing fixture manifest {name}"
        );
    }

    let expected_json_count = fs::read_dir(&expected)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .count();
    assert_eq!(expected_json_count, 12);
}
