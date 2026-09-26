use super::*;

#[test]
fn cache_hit_skips_both_audit_callbacks_and_attributes_zero_work() {
    let compiled = native(true);
    let plugin = spec("cache-calls-output", "output");
    let mut calls = 0;
    let mut callback = |_| {
        calls += 1;
        Ok("[{\"placement\":\"prepend\",\"comment\":\"cached\"}]".to_owned())
    };
    let (first, first_cost) = run(
        &compiled,
        &plugin,
        "config",
        true,
        true,
        None,
        &mut callback,
    )
    .unwrap();
    let (second, cost) = run(
        &compiled,
        &plugin,
        "config",
        true,
        true,
        None,
        &mut callback,
    )
    .unwrap();
    assert_eq!(calls, 2);
    assert!(!first_cost.cached && cost.cached);
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(second).unwrap()
    );
    assert_eq!((cost.batch_bytes, cost.operations), (0, 0));
    assert_eq!(cost.js_ns, 0.0);
}

#[test]
fn audited_nondeterminism_and_invalid_responses_never_enter_cache() {
    let compiled = native(false);
    let plugin = spec("audit-output", "output");
    let mut calls = 0;
    assert!(
        run(&compiled, &plugin, "config", true, true, None, |_| {
            calls += 1;
            Ok(
                serde_json::json!([{ "placement": "prepend", "comment": calls.to_string() }])
                    .to_string(),
            )
        })
        .unwrap_err()
        .contains("nondeterministic")
    );
    assert_eq!(calls, 2);
    let (_, cost) = run(&compiled, &plugin, "config", true, true, None, |_| {
        Ok("[]".to_owned())
    })
    .unwrap();
    assert!(!cost.cached);
    let plugin = spec("invalid-output-cache", "output");
    assert!(
        run(&compiled, &plugin, "config", true, true, None, |_| Ok(
            "{}".to_owned()
        ))
        .is_err()
    );
    let (_, cost) = run(&compiled, &plugin, "config", true, true, None, |_| {
        Ok("[]".to_owned())
    })
    .unwrap();
    assert!(!cost.cached);
}

#[test]
fn complete_pipeline_and_identity_are_part_of_the_content_key() {
    let compiled = native(true);
    let plugin = spec("identity-output", "output");
    let key = content_key_for_build(&compiled, &plugin, "config", true, "build-a").unwrap();
    for (config, audit, build) in [
        ("changed", true, "build-a"),
        ("config", false, "build-a"),
        ("config", true, "build-b"),
    ] {
        assert_ne!(
            key,
            content_key_for_build(&compiled, &plugin, config, audit, build).unwrap()
        );
    }
    let mutations: [fn(&mut CompileResult); 6] = [
        |result| result.code.push('\n'),
        |result| result.preamble.push('\n'),
        |result| result.ast = serde_json::json!({"modified": true}),
        |result| result.map = None,
        |result| result.templates = Some(vec!["changed".to_owned()]),
        |result| result.helpers.push("new-helper".to_owned()),
    ];
    for mutate in mutations {
        let mut changed = compiled.clone();
        mutate(&mut changed);
        assert_ne!(
            key,
            content_key_for_build(&changed, &plugin, "config", true, "build-a").unwrap()
        );
    }
    let inputs = [CacheInput {
        name: "style".to_owned(),
        value: "tabs".to_owned(),
    }];
    let variants = [
        PluginSpec {
            version: "2",
            ..plugin
        },
        PluginSpec {
            fingerprint: "new-code",
            ..plugin
        },
        PluginSpec {
            family: "formatter",
            ..plugin
        },
        PluginSpec {
            inputs: Some(&inputs),
            ..plugin
        },
    ];
    for variant in variants {
        assert_ne!(
            key,
            content_key_for_build(&compiled, &variant, "config", true, "build-a").unwrap()
        );
    }
}

#[test]
fn cache_requires_declared_unique_inputs_even_on_a_hit() {
    let compiled = native(false);
    let plugin = spec("validation-output", "output");
    assert!(
        run(
            &compiled,
            &PluginSpec {
                inputs: None,
                ..plugin
            },
            "config",
            true,
            true,
            None,
            |_| panic!()
        )
        .is_err()
    );
    let inputs = [
        CacheInput {
            name: "x".to_owned(),
            value: "a".to_owned(),
        },
        CacheInput {
            name: "x".to_owned(),
            value: "b".to_owned(),
        },
    ];
    assert!(
        run(
            &compiled,
            &PluginSpec {
                inputs: Some(&inputs),
                ..plugin
            },
            "config",
            true,
            true,
            None,
            |_| panic!()
        )
        .is_err()
    );
}

#[test]
fn cached_operations_cannot_disable_determinism_audits() {
    let compiled = native(false);
    let plugin = spec("cache-forces-output-audit", "output");
    let mut calls = 0;
    let error = run(&compiled, &plugin, "config", true, false, None, |_| {
        calls += 1;
        Ok(
            serde_json::json!([{ "placement": "append", "comment": calls.to_string() }])
                .to_string(),
        )
    })
    .unwrap_err();
    assert_eq!(
        error,
        "plugin `cache-forces-output-audit` returned nondeterministic output"
    );
    assert_eq!(calls, 2);
    let (_, cost) = run(&compiled, &plugin, "config", true, true, None, |_| {
        calls += 1;
        Ok("[]".to_owned())
    })
    .unwrap();
    assert_eq!(calls, 4);
    assert!(!cost.cached);
}
