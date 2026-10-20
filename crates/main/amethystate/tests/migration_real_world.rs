#![cfg(not(target_arch = "wasm32"))]

use amethystate_core::test_utils::TempPath;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize)]
struct Research {
    releases: Vec<Release>,
}

#[derive(Deserialize)]
struct Release {
    tag: String,
    commit: String,
    schema: u32,
    source: String,
    expected: String,
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn changes(previous: &Value, current: &Value, path: &str, output: &mut Vec<Value>) {
    match (previous, current) {
        (Value::Object(before), Value::Object(after)) => {
            let names: BTreeSet<_> = before.keys().chain(after.keys()).collect();
            for name in names {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                let at = format!("{path}/{escaped}");
                match (before.get(name), after.get(name)) {
                    (Some(before), Some(after)) => changes(before, after, &at, output),
                    (Some(before), None) => output.push(json!({"remove": at, "was": before})),
                    (None, Some(after)) => output.push(json!({"add": at, "value": after})),
                    (None, None) => unreachable!(),
                }
            }
        }
        _ if previous != current => {
            output.push(json!({"replace": path, "was": previous, "value": current}))
        }
        _ => {}
    }
}

#[test]
#[ignore = "run once per OS in the dedicated migration histories CI step"]
fn halloy_data_lives_from_its_first_release_to_2026_9() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/halloy_history");
    let research: Research =
        serde_json::from_slice(&std::fs::read(fixture.join("research.json")).unwrap()).unwrap();
    assert_eq!(research.releases.len(), 43);
    assert_eq!(research.releases.first().unwrap().tag, "23.1-alpha1");
    assert_eq!(research.releases.last().unwrap().tag, "2026.9");

    let history = TempPath::new("halloy_release_history");
    let folder = history.path().with_extension("checkpoints");
    std::fs::create_dir_all(&folder).unwrap();
    let data = folder.join("settings.json");
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/halloy-history");
    let executable = target.join("debug").join(format!(
        "halloy-history-app{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut last_schema = 0;
    let mut previous_snapshot: Option<std::path::PathBuf> = None;
    let mut previous_expected = None;

    for release in research.releases {
        assert_eq!(release.commit.len(), 40);
        assert!(release.source.contains(&release.commit));
        let new_schema = release.schema != last_schema;
        if new_schema {
            assert_eq!(release.schema, last_schema + 1);
            let build = Command::new(env!("CARGO"))
                .args([
                    "build",
                    "--offline",
                    "--locked",
                    "--quiet",
                    "--no-default-features",
                    "--features",
                ])
                .arg(format!("s{:02}", release.schema))
                .arg("--manifest-path")
                .arg(fixture.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&target)
                .output()
                .unwrap();
            assert!(
                build.status.success(),
                "building Halloy {}:\n{}",
                release.tag,
                String::from_utf8_lossy(&build.stderr)
            );
        }

        eprintln!(
            "Halloy {}: open schema {}, check, save, snapshot, compare",
            release.tag, release.schema
        );
        let snapshot_path = folder.join(format!("{}.json", release.tag));
        let run = Command::new(&executable)
            .arg(&data)
            .arg(&snapshot_path)
            .arg(new_schema.to_string())
            .arg(release.schema.to_string())
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "Halloy {}:\n{}\n{}",
            release.tag,
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );

        let snapshot = read_json(&snapshot_path);
        let expected = read_json(&fixture.join(&release.expected));
        assert_eq!(snapshot["schema"], release.schema);
        assert_eq!(
            snapshot["data"], expected,
            "saved snapshot of Halloy {}",
            release.tag
        );
        assert_eq!(
            snapshot["applied"],
            if new_schema && release.schema > 1 {
                json!([release.schema])
            } else {
                json!([])
            }
        );

        if let (Some(previous_path), Some(previous_expected)) =
            (&previous_snapshot, &previous_expected)
        {
            let previous = read_json(previous_path);
            let mut actual_changes = Vec::new();
            let mut expected_changes = Vec::new();
            changes(
                &previous["data"],
                &snapshot["data"],
                "",
                &mut actual_changes,
            );
            changes(previous_expected, &expected, "", &mut expected_changes);
            assert_eq!(
                actual_changes, expected_changes,
                "changes from the preceding release to {}",
                release.tag
            );
        }

        previous_snapshot = Some(snapshot_path);
        previous_expected = Some(expected);
        last_schema = release.schema;
    }
    assert_eq!(last_schema, 25);
}
