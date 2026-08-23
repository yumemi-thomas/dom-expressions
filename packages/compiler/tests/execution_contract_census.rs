//! Corpus-wide reconciliation of the semantic-trace execution contract.
//!
//! The source census is independent of DOM lowering. Compiling the complete
//! Babel fixture corpus and adversarial probe corpus with tracing enabled makes
//! every lowering path prove that it reported the sites the census found. The
//! same corpus is then compiled with tracing disabled to prove that trace
//! collection is additive and cannot change generated code.
#![cfg(not(feature = "node"))]

use std::path::{Path, PathBuf};

use dom_expressions_compiler::{compile, CompileOptions};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../babel-plugin-jsx/test")
        .canonicalize()
        .expect("the Babel fixture corpus is a workspace sibling")
}

/// Every DOM fixture source in the Babel corpus. Semantic tracing is currently
/// a DOM-only compiler contract, so SSR/universal fixture directories are
/// intentionally outside this producer-stage census.
fn fixture_sources() -> Vec<(String, String)> {
    let mut sources = Vec::new();
    let root = fixture_root();
    let mut dirs = std::fs::read_dir(&root)
        .expect("fixture root is readable")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("__dom").then(|| (name, entry.path()))
        })
        .collect::<Vec<_>>();
    dirs.sort();

    for (dir_name, dir) in dirs {
        let mut fixtures = std::fs::read_dir(&dir)
            .expect("fixture directory is readable")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let code = entry.path().join("code.js");
                code.exists().then(|| {
                    (
                        format!("{dir_name}/{}", entry.file_name().to_string_lossy()),
                        code,
                    )
                })
            })
            .collect::<Vec<_>>();
        fixtures.sort();

        for (id, path) in fixtures {
            sources.push((id, std::fs::read_to_string(path).expect("fixture is utf-8")));
        }
    }
    sources
}

/// Read the adversarial JSX probe cases from the parity suite so the
/// reconciliation corpus cannot silently drift away from the cases that
/// exercise compiler output. Probe sources are JavaScript template literals;
/// escaped backticks and interpolation markers must be restored before parse.
fn probe_sources() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("__tests__/parity-probes.test.js");
    let text = std::fs::read_to_string(path).expect("the probe corpus is readable");
    let body = {
        let start = text
            .find("const cases = {")
            .expect("probe corpus has cases");
        let end = text
            .find("describe(\"Babel vs Oxc parity probes\"")
            .expect("probe corpus has an end");
        &text[start..end]
    };

    let mut cases = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("\n  \"") {
        let after = &rest[open + 4..];
        let Some(name_end) = after.find("\": `") else {
            rest = after;
            continue;
        };
        let name = &after[..name_end];
        let source_start = &after[name_end + 4..];

        let mut source_end = None;
        let bytes = source_start.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' => index += 2,
                b'`' => {
                    source_end = Some(index);
                    break;
                }
                _ => index += 1,
            }
        }
        let Some(source_end) = source_end else {
            panic!("probe {name} has no closing template literal");
        };

        let source = source_start[..source_end]
            .replace("\\`", "`")
            .replace("\\${", "${");
        cases.push((name.to_string(), source));
        rest = &source_start[source_end..];
    }
    // These parity probes exercise existing 2.0 output edge cases whose
    // syntax is deliberately discarded or shadowed before a semantic wrapper
    // exists. They remain in the Babel/Oxc parity suite; the trace census is
    // limited to probes with a source site that the current DOM contract can
    // observe. The remaining corpus is still well above 400 cases.
    let excluded = [
        "void elements discard children",
        "stateful property aliases use last value",
    ];
    cases
        .into_iter()
        .filter(|(name, _)| !excluded.contains(&name.as_str()))
        .collect()
}

fn options(semantic_trace: bool) -> CompileOptions {
    CompileOptions {
        module_name: "r-dom".into(),
        built_ins: vec!["For".into(), "Show".into()],
        static_marker: "@once".into(),
        semantic_trace,
        ..CompileOptions::default()
    }
}

#[test]
fn every_fixture_reconciles_census_against_lowering() {
    let sources = fixture_sources();
    assert!(
        sources.len() > 40,
        "expected the full fixture corpus, found {}",
        sources.len()
    );

    let mut failures = Vec::new();
    let mut reconciled = 0;
    for (id, source) in &sources {
        match compile(source, &options(true)) {
            Ok(output) => {
                assert!(
                    output.semantic_trace.is_some(),
                    "{id}: tracing was requested but no trace came back"
                );
                reconciled += 1;
            }
            Err(error) => {
                let message = error.to_string();
                // The Babel corpus includes a small set of inputs that Oxc
                // intentionally rejects. Only semantic reconciliation errors
                // belong to this census.
                if message.contains("semantic ") {
                    failures.push(format!("{id}: {message}"));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} fixtures failed contract reconciliation:\n{}",
        failures.len(),
        sources.len(),
        failures.join("\n")
    );
    assert!(
        reconciled > 40,
        "expected most fixtures to produce a contract, got {reconciled}"
    );
}

#[test]
fn every_parity_probe_reconciles_census_against_lowering() {
    let sources = probe_sources();
    assert!(
        sources.len() > 400,
        "expected the full probe corpus, extracted {}",
        sources.len()
    );
    assert!(
        sources.iter().any(|(_, source)| source.contains('`')),
        "template-literal probes were truncated during extraction"
    );

    let mut failures = Vec::new();
    let mut reconciled = 0;
    for (name, source) in &sources {
        match compile(source, &options(true)) {
            Ok(output) => {
                assert!(
                    output.semantic_trace.is_some(),
                    "{name}: tracing was requested but no trace came back"
                );
                reconciled += 1;
            }
            Err(error) => {
                let message = error.to_string();
                if message.contains("semantic ") {
                    failures.push(format!("{name}: {message}"));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} probes failed contract reconciliation:\n{}",
        failures.len(),
        sources.len(),
        failures.join("\n")
    );
    assert!(
        reconciled > 400,
        "expected most probes to produce a contract, got {reconciled}"
    );
}

/// Semantic tracing is an observation-only side channel. Every input that
/// compiles with tracing disabled must emit byte-identical code when tracing
/// is enabled, while the trace itself is present only in the latter result.
#[test]
fn tracing_does_not_change_generated_output() {
    for (id, source) in fixture_sources() {
        let plain = match compile(&source, &options(false)) {
            Ok(output) => output,
            Err(_) => continue,
        };
        let traced = compile(&source, &options(true))
            .unwrap_or_else(|error| panic!("{id}: tracing failed: {error}"));
        assert_eq!(plain.code, traced.code, "output changed for {id}");
        assert!(plain.semantic_trace.is_none());
        assert!(traced.semantic_trace.is_some());
    }

    for (name, source) in probe_sources() {
        let plain = match compile(&source, &options(false)) {
            Ok(output) => output,
            Err(_) => continue,
        };
        let traced = compile(&source, &options(true))
            .unwrap_or_else(|error| panic!("{name}: tracing failed: {error}"));
        assert_eq!(plain.code, traced.code, "output changed for probe {name}");
        assert!(plain.semantic_trace.is_none());
        assert!(traced.semantic_trace.is_some());
    }
}
