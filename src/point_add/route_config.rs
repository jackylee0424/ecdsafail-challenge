use std::ops::RangeInclusive;

pub(crate) fn env_flag(name: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some("1")
}

pub(crate) fn env_flag_default_on(name: &str) -> bool {
    std::env::var(name).ok().as_deref() != Some("0")
}

pub(crate) fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
}

pub(crate) fn env_nonzero_usize(name: &str) -> Option<usize> {
    env_usize(name).filter(|&value| value > 0)
}

pub(crate) fn env_usize_in(name: &str, range: RangeInclusive<usize>) -> Option<usize> {
    env_usize(name).filter(|value| range.contains(value))
}

pub(crate) fn env_usize_or(name: &str, default: usize) -> usize {
    env_usize(name).unwrap_or(default)
}

pub(crate) fn env_f64_guarded(name: &str, default: f64, valid: impl FnOnce(f64) -> bool) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|&value| value.is_finite() && valid(value))
        .unwrap_or(default)
}

pub(crate) fn env_usize_csv(name: &str) -> Vec<usize> {
    std::env::var(name)
        .ok()
        .map(|value| parse_usize_csv(&value))
        .unwrap_or_default()
}

pub(crate) fn env_trim_list(name: &str) -> Option<Vec<usize>> {
    std::env::var(name)
        .ok()
        .and_then(|value| parse_trim_list(&value))
}

pub(crate) fn env_step_map(name: &str) -> Vec<(usize, usize)> {
    std::env::var(name)
        .ok()
        .map(|value| parse_step_map(&value))
        .unwrap_or_default()
}

pub(crate) fn env_step_map_value(name: &str, step: usize) -> usize {
    std::env::var(name)
        .ok()
        .map(|value| step_map_value(&parse_step_map(&value), step))
        .unwrap_or_default()
}

pub(crate) fn env_step_map_override(name: &str, step: usize) -> Option<usize> {
    let map = std::env::var(name).ok()?;
    parse_step_map(&map)
        .into_iter()
        .rev()
        .find_map(|(mapped_step, value)| (mapped_step == step).then_some(value))
}

pub(crate) fn parse_usize_csv(value: &str) -> Vec<usize> {
    value
        .split(',')
        .filter_map(|item| item.trim().parse::<usize>().ok())
        .collect()
}

pub(crate) fn parse_trim_list(value: &str) -> Option<Vec<usize>> {
    if value.trim().is_empty() {
        return None;
    }
    let trims = parse_usize_csv(value);
    (!trims.is_empty()).then_some(trims)
}

pub(crate) fn parse_step_map(value: &str) -> Vec<(usize, usize)> {
    value
        .split(',')
        .filter_map(|entry| {
            let (step, mapped_value) = entry.trim().split_once(':')?;
            Some((
                step.trim().parse::<usize>().ok()?,
                mapped_value.trim().parse::<usize>().ok()?,
            ))
        })
        .collect()
}

pub(crate) fn step_map_value(map: &[(usize, usize)], step: usize) -> usize {
    map.iter()
        .filter_map(|&(mapped_step, value)| (mapped_step == step).then_some(value))
        .sum()
}

