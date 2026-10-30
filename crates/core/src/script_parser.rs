// Tokenización de scripts simples y extracción de flags, sin shell.
use crate::{CoreError, ParsedScript, Result};
pub fn tokenize_script(script: &str) -> Result<Vec<String>> {
    let joined = script
        .lines()
        .filter(|s| !s.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\\\n", " ");
    let mut chars = joined.chars().peekable();
    let mut tokens = Vec::new();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        let mut token = String::new();
        while let Some(c) = chars.peek().copied() {
            if c.is_whitespace() {
                break;
            }
            chars.next();
            if c == '\'' || c == '"' {
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == c {
                        closed = true;
                        break;
                    }
                    token.push(next);
                }
                if !closed {
                    return Err(CoreError::Invalid(format!(
                        "Comilla {c} sin cerrar en el script."
                    )));
                }
            } else {
                token.push(c);
            }
        }
        if !token.is_empty() {
            tokens.push(token);
        }
    }
    Ok(tokens)
}
pub fn flag_value<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.windows(2)
        .find(|p| p[0] == flag)
        .map(|p| p[1].as_str())
}
pub fn has_flag(argv: &[String], flag: &str, aliases: &[&str]) -> bool {
    argv.iter()
        .any(|a| a == flag || aliases.contains(&a.as_str()))
}
pub fn to_num_or_null(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| {
        if v.trim().is_empty() {
            Some(0.0)
        } else {
            v.trim().parse::<f64>().ok()
        }
    })
    .filter(|v| v.is_finite())
}
pub fn empty_parsed_script() -> ParsedScript {
    ParsedScript {
        host: "127.0.0.1".into(),
        port: 8080.0,
        ..Default::default()
    }
}
pub fn parse_script(script: &str) -> Result<ParsedScript> {
    let mut tokens = tokenize_script(script)?;
    if tokens.is_empty() {
        return Err(CoreError::Invalid(
            "El script está vacío: falta el binario de llama-server.".into(),
        ));
    }
    let binary = tokens.remove(0);
    let argv = tokens;
    let val = |flags: &[&str]| {
        flags
            .iter()
            .find_map(|f| flag_value(&argv, f))
            .map(str::to_owned)
    };
    let num = |flags: &[&str]| {
        flags
            .iter()
            .find_map(|f| to_num_or_null(flag_value(&argv, f)))
    };
    Ok(ParsedScript {
        script: script.into(),
        binary,
        model: val(&["-hf", "--hf-repo", "-hfr", "--model", "-m"]),
        host: val(&["--host"]).unwrap_or_else(|| "127.0.0.1".into()),
        port: num(&["--port"]).unwrap_or(8080.0),
        ctx_size: num(&["--ctx-size"]),
        batch_size: num(&["--batch-size"]),
        ubatch_size: num(&["--ubatch-size"]),
        cache_type_k: val(&["--cache-type-k"]),
        cache_type_v: val(&["--cache-type-v"]),
        device: val(&["--device"]),
        tensor_split: val(&["--tensor-split"]),
        temp: num(&["--temp"]),
        top_p: num(&["--top-p"]),
        top_k: num(&["--top-k"]),
        ngl: num(&["--n-gpu-layers", "-ngl"]),
        flash_attn: has_flag(&argv, "--flash-attn", &["-fa"]),
        threads: num(&["--threads", "-t"]),
        min_p: num(&["--min-p"]),
        repeat_penalty: num(&["--repeat-penalty"]),
        model_file: val(&["--model", "-m"]),
        n_cpu_moe: num(&["--n-cpu-moe", "--cpu-moe"]).unwrap_or(0.0),
        cache_reuse: num(&["--cache-reuse"]).unwrap_or(0.0),
        no_mmproj: has_flag(&argv, "--no-mmproj", &[]),
        spec_draft_n_max: num(&["--spec-draft-n-max"]),
        cache_ram: num(&["--cache-ram", "-cram"]),
        argv,
        ..Default::default()
    })
}
