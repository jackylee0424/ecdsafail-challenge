#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnvVar {
    pub(crate) name: &'static str,
    pub(crate) value: &'static str,
}

impl EnvVar {
    const fn new(name: &'static str, value: &'static str) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvMutation {
    Set(&'static str, &'static str),
    Remove(&'static str),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Add,
    Sub,
    Fold,
    Double,
    Halve,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstantForm {
    SparseC,
    SignedSparseC,
    Generic,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlModel {
    Materialized,
    DirectControlled,
    ControlByPrep,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryModel {
    Full,
    TruncatedWindow,
    GuardedTail,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CleanupModel {
    Coherent,
    MeasuredHmr,
    PhaseCorrected,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostModel {
    Fresh,
    Borrowed,
    Streamed,
    DerivedControlHosted,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AxisKind {
    ResourceOnly,
    SeedOnly,
    ProvableExact,
    LossyIsland,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SolinasCuccaroAdderKey {
    operation: Operation,
    constant_form: ConstantForm,
    control_model: ControlModel,
    carry_model: CarryModel,
    cleanup_model: CleanupModel,
    host_model: HostModel,
}

impl SolinasCuccaroAdderKey {
    const fn new(
        operation: Operation,
        constant_form: ConstantForm,
        control_model: ControlModel,
        carry_model: CarryModel,
        cleanup_model: CleanupModel,
        host_model: HostModel,
    ) -> Self {
        Self {
            operation,
            constant_form,
            control_model,
            carry_model,
            cleanup_model,
            host_model,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProofObligations {
    mdd_node: &'static str,
    tla_model: &'static str,
    z3_query: &'static str,
    lean_theorem: &'static str,
}

impl ProofObligations {
    const fn new(
        mdd_node: &'static str,
        tla_model: &'static str,
        z3_query: &'static str,
        lean_theorem: &'static str,
    ) -> Self {
        Self {
            mdd_node,
            tla_model,
            z3_query,
            lean_theorem,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SolinasCuccaroAdderSite {
    point_add_phase: &'static str,
    key: SolinasCuccaroAdderKey,
    axis_kind: AxisKind,
    proof: ProofObligations,
    classification_note: &'static str,
}

impl SolinasCuccaroAdderSite {
    const fn new(
        point_add_phase: &'static str,
        operation: Operation,
        constant_form: ConstantForm,
        control_model: ControlModel,
        carry_model: CarryModel,
        cleanup_model: CleanupModel,
        host_model: HostModel,
        axis_kind: AxisKind,
        mdd_node: &'static str,
        tla_model: &'static str,
        z3_query: &'static str,
        lean_theorem: &'static str,
        classification_note: &'static str,
    ) -> Self {
        Self {
            point_add_phase,
            key: SolinasCuccaroAdderKey::new(
                operation,
                constant_form,
                control_model,
                carry_model,
                cleanup_model,
                host_model,
            ),
            axis_kind,
            proof: ProofObligations::new(mdd_node, tla_model, z3_query, lean_theorem),
            classification_note,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PointAddRoutePreset {
    defaults: &'static [EnvVar],
    submission_pins: &'static [EnvMutation],
    adder_sites: &'static [SolinasCuccaroAdderSite],
}

impl PointAddRoutePreset {
    pub(crate) const fn accepted_cf310ec() -> Self {
        Self {
            defaults: ACCEPTED_CF310EC_DEFAULTS,
            submission_pins: ACCEPTED_CF310EC_SUBMISSION_PINS,
            adder_sites: ACCEPTED_CF310EC_ADDER_SITES,
        }
    }

    pub(crate) fn apply_for_submission(self) {
        self.apply_defaults();
        self.apply_submission_pins();
    }

    pub(crate) fn apply_defaults(self) {
        for env in self.defaults {
            if std::env::var_os(env.name).is_none() {
                std::env::set_var(env.name, env.value);
            }
        }
    }

    pub(crate) fn apply_submission_pins(self) {
        for pin in self.submission_pins {
            apply_env_mutation(*pin);
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn solinas_cuccaro_adder_sites(self) -> &'static [SolinasCuccaroAdderSite] {
        self.adder_sites
    }
}

fn apply_env_mutation(pin: EnvMutation) {
    match pin {
        EnvMutation::Set(name, value) => std::env::set_var(name, value),
        EnvMutation::Remove(name) => std::env::remove_var(name),
    }
}

fn apply_overlay(overlay: &[EnvMutation]) {
    PointAddRoutePreset::accepted_cf310ec().apply_for_submission();
    for pin in overlay {
        apply_env_mutation(*pin);
    }
}

pub(crate) fn apply_q1175_dirty_qoffset_core_experiment() {
    apply_overlay(Q1175_DIRTY_QOFFSET_CORE_OVERLAY);
}

pub(crate) fn apply_q1175_dirty_qoffset_first_experiment() {
    apply_overlay(Q1175_DIRTY_QOFFSET_FIRST_OVERLAY);
}

pub(crate) fn apply_q1175_boundary_borrow_experiment() {
    apply_overlay(Q1175_BOUNDARY_BORROW_OVERLAY);
}

pub(crate) fn apply_q1175_apply_chunk_shape_experiment() {
    apply_overlay(Q1175_APPLY_CHUNK_SHAPE_OVERLAY);
}

pub(crate) fn apply_q1175_repaired_clean_91794252_experiment() {
    apply_overlay(Q1175_REPAIRED_CLEAN_91794252_OVERLAY);
}

#[rustfmt::skip]
const ACCEPTED_CF310EC_ADDER_SITES: &[SolinasCuccaroAdderSite] = &[
    SolinasCuccaroAdderSite::new(
        "dialog_apply_special_fold",
        Operation::Fold,
        ConstantForm::SparseC,
        ControlModel::Materialized,
        CarryModel::GuardedTail,
        CleanupModel::PhaseCorrected,
        HostModel::Borrowed,
        AxisKind::LossyIsland,
        "dev/solinas_cuccaro_adder_key.mmd#dialog_apply_special_fold",
        "dev/formal/PointAddSolinasCuccaroAdder.tla#dialog_apply_special_fold",
        "dev/formal/solinas_cuccaro_adder_bv.smt2#guarded_truncated_sparse_add",
        "PointAdd.Formal.SolinasCuccaro.apply_special_fold_action",
        "materialized/direct sparse-c apply fold; step clean bits and known-zero controls are hazards",
    ),
    SolinasCuccaroAdderSite::new(
        "kal_double",
        Operation::Double,
        ConstantForm::SparseC,
        ControlModel::DirectControlled,
        CarryModel::TruncatedWindow,
        CleanupModel::MeasuredHmr,
        HostModel::Fresh,
        AxisKind::LossyIsland,
        "dev/solinas_cuccaro_adder_key.mmd#kal_double",
        "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_double",
        "dev/formal/solinas_cuccaro_adder_bv.smt2#double_sparse_c_action",
        "PointAdd.Formal.SolinasCuccaro.sparse_c_double_mod_p",
        "cadd sparse-c double path; carry truncation comes from KAL_DOUBLE_CARRY_TRUNC_W",
    ),
    SolinasCuccaroAdderSite::new(
        "kal_fold",
        Operation::Fold,
        ConstantForm::SparseC,
        ControlModel::DirectControlled,
        CarryModel::TruncatedWindow,
        CleanupModel::MeasuredHmr,
        HostModel::DerivedControlHosted,
        AxisKind::LossyIsland,
        "dev/solinas_cuccaro_adder_key.mmd#kal_fold",
        "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_fold",
        "dev/formal/solinas_cuccaro_adder_bv.smt2#guarded_truncated_sparse_add",
        "PointAdd.Formal.SolinasCuccaro.signed_sparse_c_fold",
        "fused double_y/halve_y fold; hosted and streamed controls are lifecycle obligations",
    ),
    SolinasCuccaroAdderSite::new(
        "kal_halve",
        Operation::Halve,
        ConstantForm::SparseC,
        ControlModel::DirectControlled,
        CarryModel::TruncatedWindow,
        CleanupModel::MeasuredHmr,
        HostModel::Fresh,
        AxisKind::LossyIsland,
        "dev/solinas_cuccaro_adder_key.mmd#kal_halve",
        "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_halve",
        "dev/formal/solinas_cuccaro_adder_bv.smt2#halve_sparse_c_action",
        "PointAdd.Formal.SolinasCuccaro.sparse_c_halve_double_inverse",
        "inverse sparse-c halve path; shares KAL_DOUBLE_CARRY_TRUNC_W with double",
    ),
    SolinasCuccaroAdderSite::new(
        "round84_quotient_fold",
        Operation::Fold,
        ConstantForm::SignedSparseC,
        ControlModel::ControlByPrep,
        CarryModel::TruncatedWindow,
        CleanupModel::PhaseCorrected,
        HostModel::Streamed,
        AxisKind::LossyIsland,
        "dev/solinas_cuccaro_adder_key.mmd#round84_quotient_fold",
        "dev/formal/PointAddSolinasCuccaroAdder.tla#round84_quotient_fold",
        "dev/formal/solinas_cuccaro_adder_bv.smt2#control_by_prep_scratch",
        "PointAdd.Formal.SolinasCuccaro.round84_quotient_sparse_c",
        "round84 quotient*c fold; signed sparse form follows R84_QPROD_NAF",
    ),
];

#[rustfmt::skip]
const ACCEPTED_CF310EC_DEFAULTS: &[EnvVar] = &[
    EnvVar::new("SKIP_ALT_SEED_CHECKS", "1"),
    EnvVar::new("DIALOG_GCD_COMPRESSED_SIDECAR_LOG", "1"),
    EnvVar::new("SQUARE_ROW_WINDOW_CLEAN_COMPARE_BITS", "21"),
    EnvVar::new("SQUARE_ROW_WINDOW_MEASURED_CARRY_CLEAR", "1"),
    EnvVar::new("ROUND84_KEEP_QUOTIENT_PRODUCT", "1"),
    EnvVar::new("DIALOG_GCD_FOLD_CARRY_TRUNC_W", "17"),
    EnvVar::new("DIALOG_TAIL_NONCE", "9600076011007"),
    EnvVar::new("DIALOG_GCD_SKIP_ZERO_EDGE_CSHIFT", "1"),
    EnvVar::new("DIALOG_GCD_COMPRESSED_BLOCK_LIFECYCLE", "1"),
    EnvVar::new("DIALOG_GCD_HOST_REVERSE_RAW_BLOCK", "1"),
    EnvVar::new("DIALOG_GCD_COMPRESSED_LOG_U_HIGH_RUNWAY", "1"),
    EnvVar::new("DIALOG_GCD_COMPRESSED_LOG_U_HIGH_RUNWAY_BLOCKS", "999"),
    EnvVar::new("DIALOG_GCD_COMPOSITE_SCRATCH", "1"),
    EnvVar::new("DIALOG_GCD_BORROW_CURRENT_BLOCK", "1"),
    EnvVar::new("DIALOG_GCD_CTRL_BODY_VENTED", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_REPLAY_SWAP_HOST", "1"),
    EnvVar::new("SQUARE_SELFHOST_SAFE_LANE_REUSE", "1"),
    EnvVar::new("SQUARE_SELFHOST_GATE_SUFFIX_CARRIES", "0"),
    EnvVar::new("DIALOG_GCD_PA9024_COMPARE_SCHEDULE", "1"),
    EnvVar::new("DIALOG_GCD_PA9024_COMPARE_SCHEDULE_MARGIN", "0"),
    EnvVar::new("KAL_DOUBLE_CARRY_TRUNC_W", "19"),
    EnvVar::new("KAL_FOLD_CARRY_TRUNC_W", "18"),
    EnvVar::new("DIALOG_GCD_ROUND763_DEDUP", "1"),
    EnvVar::new("DIALOG_GCD_ROUND763_COMPRESS_LEVER", "1"),
    EnvVar::new("DIALOG_GCD_MEASURED_UNDERFLOW_GATE", "1"),
    EnvVar::new("DIALOG_GCD_COMPARE_BITS", "46"),
    EnvVar::new("DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS", "19"),
    EnvVar::new("DIALOG_GCD_APPLY_BOUNDARY_CONDITIONAL_REPLAY", "1"),
    EnvVar::new("DIALOG_GCD_SELECTED_BODY_STREAM_SUFFIX_MAP", "3:2,4:3,5:5,6:6,7:7,8:5,9:7,10:5,11:7,12:6,13:7,14:5,15:6,16:3,17:5,18:1,19:3,21:1"),
    EnvVar::new("DIALOG_GCD_REVERSE_BRANCH_CONDITIONAL_REPLAY", "1"),
    EnvVar::new("DIALOG_GCD_SPECIAL_CLEAN_CONDITIONAL_REPLAY", "1"),
    EnvVar::new("MOD_FAST_FLAG_CONDITIONAL_REPLAY", "1"),
    EnvVar::new("DIALOG_GCD_RAW_PA", "1"),
    EnvVar::new("DIALOG_GCD_K2", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_FUSED_FOLD", "1"),
    EnvVar::new("DIALOG_GCD_K2_PAIR_COMPRESS", "1"),
    EnvVar::new("DIALOG_GCD_ACTIVE_ITERATIONS", "258"),
    EnvVar::new("DIALOG_GCD_PERPOS_MAJ2", "1"),
    EnvVar::new("DIALOG_GCD_FUSED_HCLEAR_MEASURED", "1"),
    EnvVar::new("DIALOG_GCD_FUSED_DCLEAR_MEASURED", "1"),
    EnvVar::new("DIALOG_GCD_FUSED_HALVE_EDCLEAR_MEASURED", "1"),
    EnvVar::new("DIALOG_GCD_RAW_IPMUL_TERMINAL_REUSE", "1"),
    EnvVar::new("DIALOG_GCD_RAW_IPMUL_CLEAR_P_RESIDUAL", "1"),
    EnvVar::new("DIALOG_GCD_RAW_QUOTIENT_TERMINAL_REUSE", "1"),
    EnvVar::new("DIALOG_GCD_RAW_APPLY_REVERSE_MATERIALIZED_SPECIAL_SUB", "1"),
    EnvVar::new("DIALOG_GCD_RAW_APPLY_MATERIALIZED_SPECIAL_ADD", "1"),
    EnvVar::new("DIALOG_GCD_RAW_APPLY_TRUNCATED_CLEAN", "1"),
    EnvVar::new("DIALOG_GCD_RAW_TOBITVECTOR_MATERIALIZED_SUB", "0"),
    EnvVar::new("DIALOG_GCD_RAW_TOBITVECTOR_VARIABLE_WIDTH", "1"),
    EnvVar::new("DIALOG_GCD_RAW_TOBITVECTOR_BORROW_FUTURE_LOG_CARRIES", "1"),
    EnvVar::new("ROUND84_XTAIL_KARATSUBA", "0"),
    EnvVar::new("KARA_SOL_DBL_FAST", "1"),
    EnvVar::new("KARA_FREE_Z1_TOPBIT", "1"),
    EnvVar::new("DIALOG_GCD_WIDTH_MARGIN", "10"),
    EnvVar::new("DIALOG_GCD_MEASURED_APPLY_SUB", "1"),
    EnvVar::new("DIALOG_GCD_HOST_GATED", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_WINDOW_BLOCKS", "2"),
    EnvVar::new("ROUND84_XTAIL_BORROW_CARRIES", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "16"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUSTOM4", "0"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUSTOM5", "0"),
    EnvVar::new("KARA_Z02_LOWQ", "1"),
    EnvVar::new("KARA_Z2_SELFHOST", "1"),
    EnvVar::new("KARA_SOL_MOD_VENT", "1"),
    EnvVar::new("DIALOG_GCD_BRANCH_BITS_HOST_COMPARATOR", "1"),
    EnvVar::new("DIALOG_GCD_BODY_HOST_CIN", "1"),
    EnvVar::new("DIALOG_GCD_LATE_BORROW_UV_HIGH", "1"),
    EnvVar::new("DIALOG_GCD_BODY_CARRY_BAND_TRIMS", "0,3,3,3,3,3,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,3,3"),
    EnvVar::new("DIALOG_GCD_TOBITVECTOR_CSWAP_BODY_TRIM", "0"),
    EnvVar::new("DIALOG_GCD_BINDER_NOTCH_STEPS", "8,9,10"),
    EnvVar::new("DIALOG_GCD_BINDER_NOTCH_EXTRA", "3"),
    EnvVar::new("DIALOG_GCD_BINDER_NOTCH_MAP", "11:1,12:1,13:1"),
    EnvVar::new("DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS", "113:21,131:21,142:22,187:23,205:22,210:21"),
    EnvVar::new("DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS", "42:22,91:22,118:22,149:21"),
    EnvVar::new("DIALOG_GCD_FUSED_OVFCLEAR_MEASURED", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_FINAL_LOWQ", "0"),
    EnvVar::new("R84_LOWQ", "1"),
    EnvVar::new("R84_LOWQ_CIN_BORROW", "1"),
    EnvVar::new("R84_QPROD_NAF", "1"),
    EnvVar::new("ROUND84_INPLACE_SOLINAS_FOLD", "1"),
    EnvVar::new("ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W", "21"),
    EnvVar::new("SQUARE_ROW_MAX_SEG", "176"),
    EnvVar::new("DIALOG_GCD_K5_CLEAN_BLOCK", "1"),
    EnvVar::new("DIALOG_GCD_FOLD_PARK_LOW_CARRIES", "1"),
    EnvVar::new("DIALOG_GCD_SPECIAL_FOLD_BORROW_CARRIES", "1"),
    EnvVar::new("DIALOG_GCD_K2_APPLY_INPLACE_RAW_BLOCK", "1"),
    EnvVar::new("DIALOG_GCD_FOLD_FREED_TAIL", "1"),
    EnvVar::new("DIALOG_GCD_BORROW_CURRENT_S2", "1"),
    EnvVar::new("DIALOG_GCD_BORROW_ZERO_RAW_FUTURE", "1"),
    EnvVar::new("DIALOG_GCD_FREE_SCRATCH_BEFORE_SHIFT", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_BOUNDARY_SPLIT", "100"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUT", "50"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUT2", "100"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUT3", "150"),
    EnvVar::new("DIALOG_GCD_APPLY_CHUNKED_F_CUT4", "190"),
    EnvVar::new("DIALOG_GCD_WIDTH_SLOPE_X1000", "1015"),
    EnvVar::new("DIALOG_REROLL", "4269"),
    EnvVar::new("DIALOG_POST_SUB_REROLL", "503292"),
    EnvVar::new("DIALOG_GCD_SELECTED_BODY_NOCIN", "1"),
    EnvVar::new("DIALOG_TAIL_NONCE", "9600076011007"),
    EnvVar::new("ROUND84_FOLD_FAST_ADD", "0"),
    EnvVar::new("DIALOG_GCD_FOLD_MAJ2", "1"),
    EnvVar::new("DIALOG_GCD_FOLD_MAJ1", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_FINAL_TOPCLEAN", "0"),
    EnvVar::new("ROUND84_QPROD_VENT_PAD", "1"),
    EnvVar::new("DIALOG_GCD_FOLD_FREED_TAIL_ED", "1"),
    EnvVar::new("DIALOG_GCD_APPLY_FINAL_WINDOWED_FAST_BLOCKS", "0"),
    EnvVar::new("DIALOG_GCD_FUSED_BRANCH_BITS", "1"),
    EnvVar::new("DIALOG_GCD_ODD_U_LOWBIT_FASTPATH", "1"),
];

#[rustfmt::skip]
const ACCEPTED_CF310EC_SUBMISSION_PINS: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_ACTIVE_ITERATIONS", "258"),
    EnvMutation::Set("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "18"),
    EnvMutation::Remove("DIALOG_GCD_APPLY_CHUNKED_F_CUTS"),
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "0"),
    EnvMutation::Set("DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS", "20"),
    EnvMutation::Set("DIALOG_GCD_APPLY_IMPLICIT_HIGH_ZERO", "1"),
    EnvMutation::Set("DIALOG_GCD_BINDER_NOTCH_EXTRA", "3"),
    EnvMutation::Set("DIALOG_GCD_BINDER_NOTCH_MAP", "11:1,12:1,13:1"),
    EnvMutation::Set("DIALOG_GCD_BINDER_NOTCH_STEPS", "8,9,10"),
    EnvMutation::Set("DIALOG_GCD_BODY_CARRY_BAND_TRIMS", "0,3,3,3,3,3,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,3,3,3"),
    EnvMutation::Set("DIALOG_GCD_COMPARE_BITS", "46"),
    EnvMutation::Set("DIALOG_GCD_FOLD_CARRY_TRUNC_W", "18"),
    EnvMutation::Set("DIALOG_GCD_FOLD_FREED_TAIL", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_FREED_TAIL_ED", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_MAJ1", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_MAJ2", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_PARK_LOW_CARRIES", "7"),
    EnvMutation::Set("DIALOG_GCD_K2", "1"),
    EnvMutation::Set("DIALOG_GCD_K5_CLEAN_BLOCK", "1"),
    EnvMutation::Set("DIALOG_GCD_K5_HEAD11_CODEC", "1"),
    EnvMutation::Set("DIALOG_GCD_K5_FREE_CLEAN_BLOCK_DURING_SHIFT", "1"),
    EnvMutation::Set("DIALOG_GCD_ODD_U_LOWBIT_FASTPATH", "1"),
    EnvMutation::Set("DIALOG_GCD_PA9024_COMPARE_SCHEDULE", "1"),
    EnvMutation::Set("DIALOG_GCD_PA9024_COMPARE_SCHEDULE_MARGIN", "0"),
    EnvMutation::Set("DIALOG_GCD_PERPOS_MAJ2", "1"),
    EnvMutation::Set("DIALOG_GCD_RAW_IPMUL_CLEAR_P_RESIDUAL", "1"),
    EnvMutation::Set("DIALOG_GCD_RAW_TOBITVECTOR_MATERIALIZED_SUB", "0"),
    EnvMutation::Set("DIALOG_GCD_RAW_TOBITVECTOR_VARIABLE_WIDTH", "1"),
    EnvMutation::Set("DIALOG_GCD_SKIP_ZERO_EDGE_CSHIFT", "1"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_FOLD_BORROW_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES", "5"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_FOLD_RELEASE_SCRATCH", "1"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS", "113:21,131:21,142:22,187:23,205:22,210:21"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS", "42:22,91:22,118:22,149:21"),
    EnvMutation::Set("DIALOG_GCD_TOBITVECTOR_CSWAP_BODY_TRIM", "0"),
    EnvMutation::Set("DIALOG_GCD_WIDTH_MARGIN", "10"),
    EnvMutation::Set("DIALOG_GCD_WIDTH_SLOPE_X1000", "1015"),
    EnvMutation::Set("KAL_DOUBLE_CARRY_TRUNC_W", "19"),
    EnvMutation::Set("KAL_FOLD_CARRY_TRUNC_W", "18"),
    EnvMutation::Set("SQUARE_ROW_MAX_SEG", "158"),
    EnvMutation::Set("SQUARE_ROW_WINDOW_CLEAN_COMPARE_BITS", "19"),
    EnvMutation::Set("SQUARE_ROW_WINDOW_CLEAN_ROW_BITS", "2:20,11:20,12:20,13:21,16:22,19:20,20:21,21:20,26:21,29:21,32:21,37:21,44:22,46:20,53:21,56:20,64:20,70:20,75:20,78:20,87:20"),
    EnvMutation::Set("SQUARE_ROW_WINDOW_MEASURED_CARRY_CLEAR", "1"),
    EnvMutation::Set("DIALOG_TAIL_NONCE", "3452376"),
];

#[rustfmt::skip]
const Q1175_DIRTY_QOFFSET_FIRST_OVERLAY: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_FIRST_DIRTY_QOFFSET", "1"),
];

#[rustfmt::skip]
const Q1175_DIRTY_QOFFSET_CORE_OVERLAY: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET", "1"),
];

#[rustfmt::skip]
const Q1175_BOUNDARY_BORROW_OVERLAY: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET", "1"),
    EnvMutation::Set("DIALOG_GCD_BOUNDARY_REPLAY_BORROW_CLEANED", "1"),
];

#[rustfmt::skip]
const Q1175_APPLY_CHUNK_SHAPE_OVERLAY: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET", "1"),
    EnvMutation::Set("DIALOG_GCD_BOUNDARY_REPLAY_BORROW_CLEANED", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "25"),
    EnvMutation::Set(
        "DIALOG_GCD_APPLY_CHUNKED_F_CUTS",
        "16,31,46,61,75,89,103,116,129,141,153,164,175,185,195,204,213,221,229,236,243,249,253,255",
    ),
];

#[rustfmt::skip]
const Q1175_REPAIRED_CLEAN_91794252_OVERLAY: &[EnvMutation] = &[
    EnvMutation::Set("DIALOG_GCD_BOUNDARY_REPLAY_BORROW_CLEANED", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_STREAM_CONTROLS", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_HOST_STREAMED_CONTROL", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_HOST_E_TOP_CARRY", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_HOST_D_CARRY12", "1"),
    EnvMutation::Set("DIALOG_GCD_FOLD_HOST_OVF2_CARRY13", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS", "22"),
    EnvMutation::Set("DIALOG_GCD_FOLD_CARRY_TRUNC_W", "18"),
    EnvMutation::Set("DIALOG_GCD_FOLD_PARK_LOW_CARRIES", "17"),
    EnvMutation::Set("DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES", "15"),
    EnvMutation::Set("SQUARE_ROW_MAX_SEG", "144"),
    EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1"),
    EnvMutation::Set("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "25"),
    EnvMutation::Set(
        "DIALOG_GCD_APPLY_CHUNKED_F_CUTS",
        "16,31,46,61,75,89,103,116,129,141,153,164,175,185,195,204,213,221,229,236,243,249,253,255",
    ),
    EnvMutation::Set("DIALOG_TAIL_NONCE", "91794252"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_cf310ec_has_expected_submission_pins() {
        let preset = PointAddRoutePreset::accepted_cf310ec();
        assert_eq!(preset.defaults.len(), 109);
        assert_eq!(preset.submission_pins.len(), 44);
        assert!(preset
            .defaults
            .iter()
            .any(|env| env.name == "SKIP_ALT_SEED_CHECKS" && env.value == "1"));
        assert!(preset
            .submission_pins
            .iter()
            .any(|pin| matches!(pin, EnvMutation::Set("DIALOG_TAIL_NONCE", "3452376"))));
        assert!(preset
            .submission_pins
            .iter()
            .any(|pin| matches!(pin, EnvMutation::Set("SQUARE_ROW_MAX_SEG", "158"))));
        assert!(preset.submission_pins.iter().any(|pin| matches!(
            pin,
            EnvMutation::Set("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "18")
        )));
        assert!(preset
            .submission_pins
            .iter()
            .any(|pin| matches!(pin, EnvMutation::Remove("DIALOG_GCD_APPLY_CHUNKED_F_CUTS"))));
        assert_eq!(preset.solinas_cuccaro_adder_sites().len(), 5);
        assert!(preset
            .solinas_cuccaro_adder_sites()
            .iter()
            .any(|site| site.point_add_phase == "round84_quotient_fold"
                && site.key.constant_form == ConstantForm::SignedSparseC));
    }

    #[test]
    fn q1175_repaired_clean_overlay_is_explicit_and_ordered_after_cf310ec() {
        assert_eq!(Q1175_DIRTY_QOFFSET_FIRST_OVERLAY.len(), 2);
        assert_eq!(Q1175_DIRTY_QOFFSET_CORE_OVERLAY.len(), 2);
        assert_eq!(Q1175_BOUNDARY_BORROW_OVERLAY.len(), 3);
        assert_eq!(Q1175_APPLY_CHUNK_SHAPE_OVERLAY.len(), 5);
        assert_eq!(Q1175_REPAIRED_CLEAN_91794252_OVERLAY.len(), 16);
        assert!(Q1175_DIRTY_QOFFSET_FIRST_OVERLAY.iter().any(|pin| matches!(
            pin,
            EnvMutation::Set("DIALOG_GCD_APPLY_FIRST_DIRTY_QOFFSET", "1")
        )));
        assert!(Q1175_DIRTY_QOFFSET_CORE_OVERLAY.iter().any(|pin| matches!(
            pin,
            EnvMutation::Set("DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET", "1")
        )));
        assert!(Q1175_REPAIRED_CLEAN_91794252_OVERLAY
            .iter()
            .any(|pin| matches!(
                pin,
                EnvMutation::Set("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS", "25")
            )));
        assert!(Q1175_REPAIRED_CLEAN_91794252_OVERLAY
            .iter()
            .any(|pin| matches!(
                pin,
                EnvMutation::Set("DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES", "1")
            )));
        assert!(Q1175_REPAIRED_CLEAN_91794252_OVERLAY
            .iter()
            .any(|pin| matches!(pin, EnvMutation::Set("DIALOG_TAIL_NONCE", "91794252"))));
    }
}
