//! Focused semantic-trace contract tests.
//!
//! The corpus census proves coverage at scale. These tests pin the producer
//! facts that the consumer must interpret: authoritative control-flow
//! classification, wrapper observations, shared effect groups, component
//! render sites, and deferred callback receiver spans.
#![cfg(not(feature = "node"))]

use dom_expressions_compiler::{
    compile, CallbackDecision, CompileOptions, ExecutionSiteKind, SemanticTrace, TerminalDecision,
    ValueDecision, Wrapper,
};

fn options(semantic_trace: bool) -> CompileOptions {
    CompileOptions {
        module_name: "r-dom".into(),
        built_ins: vec!["For".into(), "Show".into()],
        static_marker: "@once".into(),
        semantic_trace,
        ..CompileOptions::default()
    }
}

fn trace(source: &str) -> SemanticTrace {
    compile(source, &options(true))
        .expect("compile with semantic tracing")
        .semantic_trace
        .expect("semantic trace")
}

fn source_text<'a>(source: &'a str, start: u32, end: u32) -> &'a str {
    &source[start as usize..end as usize]
}

#[test]
fn control_flow_render_is_authoritative_and_requires_configuration() {
    let source = r#"const C = () => <Show>{() => <span>{value()}</span>}</Show>;"#;
    let configured = trace(source);
    assert!(configured
        .sites
        .contains(&dom_expressions_compiler::ExecutionSite {
            span: configured
                .sites
                .iter()
                .find(|site| source_text(source, site.span.start, site.span.end)
                    == "() => <span>{value()}</span>")
                .expect("function child site")
                .span,
            kind: ExecutionSiteKind::ControlFlowRender,
            decision: TerminalDecision::Callback(CallbackDecision::LaterRender),
        }));

    let unconfigured = compile(
        source,
        &CompileOptions {
            built_ins: Vec::new(),
            ..options(true)
        },
    )
    .expect("compile unconfigured built-in")
    .semantic_trace
    .expect("semantic trace");
    assert!(unconfigured.sites.iter().any(|site| {
        source_text(source, site.span.start, site.span.end) == "() => <span>{value()}</span>"
            && site.kind == ExecutionSiteKind::ComponentChild
            && site.decision == TerminalDecision::Value(ValueDecision::EagerOnce)
    }));

    let shadowed_source = r#"const Show = Thing; const C = () => <Show>{() => value()}</Show>;"#;
    let shadowed = trace(shadowed_source);
    assert!(shadowed.sites.iter().any(|site| {
        source_text(shadowed_source, site.span.start, site.span.end) == "() => value()"
            && site.kind == ExecutionSiteKind::ComponentChild
    }));
}

#[test]
fn owner_facts_preserve_wrapper_identity_and_shared_effect_groups() {
    let source = r#"const C = (props) => <div title={props.title} id={props.id} />;"#;
    let rendered = trace(source);
    let effects = rendered
        .owner_establishments
        .iter()
        .filter(|fact| fact.wrapper == "effect")
        .map(|fact| {
            (
                source_text(source, fact.span.start, fact.span.end),
                fact.group_id,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        effects,
        [("title={props.title}", Some(0)), ("id={props.id}", Some(0))]
    );

    let custom = compile(
        "const C = (props) => <div title={props.value} />;",
        &CompileOptions {
            effect_wrapper: Wrapper::Name("createRenderEffect".into()),
            ..options(true)
        },
    )
    .expect("compile custom effect wrapper")
    .semantic_trace
    .expect("semantic trace");
    assert!(custom.owner_establishments.iter().any(|fact| {
        fact.wrapper == "createRenderEffect"
            && source_text(
                "const C = (props) => <div title={props.value} />;",
                fact.span.start,
                fact.span.end,
            ) == "title={props.value}"
    }));
}

#[test]
fn owner_facts_cover_insert_events_refs_and_the_2_0_scope_wrapper() {
    let source = r#"const C = (props) => <div title={props.title} onClick={props.onClick} ref={props.ref}>{props.child}</div>;"#;
    let rendered = trace(source);
    let facts = rendered
        .owner_establishments
        .iter()
        .map(|fact| {
            (
                fact.wrapper.as_str(),
                source_text(source, fact.span.start, fact.span.end),
            )
        })
        .collect::<Vec<_>>();
    for expected in [
        ("effect", "title={props.title}"),
        (
            "insert",
            "<div title={props.title} onClick={props.onClick} ref={props.ref}>{props.child}</div>",
        ),
        ("addEventListener", "onClick={props.onClick}"),
        ("ref-apply", "ref={props.ref}"),
    ] {
        assert!(facts.contains(&expected), "missing owner fact {expected:?}");
    }

    let hydration_source = "const C = (props) => <div>{props.child()}</div>;";
    let hydration = compile(
        hydration_source,
        &CompileOptions {
            hydratable: true,
            ..options(true)
        },
    )
    .expect("compile hydratable source")
    .semantic_trace
    .expect("semantic trace");
    assert!(hydration
        .owner_establishments
        .iter()
        .any(|fact| fact.wrapper == "scope"));
}

#[test]
fn component_render_and_deferred_callback_facts_are_spans_only() {
    let source =
        r#"const C = (props) => <Thing label={props.label} ref={props.ref} {...props.data} />;"#;
    let rendered = trace(source);
    assert_eq!(
        rendered
            .component_render_sites
            .iter()
            .map(|fact| source_text(source, fact.span.start, fact.span.end))
            .collect::<Vec<_>>(),
        ["<Thing label={props.label} ref={props.ref} {...props.data} />"]
    );
    let callbacks = rendered
        .deferred_callback_sites
        .iter()
        .map(|fact| {
            (
                source_text(source, fact.span.start, fact.span.end),
                source_text(source, fact.receiver_span.start, fact.receiver_span.end),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        callbacks,
        [
            (
                "props.label",
                "<Thing label={props.label} ref={props.ref} {...props.data} />",
            ),
            (
                "props.ref",
                "<Thing label={props.label} ref={props.ref} {...props.data} />",
            ),
            (
                "props.data",
                "<Thing label={props.label} ref={props.ref} {...props.data} />",
            ),
        ]
    );
}

#[test]
fn disabled_wrappers_do_not_invent_wrapper_facts() {
    let source = "const C = (props) => <div title={props.title}>{props.child}</div>;";
    let rendered = compile(
        source,
        &CompileOptions {
            effect_wrapper: Wrapper::Disabled,
            memo_wrapper: Wrapper::Disabled,
            ..options(true)
        },
    )
    .expect("compile with disabled wrappers")
    .semantic_trace
    .expect("semantic trace");
    assert!(rendered
        .owner_establishments
        .iter()
        .all(|fact| fact.wrapper != "effect" && fact.wrapper != "memo"));
}