pub(crate) fn step_map_override(map: &[(usize, usize)], step: usize) -> Option<usize> {
    map.iter()
        .rev()
        .find_map(|&(mapped_step, value)| (mapped_step == step).then_some(value))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DialogGcdCoreRouteEnv {
    pub(crate) active_iterations: usize,
    pub(crate) compare_bits: usize,
    pub(crate) apply_clean_compare_bits: usize,
    pub(crate) width_margin: f64,
    pub(crate) width_slope: f64,
    pub(crate) body_carry_trims: Option<Vec<usize>>,
    pub(crate) pa9024_compare_schedule: bool,
    pub(crate) pa9024_compare_margin: usize,
    pub(crate) pa9024_compare_floor: usize,
    pub(crate) compare_step_bits: Vec<(usize, usize)>,
    pub(crate) odd_u_lowbit_fastpath: bool,
    pub(crate) k2: bool,
    pub(crate) variable_width: bool,
    pub(crate) raw_tobitvector_materialized_sub: bool,
    pub(crate) tobitvector_cswap_body_trim: bool,
    pub(crate) tobitvector_shift_body_trim: bool,
    pub(crate) skip_zero_edge_tobit_fwd_cshift: bool,
    pub(crate) width_step_bumps: Vec<(usize, usize)>,
    pub(crate) body_step_givebacks: Vec<(usize, usize)>,
    pub(crate) k2_force0: bool,
    pub(crate) strict_compare: bool,
    pub(crate) body_carry_trunc_w: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DialogGcdCoreRouteEnvNames {
    pub(crate) active_iterations: &'static str,
    pub(crate) compare_bits: &'static str,
    pub(crate) apply_clean_compare_bits: &'static str,
    pub(crate) pa9024_compare_schedule: &'static str,
    pub(crate) pa9024_compare_schedule_floor: &'static str,
}

impl DialogGcdCoreRouteEnvNames {
    pub(crate) const DEFAULT: Self = Self {
        active_iterations: "DIALOG_GCD_ACTIVE_ITERATIONS",
        compare_bits: "DIALOG_GCD_COMPARE_BITS",
        apply_clean_compare_bits: "DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS",
        pa9024_compare_schedule: "DIALOG_GCD_PA9024_COMPARE_SCHEDULE",
        pa9024_compare_schedule_floor: "DIALOG_GCD_PA9024_COMPARE_SCHEDULE_FLOOR",
    };
}

impl DialogGcdCoreRouteEnv {
    pub(crate) fn from_env(
        max_iterations: usize,
        n: usize,
        default_compare_bits: usize,
        default_width_margin: f64,
        default_width_slope: f64,
    ) -> Self {
        Self::from_env_names(
            DialogGcdCoreRouteEnvNames::DEFAULT,
            max_iterations,
            n,
            default_compare_bits,
            default_width_margin,
            default_width_slope,
        )
    }

    pub(crate) fn from_env_names(
        names: DialogGcdCoreRouteEnvNames,
        max_iterations: usize,
        n: usize,
        default_compare_bits: usize,
        default_width_margin: f64,
        default_width_slope: f64,
    ) -> Self {
        let compare_bits = env_usize_in(names.compare_bits, 1..=n).unwrap_or(default_compare_bits);

        Self {
            active_iterations: env_usize_in(names.active_iterations, 1..=max_iterations)
                .unwrap_or(max_iterations),
            compare_bits,
            apply_clean_compare_bits: env_usize_in(names.apply_clean_compare_bits, 1..=n)
                .unwrap_or(compare_bits),
            width_margin: env_f64_guarded(
                "DIALOG_GCD_WIDTH_MARGIN",
                default_width_margin,
                |margin| margin >= 0.0 && margin <= n as f64,
            ),
            width_slope: env_f64_guarded(
                "DIALOG_GCD_WIDTH_SLOPE_X1000",
                default_width_slope * 1000.0,
                |slope| slope > 0.0 && slope <= 4000.0,
            ) / 1000.0,
            body_carry_trims: env_trim_list("DIALOG_GCD_BODY_CARRY_BAND_TRIMS"),
            pa9024_compare_schedule: env_flag(names.pa9024_compare_schedule),
            pa9024_compare_margin: env_usize_or("DIALOG_GCD_PA9024_COMPARE_SCHEDULE_MARGIN", 0),
            pa9024_compare_floor: env_usize_in(names.pa9024_compare_schedule_floor, 0..=n)
                .unwrap_or(1)
                .max(1),
            compare_step_bits: env_step_map("DIALOG_GCD_COMPARE_STEP_BITS"),
            odd_u_lowbit_fastpath: env_flag("DIALOG_GCD_ODD_U_LOWBIT_FASTPATH"),
            k2: env_flag("DIALOG_GCD_K2"),
            variable_width: env_flag_default_on("DIALOG_GCD_RAW_TOBITVECTOR_VARIABLE_WIDTH"),
            raw_tobitvector_materialized_sub: env_flag_default_on(
                "DIALOG_GCD_RAW_TOBITVECTOR_MATERIALIZED_SUB",
            ),
            tobitvector_cswap_body_trim: env_flag("DIALOG_GCD_TOBITVECTOR_CSWAP_BODY_TRIM"),
            tobitvector_shift_body_trim: env_flag("DIALOG_GCD_TOBITVECTOR_SHIFT_BODY_TRIM"),
            skip_zero_edge_tobit_fwd_cshift: env_flag("DIALOG_GCD_SKIP_ZERO_EDGE_CSHIFT")
                || env_flag("DIALOG_GCD_SKIP_ZERO_EDGE_TOBIT_CSHIFT")
                || env_flag("DIALOG_GCD_SKIP_ZERO_EDGE_TOBIT_FWD_CSHIFT"),
            width_step_bumps: env_step_map("DIALOG_GCD_WIDTH_STEP_BUMPS"),
            body_step_givebacks: env_step_map("DIALOG_GCD_BODY_STEP_GIVEBACKS"),
            k2_force0: env_flag("DIALOG_GCD_K2_FORCE0"),
            strict_compare: env_flag("DIALOG_GCD_FILTER_STRICT_COMPARE"),
            body_carry_trunc_w: env_usize_or("DIALOG_GCD_BODY_CARRY_TRUNC_W", 0),
        }
    }
}
