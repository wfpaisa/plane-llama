// Cálculo instantáneo del cliente; conserva las diferencias del referente TS.
use crate::{
    optimizer::{kv_mib, selected_devices, split_per_device},
    *,
};
pub const KV_TYPES: &[&str] = &["f16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0", "iq4_nl"];
pub fn total_free_for(devices: &[LlamaDevice], ids: &[String]) -> f64 {
    selected_devices(devices, ids)
        .iter()
        .map(|d| d.free_mi_b)
        .sum()
}
pub fn build_breakdown(
    meta: &ModelMeta,
    params: &TunedParams,
    devices: &[LlamaDevice],
) -> VramBreakdown {
    let selected = selected_devices(devices, &params.device);
    let Some(weights) = meta
        .weights_file_mi_b
        .or_else(|| Some(meta.params_b? * 1e9 * meta.bytes_per_param? / 1048576.))
    else {
        return VramBreakdown {
            per_device_mi_b: vec![0.; selected.len()],
            ..Default::default()
        };
    };
    let layers = meta.layers.unwrap_or(32.);
    let weights = weights
        * if params.ngl >= layers {
            1.
        } else {
            (params.ngl / layers).max(0.)
        };
    let kv = kv_mib(meta, params, (params.ctx_size - params.cache_reuse).max(0.));
    let mut overhead = 128. + params.ubatch_size * 0.5;
    if !params.no_mmproj {
        overhead += meta.mmproj_size_mi_b.unwrap_or(0.);
    }
    if params.spec_draft_max > 0. {
        overhead += weights * (0.08 + 0.035 * (params.spec_draft_max - 1.));
    }
    let per_device = split_per_device(
        weights + kv + overhead,
        &selected,
        params.tensor_split.as_deref(),
    );
    let total = per_device.iter().sum();
    VramBreakdown {
        fits: total <= total_free_for(devices, &params.device),
        total_mi_b: total,
        weights_mi_b: weights,
        kv_mi_b: kv,
        overhead_mi_b: overhead,
        per_device_mi_b: per_device,
        ..Default::default()
    }
}
