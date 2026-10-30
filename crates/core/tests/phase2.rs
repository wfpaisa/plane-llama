// Regresiones de métricas, GGUF, estimación y utilidades puras del cliente.
use plane_llama_core::{
    client::{flag_writer::*, format::*, store_logic::*, vram_estimate::*},
    optimizer::*,
    *,
};
#[test]
fn prompt_timing_never_becomes_generation() {
    let entries = vec![LogEntry {
        msg: "prompt eval time = 10 ms / 2 tokens (200 tokens per second)".into(),
        ..Default::default()
    }];
    let metrics = metrics::parse_metrics_from_logs(&entries);
    assert_eq!(metrics.prompt_tokens_per_second, Some(200.));
    assert_eq!(metrics.generation_tokens_per_second, None);
    assert_eq!(metrics.generation_token_count, None);
    assert_eq!(metrics.generation_time_ms, None);
    let mut entries = vec![LogEntry {
        msg: "eval time = 200 ms / 10 tokens (50 tokens per second)".into(),
        ..Default::default()
    }];
    entries.extend(vec![LogEntry {
        msg: "prompt eval time = 10 ms / 2 tokens (200 tokens per second)".into(),
        ..Default::default()
    }]);
    assert_eq!(
        metrics::parse_metrics_from_logs(&entries).generation_tokens_per_second,
        Some(50.)
    );
}
fn string(bytes: &mut Vec<u8>, s: &str) {
    bytes.extend((s.len() as u64).to_le_bytes());
    bytes.extend(s.as_bytes());
}
fn scalar(bytes: &mut Vec<u8>, key: &str, value: u32) {
    string(bytes, key);
    bytes.extend(4_u32.to_le_bytes());
    bytes.extend(value.to_le_bytes());
}
#[test]
fn gguf_architecture_and_truncation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("model.gguf");
    let mut bytes = b"GGUF".to_vec();
    bytes.extend(3_u32.to_le_bytes());
    bytes.extend(0_u64.to_le_bytes());
    bytes.extend(7_u64.to_le_bytes());
    scalar(&mut bytes, "qwen.full_attention_interval", 4);
    scalar(&mut bytes, "qwen.block_count", 32);
    scalar(&mut bytes, "qwen.attention.head_count_kv", 8);
    scalar(&mut bytes, "qwen.attention.key_length", 128);
    scalar(&mut bytes, "qwen.attention.value_length", 128);
    scalar(&mut bytes, "qwen.context_length", 32768);
    assert_eq!(parse_gguf_header(&bytes).attention_layers, Some(8.));
    string(&mut bytes, "qwen.attention.recurrent_layers");
    bytes.extend(9_u32.to_le_bytes());
    bytes.extend(7_u32.to_le_bytes());
    bytes.extend(4_u64.to_le_bytes());
    bytes.extend([1, 0, 1, 0]);
    std::fs::write(&file, &bytes).unwrap();
    std::fs::write(dir.path().join("mmproj.gguf"), b"GGUF").unwrap();
    let arch = read_gguf_arch(&file);
    assert_eq!(arch.layers, Some(32.));
    assert_eq!(arch.attention_layers, Some(2.));
    assert_eq!(arch.kv_heads, Some(8.));
    assert_eq!(arch.context_length, Some(32768.));
    for end in 0..bytes.len() {
        let _ = parse_gguf_header(&bytes[..end]);
    }
    let resolved = resolve_model_file(None, Some(file.to_str().unwrap()));
    assert_eq!(resolved.arch, Some(arch));
    assert!(resolved.mmproj_file.is_some());
    assert_eq!(resolved.size_mi_b, Some(bytes.len() as f64 / 1048576.));
    let from_dir = resolve_model_file(None, Some(dir.path().to_str().unwrap()));
    assert_eq!(from_dir.file, resolved.file);
}
#[test]
fn estimate_preserves_backend_client_differences() {
    let mut meta = parse_model_meta(Some("org/Qwen3-8B:Q4_K_S"));
    assert_eq!(meta.params_b, Some(8.));
    assert_eq!(meta.layers, Some(32.));
    assert_eq!(meta.bytes_per_param, Some(0.56));
    let device = LlamaDevice {
        id: "CUDA0".into(),
        free_mi_b: 15000.,
        total_mi_b: 16384.,
        ..Default::default()
    };
    let params = parse_params_from_script("llama-server --ctx-size 4096 --n-gpu-layers 16");
    let backend =
        build_estimate_response(meta.clone(), vec![device.clone()], params.clone(), "ctx");
    let client = build_breakdown(&meta, &params, std::slice::from_ref(&device));
    assert_eq!(backend.heuristic.weights_mi_b, client.weights_mi_b * 2.);
    assert_eq!(backend.heuristic.overhead_mi_b - client.overhead_mi_b, 128.);
    meta.weights_file_mi_b = Some(8000.);
    meta.mmproj_size_mi_b = Some(100.);
    meta.attention_layers = Some(8.);
    let mut params = params;
    params.cache_reuse = 2048.;
    params.spec_draft_max = 2.;
    let backend = build_estimate_response(
        meta.clone(),
        vec![device.clone()],
        params.clone(),
        "quality",
    );
    let client = build_breakdown(&meta, &params, &[device]);
    assert_eq!(backend.heuristic.weights_mi_b, 4000.);
    assert_eq!(client.weights_mi_b, 4000.);
    assert_eq!(backend.recommendation.cache_type_v, "q8_0");
    assert_eq!(
        client.total_mi_b,
        client.per_device_mi_b.iter().sum::<f64>()
    );
    assert_eq!(backend.heuristic.kv_mi_b, client.kv_mi_b);
    assert!(!build_breakdown(&ModelMeta::default(), &params, &[]).fits);
}
#[test]
fn writer_preserves_quoted_paths_and_unknown_flags() {
    let script = "'/bin/llama server' --model '/models/my model.gguf' -ngl 7 --fa off --draft 3 --unknown value --cache-type-k-draft f16";
    let mut params = parse_params_from_script(script);
    params.ngl = 25.;
    params.flash_attn = true;
    params.spec_draft_max = 4.;
    let enabled = parse_enabled_from_script(script);
    let result = apply_tuned_params(script, &params, &enabled);
    let parsed = script_parser::parse_script(&result).unwrap();
    assert_eq!(parsed.binary, "/bin/llama server");
    assert_eq!(parsed.model_file.as_deref(), Some("/models/my model.gguf"));
    assert_eq!(parsed.ngl, Some(25.));
    assert_eq!(parsed.spec_draft_n_max, Some(4.));
    assert!(parsed.flash_attn);
    assert!(!parsed.argv.contains(&"-ngl".into()));
    assert!(parsed.argv.contains(&"--unknown".into()));
    assert_eq!(
        script_parser::flag_value(&parsed.argv, "--cache-type-k-draft"),
        Some("f16")
    );
    let result = apply_tuned_params(&result, &params, &TunedFlagsEnabled::new());
    assert!(!result.contains("--n-gpu-layers"));
    assert!(!result.contains("--cache-type-k-draft"));
}
#[test]
fn formatting_catalog_and_store() {
    assert_eq!(fmt(Some(12345.25), 2), "12.345,25");
    assert_eq!(fmt_gb(Some(15872.), 1), "15,9");
    assert_eq!(fmt_ms(Some(61500.)), "01:02");
    assert_eq!(fmt(None, 2), "—");
    assert_eq!(effective_batch(None, None), (2048., 512.));
    let model = parse_model(Some("unsloth/Qwen3.6-35B-A3B-MTP-GGUF:Q4_K_S")).unwrap();
    assert_eq!(model.base, "Qwen3.6");
    assert_eq!(model.size.as_deref(), Some("35B-A3B"));
    assert!(model.mtp);
    assert!(is_model_moe(Some("35B-A3B")));
    assert_eq!(client::flags_catalog::flags_catalog().len(), 237);
    let mut first = BenchmarkResult {
        id: "a".into(),
        generation_tokens_per_second: Some(50.),
        ..Default::default()
    };
    first.config.model = Some("org/Qwen-7B:q4".into());
    let mut second = first.clone();
    second.id = "b".into();
    second.generation_tokens_per_second = Some(100.);
    let history = vec![first, second];
    assert_eq!(compare_text("alpha", "Beta"), std::cmp::Ordering::Less);
    assert_eq!(sort_history(&history, "unknown", false)[0].id, "a");
    assert_eq!(sort_history(&history, "totalVramMiB", true)[0].id, "a");
    assert_eq!(compute_bests(&history).g, 100.);
    assert_eq!(model_options(&history), vec!["Qwen-7B"]);
    assert_eq!(
        chart_results(&history, &["b".into(), "missing".into(), "a".into()])[0].id,
        "b"
    );
    assert_eq!(
        sort_history(&history, "generationTokensPerSecond", true)[0].id,
        "b"
    );
    assert_eq!(filter_history(&history, &["other".into()]).len(), 0);
}

#[test]
fn hf_cache_resolution_with_quant_and_projector() {
    if std::env::var_os("PLANE_HF_FIXTURE").is_some() {
        let resolved = resolve_model_file(Some("org/model:Q8_0"), None);
        assert!(
            resolved
                .file
                .as_deref()
                .unwrap()
                .ends_with("model-Q8_0.gguf")
        );
        assert!(
            resolved
                .mmproj_file
                .as_deref()
                .unwrap()
                .ends_with("mmproj.gguf")
        );
        assert_eq!(resolved.size_mi_b, Some(4. / 1048576.));
        assert!(
            resolve_model_file(Some("org/missing:Q4_0"), None)
                .file
                .is_none()
        );
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let snapshot = dir.path().join("hub/models--org--model/snapshots/abc");
    std::fs::create_dir_all(&snapshot).unwrap();
    for file in ["model-Q4_0.gguf", "model-Q8_0.gguf", "mmproj.gguf"] {
        std::fs::write(snapshot.join(file), b"GGUF").unwrap();
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "hf_cache_resolution_with_quant_and_projector"])
        .env("HF_HOME", dir.path())
        .env("PLANE_HF_FIXTURE", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
