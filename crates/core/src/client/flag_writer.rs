// Lectura y reescritura de parámetros sin alterar flags ajenos al optimizador.
use crate::*;
use std::collections::BTreeMap;
pub type TunedFlagsEnabled = BTreeMap<String, bool>;
const FLAGS: &[(&str, &str, &[&str])] = &[
    ("ctxSize", "--ctx-size", &[]),
    ("ngl", "--n-gpu-layers", &["-ngl", "--ngl"]),
    ("cacheTypeK", "--cache-type-k", &[]),
    ("cacheTypeV", "--cache-type-v", &[]),
    ("cacheTypeKDraft", "--cache-type-k-draft", &[]),
    ("cacheTypeVDraft", "--cache-type-v-draft", &[]),
    ("batchSize", "--batch-size", &[]),
    ("ubatchSize", "--ubatch-size", &[]),
    ("device", "--device", &[]),
    ("nCpuMoe", "--n-cpu-moe", &["--cpu-moe"]),
    ("cacheReuse", "--cache-reuse", &[]),
    (
        "specDraftMax",
        "--spec-draft-n-max",
        &["--draft-max", "--draft", "--draft-n"],
    ),
    ("cacheRam", "--cache-ram", &["-cram", "--cram"]),
];
pub fn params_from_parsed(p: &ParsedScript) -> TunedParams {
    TunedParams {
        ctx_size: p.ctx_size.unwrap_or(8192.),
        ngl: p.ngl.unwrap_or(999.),
        cache_type_k: p
            .cache_type_k
            .clone()
            .unwrap_or("q8_0".into())
            .to_lowercase(),
        cache_type_v: p
            .cache_type_v
            .clone()
            .unwrap_or("q8_0".into())
            .to_lowercase(),
        batch_size: p.batch_size.unwrap_or(512.),
        ubatch_size: p.ubatch_size.unwrap_or(128.),
        flash_attn: p.flash_attn,
        device: p
            .device
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        tensor_split: p
            .tensor_split
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .filter(|n| n.is_finite() && *n > 0.)
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
        n_cpu_moe: p.n_cpu_moe,
        cache_reuse: p.cache_reuse,
        no_mmproj: p.no_mmproj,
        spec_draft_max: p.spec_draft_n_max.unwrap_or(0.),
        cache_ram: p.cache_ram.unwrap_or(8192.),
        ..Default::default()
    }
}
pub fn parse_params_from_script(script: &str) -> TunedParams {
    let tokens = script_parser::tokenize_script(script).unwrap_or_default();
    let value = |forms: &[&str]| {
        tokens
            .windows(2)
            .find(|pair| forms.contains(&pair[0].as_str()))
            .map(|pair| pair[1].as_str())
    };
    let number = |forms: &[&str], default: f64| {
        script_parser::to_num_or_null(value(forms)).unwrap_or(default)
    };
    let switch = |forms: &[&str]| {
        tokens
            .iter()
            .position(|s| forms.contains(&s.as_str()))
            .is_some_and(|i| tokens.get(i + 1).is_none_or(|s| s != "off"))
    };
    let mut params = TunedParams {
        ctx_size: number(&["--ctx-size"], 8192.),
        ngl: number(&["--n-gpu-layers", "-ngl", "--ngl"], 999.),
        cache_type_k: value(&["--cache-type-k"]).unwrap_or("q8_0").to_lowercase(),
        cache_type_v: value(&["--cache-type-v"]).unwrap_or("q8_0").to_lowercase(),
        batch_size: number(&["--batch-size"], 512.),
        ubatch_size: number(&["--ubatch-size"], 128.),
        flash_attn: switch(&["--flash-attn", "-fa", "--fa"]),
        device: value(&["--device"])
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        tensor_split: value(&["--tensor-split"])
            .map(|s| {
                s.split(',')
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .filter(|n| n.is_finite() && *n > 0.)
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
        n_cpu_moe: number(&["--n-cpu-moe", "--cpu-moe"], 0.),
        cache_reuse: number(&["--cache-reuse"], 0.),
        no_mmproj: switch(&["--no-mmproj"]),
        spec_draft_max: number(
            &["--spec-draft-n-max", "--draft-max", "--draft", "--draft-n"],
            0.,
        ),
        cache_ram: number(&["--cache-ram", "-cram", "--cram"], 8192.),
        ..Default::default()
    };
    for (key, flag) in [
        ("cacheTypeKDraft", "--cache-type-k-draft"),
        ("cacheTypeVDraft", "--cache-type-v-draft"),
    ] {
        params.extra.insert(
            key.into(),
            serde_json::json!(value(&[flag]).unwrap_or("q8_0").to_lowercase()),
        );
    }
    params
}
pub fn parse_enabled_from_script(script: &str) -> TunedFlagsEnabled {
    let tokens = script_parser::tokenize_script(script).unwrap_or_default();
    FLAGS
        .iter()
        .map(|(key, flag, aliases)| {
            (
                (*key).into(),
                std::iter::once(*flag)
                    .chain(aliases.iter().copied())
                    .any(|f| script_parser::flag_value(&tokens, f).is_some()),
            )
        })
        .collect()
}
fn remove(tokens: &mut Vec<String>, flag: &str) {
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == flag || tokens[i].starts_with(&format!("{flag}=")) {
            let count =
                if tokens[i] == flag && tokens.get(i + 1).is_some_and(|s| !s.starts_with('-')) {
                    2
                } else {
                    1
                };
            tokens.drain(i..i + count);
        } else {
            i += 1;
        }
    }
}
fn quote(s: &str) -> String {
    if s.chars().any(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.into()
    }
}
pub fn render_tokens(tokens: &[String]) -> String {
    let mut parts = vec![];
    for (i, token) in tokens.iter().enumerate() {
        if i == 0 || token.starts_with('-') {
            parts.push(quote(token));
        } else if let Some(last) = parts.last_mut() {
            last.push(' ');
            last.push_str(&quote(token));
        }
    }
    parts.join(" \\\n")
}
pub fn apply_tuned_params(
    script: &str,
    params: &TunedParams,
    enabled: &TunedFlagsEnabled,
) -> String {
    let mut tokens = script_parser::tokenize_script(script).unwrap_or_default();
    let values = serde_json::to_value(params).unwrap_or_default();
    for (key, flag, aliases) in FLAGS {
        remove(&mut tokens, flag);
        for alias in *aliases {
            remove(&mut tokens, alias);
        }
        if enabled.get(*key).copied().unwrap_or(false) {
            let value = &values[*key];
            let text = if let Some(arr) = value.as_array() {
                arr.iter()
                    .map(|v| {
                        v.as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| v.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string())
            };
            if !text.is_empty() && text != "null" {
                tokens.extend([(*flag).into(), text]);
            }
        }
    }
    for flag in [
        "--flash-attn",
        "-fa",
        "--fa",
        "--no-mmproj",
        "--tensor-split",
    ] {
        remove(&mut tokens, flag);
    }
    if params.flash_attn {
        tokens.extend(["--flash-attn".into(), "on".into()]);
    }
    if params.no_mmproj {
        tokens.push("--no-mmproj".into());
    }
    if let Some(split) = params.tensor_split.as_ref().filter(|s| !s.is_empty()) {
        tokens.extend([
            "--tensor-split".into(),
            split
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
        ]);
    }
    render_tokens(&tokens)
}
