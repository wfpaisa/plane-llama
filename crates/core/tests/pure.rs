// Pruebas de contratos, parsers y persistencia sin GPU.
use plane_llama_core::{
    devices::*, gpu::*, history::History, logs::Logs, mem::*, metrics::*, script_parser::*, *,
};
#[test]
fn scripts() {
    let p = parse_script("# comentario\n'/tmp/llama server' \\\n --model \"a b.gguf\" --port 9000 -fa off --ctx-size nope -t 4 #literal").unwrap();
    assert_eq!(p.binary, "/tmp/llama server");
    assert_eq!(p.model.as_deref(), Some("a b.gguf"));
    assert_eq!(p.port, 9000.0);
    assert!(p.flash_attn);
    assert_eq!(p.ctx_size, None);
    assert_eq!(p.threads, Some(4.0));
    assert_eq!(p.argv.last().unwrap(), "#literal");
    assert!(parse_script("  # nada").is_err());
    assert!(tokenize_script("a 'b").is_err());
    assert_eq!(tokenize_script("a '' x\"y z\"").unwrap(), ["a", "xy z"]);
    assert_eq!(to_num_or_null(Some("Infinity")), None);
}
#[test]
fn hardware() {
    let d = parse_list_devices(include_str!("fixtures/devices.txt"));
    assert_eq!(d.len(), 2);
    assert_eq!(detect_backend(&d), "vulkan");
    let mut f = d.clone();
    f[0].free_mi_b -= 100.0;
    assert_eq!(
        compute_device_vram(&d, &f, Some("Vulkan0"))[0].used_mi_b,
        Some(100.0)
    );
    let gpu = parse_nvidia_csv(include_str!("fixtures/nvidia.csv"));
    assert_eq!(gpu[1].mem_used_mi_b, None);
    assert_eq!(
        subtract_gpu_baseline(&gpu, &gpu)[0].mem_used_mi_b,
        Some(0.0)
    );
    let ram = parse_meminfo("MemTotal: 4096 kB\nMemAvailable: 1024 kB\n");
    assert_eq!(ram.mem_used_mi_b, Some(3.0));
    assert_eq!(subtract_ram_baseline(&ram, &ram), Some(0.0));
    assert!(
        parse_meminfo(include_str!("fixtures/meminfo.txt"))
            .mem_total_mi_b
            .is_some()
    );
}
fn lines(raw: &str) -> Vec<LogEntry> {
    raw.lines()
        .enumerate()
        .map(|(i, s)| LogEntry {
            t: i as f64 * 1000.0,
            msg: s.into(),
            ..Default::default()
        })
        .collect()
}
#[test]
fn metrics_latest_drafts_and_fallback() {
    let raw = "loading model\nmodel loaded\nprompt eval time = 100 ms / 20 tokens (200 tokens per second)\neval time = 200 ms / 10 tokens (50 tokens per second)\ndraft acceptance = 0.8 ( 8 accepted / 10 generated)";
    let m = parse_metrics_from_logs(&lines(raw));
    assert_eq!(m.load_time_seconds, Some(1.0));
    assert_eq!(m.prompt_token_count, Some(20.0));
    assert_eq!(m.generation_time_ms, Some(200.0));
    assert_eq!(m.generation_tokens_per_second, Some(50.0));
    assert_eq!(m.acc_drafts, Some(8.0));
    let m = parse_metrics_from_logs(&lines(
        "model loaded 2500 ms\ndraft-mtp: #gen drafts = 12 #acc drafts = 9",
    ));
    assert_eq!(m.load_time_seconds, Some(2.5));
    assert_eq!(m.gen_drafts, Some(12.0));
    let real = parse_metrics_from_logs(&lines(include_str!("fixtures/logs.txt")));
    assert!(real.generation_tokens_per_second.is_some());
}
#[test]
fn log_cursor_survives_trim_and_clear() {
    let logs = Logs::default();
    let mut rx = logs.events.subscribe();
    logs.push("stdout", "uno\n");
    assert!(rx.try_recv().is_ok());
    for _ in 0..5001 {
        logs.push("stderr", "dos");
    }
    assert_eq!(logs.read(None).entries.len(), 5000);
    let cursor = logs.read(None).cursor;
    logs.clear();
    assert_eq!(logs.read(None).cursor, cursor);
    logs.push("stdout", "tres");
    assert_eq!(logs.read(Some(cursor as u64)).entries.len(), 1);
}
#[test]
fn history_atomic_cap_patch_and_compatibility() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = Config {
        data_dir: dir.path().into(),
    };
    let mut h = History::new(cfg.clone()).unwrap();
    for i in 0..202 {
        h.save(BenchmarkResult {
            id: i.to_string(),
            config: empty_parsed_script(),
            ..Default::default()
        })
        .unwrap();
    }
    assert_eq!(h.all().len(), 200);
    let patch: ResultPatch = serde_json::from_str(r#"{"rating":null,"favorite":true}"#).unwrap();
    let r = h.patch("201", patch).unwrap();
    assert_eq!(
        serde_json::to_value(r).unwrap()["rating"],
        serde_json::Value::Null
    );
    assert!(
        h.patch(
            "201",
            ResultPatch {
                rating: Some(Some(11.0)),
                ..Default::default()
            }
        )
        .is_err()
    );
    h.delete(&["201".into(), "200".into()]).unwrap();
    assert_eq!(History::new(cfg.clone()).unwrap().all().len(), 198);
    assert!(!cfg.file("history.json.tmp").exists());
    h.clear().unwrap();
    assert!(h.all().is_empty());
}
#[test]
fn synthetic_history_roundtrip() {
    let mut rows = vec![];
    for i in 0..3 {
        let mut row = BenchmarkResult {
            id: i.to_string(),
            config: parse_script("llama-server -hf org/Model:Q4_K_M --ctx-size 8192").unwrap(),
            ..Default::default()
        };
        row.rating = if i == 0 {
            None
        } else {
            Some(if i == 1 { None } else { Some(8.0) })
        };
        row.favorite = Some(true);
        row.extra
            .insert("legacyField".into(), serde_json::json!(42));
        rows.push(row);
    }
    let value = serde_json::to_value(rows).unwrap();
    let parsed: Vec<BenchmarkResult> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}
#[test]
#[ignore = "Requiere el historial local del referente (solo lectura)"]
fn real_history_roundtrip() {
    let value: serde_json::Value = serde_json::from_slice(
        &std::fs::read("/home/projects/plane-llama-bench/data/history.json").unwrap(),
    )
    .unwrap();
    let parsed: Vec<BenchmarkResult> = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}
