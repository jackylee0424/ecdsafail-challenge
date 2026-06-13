use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteEntryKind {
    DefaultIfAbsent,
    Forced,
    Removed,
}

impl RouteEntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RouteEntryKind::DefaultIfAbsent => "default_if_absent",
            RouteEntryKind::Forced => "forced",
            RouteEntryKind::Removed => "removed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub kind: RouteEntryKind,
    pub name: String,
    pub value: Option<String>,
    pub source_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutePreset {
    pub entries: Vec<RouteEntry>,
}

impl RoutePreset {
    pub fn effective_env(&self, initial: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut env = initial.clone();
        for entry in &self.entries {
            match entry.kind {
                RouteEntryKind::DefaultIfAbsent => {
                    if let Some(value) = &entry.value {
                        env.entry(entry.name.clone())
                            .or_insert_with(|| value.clone());
                    }
                }
                RouteEntryKind::Forced => {
                    if let Some(value) = &entry.value {
                        env.insert(entry.name.clone(), value.clone());
                    }
                }
                RouteEntryKind::Removed => {
                    env.remove(&entry.name);
                }
            }
        }
        env
    }

    pub fn value_for(&self, kind: RouteEntryKind, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.kind == kind && entry.name == name)
            .and_then(|entry| entry.value.as_deref())
    }

    pub fn has_removed(&self, name: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.kind == RouteEntryKind::Removed && entry.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

pub fn parse_route_preset(source: &str) -> Result<RoutePreset, ParseError> {
    match parse_configure_route_preset(source) {
        Ok(preset) => Ok(preset),
        Err(configure_err) => parse_typed_route_preset(source).map_err(|typed_err| {
            ParseError::new(format!(
                "could not parse legacy configure route ({configure_err}) or typed route preset ({typed_err})"
            ))
        }),
    }
}

fn parse_configure_route_preset(source: &str) -> Result<RoutePreset, ParseError> {
    let body = extract_function_body(source, "configure_ecdsafail_submission_route")?;
    let body = strip_rust_comments(body)?;
    let mut entries = Vec::new();

    for (statement_index, statement) in split_statements(&body).into_iter().enumerate() {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        if let Some(entry) = parse_route_statement(statement, statement_index)? {
            entries.push(entry);
        }
    }

    if entries.is_empty() {
        return Err(ParseError::new(
            "configure_ecdsafail_submission_route contained no route entries".to_string(),
        ));
    }
    Ok(RoutePreset { entries })
}

fn parse_typed_route_preset(source: &str) -> Result<RoutePreset, ParseError> {
    let source = strip_rust_comments(source)?;
    let mut entries = Vec::new();
    let mut in_defaults = false;
    let mut in_pins = false;

    for (line_index, line) in source.lines().enumerate() {
        let statement = line.trim().trim_end_matches(',').trim();
        if statement.is_empty() {
            continue;
        }
        if statement.starts_with("const ACCEPTED_CF310EC_DEFAULTS:") {
            in_defaults = true;
            continue;
        }
        if statement.starts_with("const ACCEPTED_CF310EC_SUBMISSION_PINS:") {
            in_pins = true;
            continue;
        }
        if statement == "];" {
            in_defaults = false;
            in_pins = false;
            continue;
        }
        if in_defaults {
            let Some(args) = call_args(statement, "EnvVar::new")? else {
                continue;
            };
            let strings = parse_string_args(args, "EnvVar::new", 2, line_index)?;
            entries.push(RouteEntry {
                kind: RouteEntryKind::DefaultIfAbsent,
                name: validate_env_name(&strings[0], "EnvVar::new", line_index)?,
                value: Some(strings[1].clone()),
                source_index: line_index,
            });
        } else if in_pins {
            if let Some(args) = call_args(statement, "EnvMutation::Set")? {
                let strings = parse_string_args(args, "EnvMutation::Set", 2, line_index)?;
                entries.push(RouteEntry {
                    kind: RouteEntryKind::Forced,
                    name: validate_env_name(&strings[0], "EnvMutation::Set", line_index)?,
                    value: Some(strings[1].clone()),
                    source_index: line_index,
                });
            } else if let Some(args) = call_args(statement, "EnvMutation::Remove")? {
                let strings = parse_string_args(args, "EnvMutation::Remove", 1, line_index)?;
                entries.push(RouteEntry {
                    kind: RouteEntryKind::Removed,
                    name: validate_env_name(&strings[0], "EnvMutation::Remove", line_index)?,
                    value: None,
                    source_index: line_index,
                });
            }
        }
    }

    if entries.is_empty() {
        return Err(ParseError::new(
            "typed route preset contained no EnvVar or EnvMutation entries",
        ));
    }
    Ok(RoutePreset { entries })
}

pub fn parse_declared_adder_site_contract(source: &str) -> Result<Vec<String>, ParseError> {
    let source = strip_rust_comments(source)?;
    let body = extract_const_array(&source, "ACCEPTED_CF310EC_ADDER_SITES")?;
    let mut contract = Vec::new();

    for (index, item) in split_top_level_args(body)?.into_iter().enumerate() {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let args = call_args(item, "SolinasCuccaroAdderSite::new")?.ok_or_else(|| {
            ParseError::new(format!(
                "malformed ACCEPTED_CF310EC_ADDER_SITES item {index}: expected SolinasCuccaroAdderSite::new"
            ))
        })?;
        let parts = split_top_level_args(args)?;
        if parts.len() != 13 {
            return Err(ParseError::new(format!(
                "malformed SolinasCuccaroAdderSite::new item {index}: expected 13 args, found {}",
                parts.len()
            )));
        }

        let point_add_phase =
            parse_string_literal_arg(parts[0], "SolinasCuccaroAdderSite::new", index)?;
        let key = SolinasCuccaroAdderKey {
            operation: parse_operation_arg(parts[1], index)?,
            constant_form: parse_constant_form_arg(parts[2], index)?,
            control_model: parse_control_model_arg(parts[3], index)?,
            carry_model: parse_carry_model_arg(parts[4], index)?,
            cleanup_model: parse_cleanup_model_arg(parts[5], index)?,
            host_model: parse_host_model_arg(parts[6], index)?,
        };
        let axis_kind = parse_axis_kind_arg(parts[7], index)?;
        let mdd_node = parse_string_literal_arg(parts[8], "SolinasCuccaroAdderSite::new", index)?;
        let tla_model = parse_string_literal_arg(parts[9], "SolinasCuccaroAdderSite::new", index)?;
        let z3_query = parse_string_literal_arg(parts[10], "SolinasCuccaroAdderSite::new", index)?;
        let lean_theorem =
            parse_string_literal_arg(parts[11], "SolinasCuccaroAdderSite::new", index)?;
        let classification_note =
            parse_string_literal_arg(parts[12], "SolinasCuccaroAdderSite::new", index)?;
        contract.push(adder_site_contract_line(
            &point_add_phase,
            &key,
            axis_kind,
            &mdd_node,
            &tla_model,
            &z3_query,
            &lean_theorem,
            &classification_note,
        ));
    }

    if contract.is_empty() {
        return Err(ParseError::new(
            "ACCEPTED_CF310EC_ADDER_SITES contained no formal site declarations",
        ));
    }
    contract.sort();
    Ok(contract)
}

fn extract_const_array<'a>(source: &'a str, name: &str) -> Result<&'a str, ParseError> {
    let start = source
        .find(&format!("const {name}:"))
        .ok_or_else(|| ParseError::new(format!("missing const array {name}")))?;
    let open = source[start..]
        .find("= &[")
        .map(|offset| start + offset + 3)
        .ok_or_else(|| ParseError::new(format!("missing = &[ opener for {name}")))?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (offset, ch) in source[open..].char_indices() {
        let absolute = open + offset;
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '[' {
            depth += 1;
        } else if ch == ']' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok(&source[open + 1..absolute]);
            }
        }
    }
    Err(ParseError::new(format!("missing closing ] for {name}")))
}

fn extract_function_body<'a>(source: &'a str, name: &str) -> Result<&'a str, ParseError> {
    let function_start = source
        .find(&format!("fn {name}"))
        .ok_or_else(|| ParseError::new(format!("missing function {name}")))?;
    let open_brace = source[function_start..]
        .find('{')
        .map(|offset| function_start + offset)
        .ok_or_else(|| ParseError::new(format!("missing opening brace for {name}")))?;

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;
    let mut chars = source[open_brace..].char_indices().peekable();
    while let Some((offset, ch)) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if block_comment_depth > 0 {
            if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
                chars.next();
                block_comment_depth += 1;
            } else if ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                block_comment_depth -= 1;
            }
            continue;
        }
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            chars.next();
            in_line_comment = true;
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            chars.next();
            block_comment_depth = 1;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let close = open_brace + offset;
                return Ok(&source[open_brace + 1..close]);
            }
        }
    }
    Err(ParseError::new(format!("missing closing brace for {name}")))
}

fn strip_rust_comments(source: &str) -> Result<String, ParseError> {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.char_indices().peekable();
    let mut in_string = false;
    let mut escape = false;
    let mut in_line_comment = false;
    let mut block_comment_depth = 0usize;

    while let Some((_, ch)) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if block_comment_depth > 0 {
            if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
                chars.next();
                block_comment_depth += 1;
                out.push(' ');
                out.push(' ');
            } else if ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                block_comment_depth -= 1;
                out.push(' ');
                out.push(' ');
            } else if ch == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
        } else if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            chars.next();
            in_line_comment = true;
            out.push(' ');
            out.push(' ');
        } else if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            chars.next();
            block_comment_depth = 1;
            out.push(' ');
            out.push(' ');
        } else {
            out.push(ch);
        }
    }

    if in_string {
        return Err(ParseError::new(
            "unterminated string literal in route source",
        ));
    }
    if block_comment_depth > 0 {
        return Err(ParseError::new(
            "unterminated block comment in route source",
        ));
    }
    Ok(out)
}

fn parse_route_statement(
    statement: &str,
    statement_index: usize,
) -> Result<Option<RouteEntry>, ParseError> {
    if let Some(args) = call_args(statement, "set_default_env")? {
        let strings = parse_string_args(args, "set_default_env", 2, statement_index)?;
        return Ok(Some(RouteEntry {
            kind: RouteEntryKind::DefaultIfAbsent,
            name: validate_env_name(&strings[0], "set_default_env", statement_index)?,
            value: Some(strings[1].clone()),
            source_index: statement_index,
        }));
    }
    if let Some(args) = call_args(statement, "std::env::set_var")? {
        let strings = parse_string_args(args, "std::env::set_var", 2, statement_index)?;
        return Ok(Some(RouteEntry {
            kind: RouteEntryKind::Forced,
            name: validate_env_name(&strings[0], "std::env::set_var", statement_index)?,
            value: Some(strings[1].clone()),
            source_index: statement_index,
        }));
    }
    if let Some(args) = call_args(statement, "std::env::remove_var")? {
        let strings = parse_string_args(args, "std::env::remove_var", 1, statement_index)?;
        return Ok(Some(RouteEntry {
            kind: RouteEntryKind::Removed,
            name: validate_env_name(&strings[0], "std::env::remove_var", statement_index)?,
            value: None,
            source_index: statement_index,
        }));
    }
    Ok(None)
}

fn call_args<'a>(statement: &'a str, callee: &str) -> Result<Option<&'a str>, ParseError> {
    let Some(callee_start) = find_callee(statement, callee) else {
        return Ok(None);
    };
    let after_callee = callee_start + callee.len();
    let Some(open_rel) = statement[after_callee..].find('(') else {
        return Err(ParseError::new(format!("malformed {callee}: missing '('")));
    };
    if !statement[after_callee..after_callee + open_rel]
        .chars()
        .all(char::is_whitespace)
    {
        return Err(ParseError::new(format!(
            "malformed {callee}: unexpected tokens before '('"
        )));
    }
    let open = after_callee + open_rel;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in statement[open..].char_indices() {
        let absolute = open + idx;
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                if !statement[absolute + ch.len_utf8()..].trim().is_empty() {
                    return Err(ParseError::new(format!(
                        "malformed {callee}: unexpected tokens after call"
                    )));
                }
                return Ok(Some(&statement[open + 1..absolute]));
            }
        }
    }
    Err(ParseError::new(format!(
        "malformed {callee}: missing closing ')'"
    )))
}

fn find_callee(statement: &str, callee: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escape = false;
    for (start, ch) in statement.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            continue;
        }
        if statement[start..].starts_with(callee) {
            let before = statement[..start].chars().next_back();
            let after = statement[start + callee.len()..].chars().next();
            let before_ok = before.map(is_ident_continue).map(|v| !v).unwrap_or(true);
            let after_ok = after
                .map(|ch| ch == '(' || ch.is_whitespace())
                .unwrap_or(false);
            if before_ok && after_ok {
                return Some(start);
            }
        }
    }
    None
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn parse_string_args(
    args: &str,
    callee: &str,
    expected: usize,
    statement_index: usize,
) -> Result<Vec<String>, ParseError> {
    let parts = split_top_level_args(args)?;
    if parts.len() != expected {
        return Err(ParseError::new(format!(
            "malformed {callee} at route statement {statement_index}: expected {expected} string literal args, found {}",
            parts.len()
        )));
    }
    parts
        .into_iter()
        .map(|part| parse_string_literal_arg(part, callee, statement_index))
        .collect()
}

fn split_top_level_args(args: &str) -> Result<Vec<&str>, ParseError> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in args.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    return Err(ParseError::new("unbalanced delimiter in route assignment"));
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                parts.push(&args[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    if in_string {
        return Err(ParseError::new(
            "unterminated string literal in route assignment",
        ));
    }
    if depth != 0 {
        return Err(ParseError::new("unbalanced delimiter in route assignment"));
    }
    let tail = &args[start..];
    if !tail.trim().is_empty() {
        parts.push(tail);
    }
    Ok(parts)
}

fn parse_string_literal_arg(
    raw: &str,
    callee: &str,
    statement_index: usize,
) -> Result<String, ParseError> {
    let raw = raw.trim();
    let (value, consumed) = parse_string_literal(raw).map_err(|err| {
        ParseError::new(format!(
            "malformed {callee} at route statement {statement_index}: {err}"
        ))
    })?;
    if !raw[consumed..].trim().is_empty() {
        return Err(ParseError::new(format!(
            "malformed {callee} at route statement {statement_index}: argument is not a plain string literal"
        )));
    }
    Ok(value)
}

fn parse_string_literal(raw: &str) -> Result<(String, usize), &'static str> {
    if !raw.starts_with('"') {
        return Err("argument is not a string literal");
    }
    let mut value = String::new();
    let mut escape = false;
    for (idx, ch) in raw[1..].char_indices() {
        let absolute = idx + 1;
        if escape {
            value.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                other => other,
            });
            escape = false;
        } else if ch == '\\' {
            escape = true;
        } else if ch == '"' {
            return Ok((value, absolute + ch.len_utf8()));
        } else {
            value.push(ch);
        }
    }
    Err("unterminated string literal")
}

fn validate_env_name(
    name: &str,
    callee: &str,
    statement_index: usize,
) -> Result<String, ParseError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit())
    {
        return Err(ParseError::new(format!(
            "malformed {callee} at route statement {statement_index}: invalid env name {name:?}"
        )));
    }
    Ok(name.to_string())
}

fn enum_variant_arg<'a>(
    raw: &'a str,
    enum_name: &str,
    site_index: usize,
) -> Result<&'a str, ParseError> {
    let raw = raw.trim();
    let prefix = format!("{enum_name}::");
    raw.strip_prefix(&prefix).ok_or_else(|| {
        ParseError::new(format!(
            "malformed SolinasCuccaroAdderSite::new item {site_index}: expected {enum_name}:: variant, found {raw:?}"
        ))
    })
}

fn parse_operation_arg(raw: &str, site_index: usize) -> Result<Operation, ParseError> {
    match enum_variant_arg(raw, "Operation", site_index)? {
        "Add" => Ok(Operation::Add),
        "Sub" => Ok(Operation::Sub),
        "Fold" => Ok(Operation::Fold),
        "Double" => Ok(Operation::Double),
        "Halve" => Ok(Operation::Halve),
        other => Err(ParseError::new(format!(
            "unknown Operation variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_constant_form_arg(raw: &str, site_index: usize) -> Result<ConstantForm, ParseError> {
    match enum_variant_arg(raw, "ConstantForm", site_index)? {
        "SparseC" => Ok(ConstantForm::SparseC),
        "SignedSparseC" => Ok(ConstantForm::SignedSparseC),
        "Generic" => Ok(ConstantForm::Generic),
        other => Err(ParseError::new(format!(
            "unknown ConstantForm variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_control_model_arg(raw: &str, site_index: usize) -> Result<ControlModel, ParseError> {
    match enum_variant_arg(raw, "ControlModel", site_index)? {
        "Materialized" => Ok(ControlModel::Materialized),
        "DirectControlled" => Ok(ControlModel::DirectControlled),
        "ControlByPrep" => Ok(ControlModel::ControlByPrep),
        other => Err(ParseError::new(format!(
            "unknown ControlModel variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_carry_model_arg(raw: &str, site_index: usize) -> Result<CarryModel, ParseError> {
    match enum_variant_arg(raw, "CarryModel", site_index)? {
        "Full" => Ok(CarryModel::Full),
        "TruncatedWindow" => Ok(CarryModel::TruncatedWindow),
        "GuardedTail" => Ok(CarryModel::GuardedTail),
        other => Err(ParseError::new(format!(
            "unknown CarryModel variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_cleanup_model_arg(raw: &str, site_index: usize) -> Result<CleanupModel, ParseError> {
    match enum_variant_arg(raw, "CleanupModel", site_index)? {
        "Coherent" => Ok(CleanupModel::Coherent),
        "MeasuredHmr" => Ok(CleanupModel::MeasuredHmr),
        "PhaseCorrected" => Ok(CleanupModel::PhaseCorrected),
        other => Err(ParseError::new(format!(
            "unknown CleanupModel variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_host_model_arg(raw: &str, site_index: usize) -> Result<HostModel, ParseError> {
    match enum_variant_arg(raw, "HostModel", site_index)? {
        "Fresh" => Ok(HostModel::Fresh),
        "Borrowed" => Ok(HostModel::Borrowed),
        "Streamed" => Ok(HostModel::Streamed),
        "DerivedControlHosted" => Ok(HostModel::DerivedControlHosted),
        other => Err(ParseError::new(format!(
            "unknown HostModel variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn parse_axis_kind_arg(raw: &str, site_index: usize) -> Result<AxisKind, ParseError> {
    match enum_variant_arg(raw, "AxisKind", site_index)? {
        "ResourceOnly" => Ok(AxisKind::ResourceOnly),
        "SeedOnly" => Ok(AxisKind::SeedOnly),
        "ProvableExact" => Ok(AxisKind::ProvableExact),
        "LossyIsland" => Ok(AxisKind::LossyIsland),
        "Unknown" => Ok(AxisKind::Unknown),
        other => Err(ParseError::new(format!(
            "unknown AxisKind variant {other:?} at formal site {site_index}"
        ))),
    }
}

fn split_statements(body: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in body.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == ';' {
            statements.push(&body[start..idx]);
            start = idx + ch.len_utf8();
        }
    }
    if start < body.len() {
        statements.push(&body[start..]);
    }
    statements
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operation {
    Add,
    Sub,
    Fold,
    Double,
    Halve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstantForm {
    SparseC,
    SignedSparseC,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlModel {
    Materialized,
    DirectControlled,
    ControlByPrep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CarryModel {
    Full,
    TruncatedWindow,
    GuardedTail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CleanupModel {
    Coherent,
    MeasuredHmr,
    PhaseCorrected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostModel {
    Fresh,
    Borrowed,
    Streamed,
    DerivedControlHosted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AxisKind {
    ResourceOnly,
    SeedOnly,
    ProvableExact,
    LossyIsland,
    Unknown,
}

macro_rules! as_str_impl {
    ($ty:ty, {$($variant:ident => $value:literal),+ $(,)?}) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }
    };
}

as_str_impl!(Operation, {
    Add => "add",
    Sub => "sub",
    Fold => "fold",
    Double => "double",
    Halve => "halve",
});
as_str_impl!(ConstantForm, {
    SparseC => "sparse_c",
    SignedSparseC => "signed_sparse_c",
    Generic => "generic",
});
as_str_impl!(ControlModel, {
    Materialized => "materialized",
    DirectControlled => "direct_controlled",
    ControlByPrep => "control_by_prep",
});
as_str_impl!(CarryModel, {
    Full => "full",
    TruncatedWindow => "truncated_window",
    GuardedTail => "guarded_tail",
});
as_str_impl!(CleanupModel, {
    Coherent => "coherent",
    MeasuredHmr => "measured_hmr",
    PhaseCorrected => "phase_corrected",
});
as_str_impl!(HostModel, {
    Fresh => "fresh",
    Borrowed => "borrowed",
    Streamed => "streamed",
    DerivedControlHosted => "derived_control_hosted",
});
as_str_impl!(AxisKind, {
    ResourceOnly => "resource_only",
    SeedOnly => "seed_only",
    ProvableExact => "provable_exact",
    LossyIsland => "lossy_island",
    Unknown => "unknown",
});

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SolinasCuccaroAdderKey {
    pub operation: Operation,
    pub constant_form: ConstantForm,
    pub control_model: ControlModel,
    pub carry_model: CarryModel,
    pub cleanup_model: CleanupModel,
    pub host_model: HostModel,
}

impl SolinasCuccaroAdderKey {
    pub fn canonical(&self) -> String {
        format!(
            "operation={};constant_form={};control_model={};carry_model={};cleanup_model={};host_model={}",
            self.operation.as_str(),
            self.constant_form.as_str(),
            self.control_model.as_str(),
            self.carry_model.as_str(),
            self.cleanup_model.as_str(),
            self.host_model.as_str()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofObligations {
    pub mdd_node: &'static str,
    pub tla_model: &'static str,
    pub z3_query: &'static str,
    pub lean_theorem: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AdderSite {
    pub point_add_phase: &'static str,
    pub key: SolinasCuccaroAdderKey,
    pub proof: ProofObligations,
    pub axis_kind: AxisKind,
    pub classification_note: &'static str,
}

pub fn adder_site_contract_lines(sites: &[AdderSite]) -> Vec<String> {
    let mut lines: Vec<String> = sites
        .iter()
        .map(|site| {
            adder_site_contract_line(
                site.point_add_phase,
                &site.key,
                site.axis_kind,
                site.proof.mdd_node,
                site.proof.tla_model,
                site.proof.z3_query,
                site.proof.lean_theorem,
                site.classification_note,
            )
        })
        .collect();
    lines.sort();
    lines
}

fn adder_site_contract_line(
    point_add_phase: &str,
    key: &SolinasCuccaroAdderKey,
    axis_kind: AxisKind,
    mdd_node: &str,
    tla_model: &str,
    z3_query: &str,
    lean_theorem: &str,
    classification_note: &str,
) -> String {
    format!(
        "point_add_phase={point_add_phase}|key={}|axis_kind={}|mdd_node={mdd_node}|tla_model={tla_model}|z3_query={z3_query}|lean_theorem={lean_theorem}|note={classification_note}",
        key.canonical(),
        axis_kind.as_str()
    )
}

pub fn classify_adder_sites(env: &BTreeMap<String, String>) -> Vec<AdderSite> {
    let kal_fold_carry = fold_carry_model(env);
    let fused_fold_carry = if has_map_or_positive(env, "DIALOG_GCD_FOLD_CARRY_TRUNC_STEP_WINDOWS") {
        CarryModel::GuardedTail
    } else if positive_usize(env, "DIALOG_GCD_FOLD_CARRY_TRUNC_W") {
        CarryModel::TruncatedWindow
    } else {
        kal_fold_carry
    };
    let special_fold_carry =
        if has_map_or_positive(env, "DIALOG_GCD_SPECIAL_FOLD_CARRY_TRUNC_STEP_WINDOWS")
            || enabled(env, "DIALOG_GCD_APPLY_IMPLICIT_HIGH_ZERO")
        {
            CarryModel::GuardedTail
        } else {
            kal_fold_carry
        };
    let double_carry = if positive_usize(env, "KAL_DOUBLE_CARRY_TRUNC_W") {
        CarryModel::TruncatedWindow
    } else {
        CarryModel::Full
    };
    let fold_cleanup = if enabled(env, "DIALOG_GCD_FUSED_HCLEAR_MEASURED")
        || enabled(env, "DIALOG_GCD_FUSED_DCLEAR_MEASURED")
        || enabled(env, "DIALOG_GCD_FUSED_HALVE_EDCLEAR_MEASURED")
    {
        CleanupModel::MeasuredHmr
    } else {
        CleanupModel::Coherent
    };
    let fold_host = if fold_streaming_enabled(env) {
        HostModel::Streamed
    } else if enabled(env, "DIALOG_GCD_FOLD_HOST_DERIVED_CONTROLS")
        || enabled(env, "DIALOG_GCD_FOLD_FREED_TAIL")
        || enabled(env, "DIALOG_GCD_FOLD_FREED_TAIL_ED")
        || enabled(env, "DIALOG_GCD_FOLD_HOST_N10")
        || enabled(env, "DIALOG_GCD_FOLD_HOST_H_N10")
        || enabled(env, "DIALOG_GCD_FOLD_HOST_H_XED_N10")
        || enabled(env, "DIALOG_GCD_FOLD_HOST_E")
        || enabled(env, "DIALOG_GCD_FOLD_HOST_D")
    {
        HostModel::DerivedControlHosted
    } else {
        HostModel::Fresh
    };
    let apply_control = if enabled(env, "DIALOG_GCD_RAW_APPLY_MATERIALIZED_SPECIAL_ADD")
        || enabled(env, "DIALOG_GCD_RAW_APPLY_REVERSE_MATERIALIZED_SPECIAL_SUB")
    {
        ControlModel::Materialized
    } else if enabled(env, "DIALOG_GCD_RAW_APPLY_DIRECT_SPECIAL_ADD") {
        ControlModel::DirectControlled
    } else {
        ControlModel::Materialized
    };
    let apply_cleanup = if enabled(env, "DIALOG_GCD_SPECIAL_CLEAN_CONDITIONAL_REPLAY") {
        CleanupModel::PhaseCorrected
    } else if enabled(env, "DIALOG_GCD_MEASURED_UNDERFLOW_GATE") {
        CleanupModel::MeasuredHmr
    } else {
        CleanupModel::Coherent
    };
    let apply_host = if enabled(env, "DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES")
        || enabled(env, "DIALOG_GCD_SPECIAL_FOLD_BORROW_CARRIES")
    {
        HostModel::Borrowed
    } else {
        HostModel::Fresh
    };

    let mut sites = vec![
        AdderSite {
            point_add_phase: "kal_double",
            key: SolinasCuccaroAdderKey {
                operation: Operation::Double,
                constant_form: ConstantForm::SparseC,
                control_model: ControlModel::DirectControlled,
                carry_model: double_carry,
                cleanup_model: CleanupModel::MeasuredHmr,
                host_model: if enabled(env, "KAL_VENT_DOUBLE") {
                    HostModel::Borrowed
                } else {
                    HostModel::Fresh
                },
            },
            proof: proof_obligations_for("kal_double"),
            axis_kind: axis_kind_for(double_carry, CleanupModel::MeasuredHmr, HostModel::Fresh),
            classification_note:
                "cadd sparse-c double path; carry truncation comes from KAL_DOUBLE_CARRY_TRUNC_W",
        },
        AdderSite {
            point_add_phase: "kal_halve",
            key: SolinasCuccaroAdderKey {
                operation: Operation::Halve,
                constant_form: ConstantForm::SparseC,
                control_model: ControlModel::DirectControlled,
                carry_model: double_carry,
                cleanup_model: CleanupModel::MeasuredHmr,
                host_model: HostModel::Fresh,
            },
            proof: proof_obligations_for("kal_halve"),
            axis_kind: axis_kind_for(double_carry, CleanupModel::MeasuredHmr, HostModel::Fresh),
            classification_note:
                "inverse sparse-c halve path; shares KAL_DOUBLE_CARRY_TRUNC_W with double",
        },
        AdderSite {
            point_add_phase: "kal_fold",
            key: SolinasCuccaroAdderKey {
                operation: Operation::Fold,
                constant_form: ConstantForm::SparseC,
                control_model: ControlModel::DirectControlled,
                carry_model: fused_fold_carry,
                cleanup_model: fold_cleanup,
                host_model: fold_host,
            },
            proof: proof_obligations_for("kal_fold"),
            axis_kind: axis_kind_for(fused_fold_carry, fold_cleanup, fold_host),
            classification_note:
                "fused double_y/halve_y fold; hosted and streamed controls are lifecycle obligations",
        },
        AdderSite {
            point_add_phase: "dialog_apply_special_fold",
            key: SolinasCuccaroAdderKey {
                operation: Operation::Fold,
                constant_form: ConstantForm::SparseC,
                control_model: apply_control,
                carry_model: special_fold_carry,
                cleanup_model: apply_cleanup,
                host_model: apply_host,
            },
            proof: proof_obligations_for("dialog_apply_special_fold"),
            axis_kind: axis_kind_for(special_fold_carry, apply_cleanup, apply_host),
            classification_note:
                "materialized/direct sparse-c apply fold; step clean bits and known-zero controls are hazards",
        },
        AdderSite {
            point_add_phase: "round84_quotient_fold",
            key: SolinasCuccaroAdderKey {
                operation: Operation::Fold,
                constant_form: if enabled(env, "R84_QPROD_NAF") {
                    ConstantForm::SignedSparseC
                } else {
                    ConstantForm::SparseC
                },
                control_model: ControlModel::ControlByPrep,
                carry_model: if positive_usize(env, "ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W") {
                    CarryModel::TruncatedWindow
                } else {
                    CarryModel::Full
                },
                cleanup_model: CleanupModel::PhaseCorrected,
                host_model: if enabled(env, "ROUND84_QPROD_VENT_PAD") {
                    HostModel::Streamed
                } else {
                    HostModel::Fresh
                },
            },
            proof: proof_obligations_for("round84_quotient_fold"),
            axis_kind: axis_kind_for(
                if positive_usize(env, "ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W") {
                    CarryModel::TruncatedWindow
                } else {
                    CarryModel::Full
                },
                CleanupModel::PhaseCorrected,
                if enabled(env, "ROUND84_QPROD_VENT_PAD") {
                    HostModel::Streamed
                } else {
                    HostModel::Fresh
                },
            ),
            classification_note:
                "round84 quotient*c fold; signed sparse form follows R84_QPROD_NAF",
        },
    ];
    sites.sort();
    sites
}

fn proof_obligations_for(point_add_phase: &str) -> ProofObligations {
    match point_add_phase {
        "kal_double" => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#kal_double",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_double",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#double_sparse_c_action",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.sparse_c_double_mod_p",
        },
        "kal_halve" => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#kal_halve",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_halve",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#halve_sparse_c_action",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.sparse_c_halve_double_inverse",
        },
        "kal_fold" => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#kal_fold",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#kal_fold",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#guarded_truncated_sparse_add",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.signed_sparse_c_fold",
        },
        "dialog_apply_special_fold" => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#dialog_apply_special_fold",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#dialog_apply_special_fold",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#guarded_truncated_sparse_add",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.apply_special_fold_action",
        },
        "round84_quotient_fold" => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#round84_quotient_fold",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#round84_quotient_fold",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#control_by_prep_scratch",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.round84_quotient_sparse_c",
        },
        _ => ProofObligations {
            mdd_node: "dev/solinas_cuccaro_adder_key.mmd#unknown",
            tla_model: "dev/formal/PointAddSolinasCuccaroAdder.tla#unknown",
            z3_query: "dev/formal/solinas_cuccaro_adder_bv.smt2#unknown",
            lean_theorem: "PointAdd.Formal.SolinasCuccaro.unknown",
        },
    }
}

fn axis_kind_for(
    carry_model: CarryModel,
    cleanup_model: CleanupModel,
    host_model: HostModel,
) -> AxisKind {
    if matches!(
        carry_model,
        CarryModel::TruncatedWindow | CarryModel::GuardedTail
    ) {
        AxisKind::LossyIsland
    } else if cleanup_model == CleanupModel::PhaseCorrected {
        AxisKind::Unknown
    } else if host_model != HostModel::Fresh {
        AxisKind::ResourceOnly
    } else {
        AxisKind::ProvableExact
    }
}

pub fn nonce_quality_input(env: &BTreeMap<String, String>, sites: &[AdderSite]) -> String {
    const ENV_FIELDS: &[&str] = &[
        "DIALOG_TAIL_NONCE",
        "DIALOG_GCD_ACTIVE_ITERATIONS",
        "DIALOG_GCD_WIDTH_MARGIN",
        "DIALOG_GCD_WIDTH_SLOPE_X1000",
        "DIALOG_GCD_COMPARE_BITS",
        "DIALOG_GCD_PA9024_COMPARE_SCHEDULE",
        "DIALOG_GCD_PA9024_COMPARE_SCHEDULE_MARGIN",
        "DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS",
        "DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS",
        "DIALOG_GCD_APPLY_CHUNKED_F_CUTS",
        "DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES",
        "DIALOG_GCD_APPLY_IMPLICIT_HIGH_ZERO",
        "DIALOG_GCD_BODY_CARRY_BAND_TRIMS",
        "DIALOG_GCD_COMPARE_STEP_BITS",
        "DIALOG_GCD_FOLD_CARRY_TRUNC_STEP_WINDOWS",
        "DIALOG_GCD_FOLD_FREED_TAIL",
        "DIALOG_GCD_FOLD_FREED_TAIL_ED",
        "DIALOG_GCD_FOLD_HOST_DERIVED_CONTROLS",
        "DIALOG_GCD_FOLD_HOST_STREAMED_CONTROL",
        "DIALOG_GCD_FOLD_PARK_LOW_CARRIES",
        "DIALOG_GCD_FOLD_STREAM_CONTROLS",
        "DIALOG_GCD_FUSED_DCLEAR_MEASURED",
        "DIALOG_GCD_FUSED_HALVE_EDCLEAR_MEASURED",
        "DIALOG_GCD_FUSED_HCLEAR_MEASURED",
        "DIALOG_GCD_FUSED_OVFCLEAR_MEASURED",
        "DIALOG_GCD_K5_HEAD11_CODEC",
        "DIALOG_GCD_MEASURED_UNDERFLOW_GATE",
        "DIALOG_GCD_RAW_APPLY_DIRECT_SPECIAL_ADD",
        "DIALOG_GCD_RAW_APPLY_MATERIALIZED_SPECIAL_ADD",
        "DIALOG_GCD_RAW_APPLY_REVERSE_MATERIALIZED_SPECIAL_SUB",
        "DIALOG_GCD_RAW_TOBITVECTOR_MATERIALIZED_SUB",
        "DIALOG_GCD_SELECTED_BODY_NOCIN",
        "DIALOG_GCD_SELECTED_BODY_STREAM_SUFFIX_MAP",
        "DIALOG_GCD_SPECIAL_CLEAN_CONDITIONAL_REPLAY",
        "DIALOG_GCD_SPECIAL_FOLD_BORROW_CARRIES",
        "DIALOG_GCD_SPECIAL_FOLD_CARRY_TRUNC_STEP_WINDOWS",
        "DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES",
        "DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS",
        "DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS",
        "DIALOG_GCD_TOBITVECTOR_CSWAP_BODY_TRIM",
        "DIALOG_GCD_TOBITVECTOR_SHIFT_BODY_TRIM",
        "KAL_DOUBLE_CARRY_TRUNC_W",
        "KAL_FOLD_CARRY_TRUNC_W",
        "R84_QPROD_NAF",
        "ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W",
        "ROUND84_QPROD_VENT_PAD",
    ];

    let mut out = String::from("nonce_quality_input_v1\n");
    for field in ENV_FIELDS {
        out.push_str(field);
        out.push('=');
        if let Some(value) = env.get(*field) {
            out.push_str(value);
        }
        out.push('\n');
    }
    for site in sites {
        out.push_str(site.point_add_phase);
        out.push(':');
        out.push_str(&site.key.canonical());
        out.push('\n');
    }
    out
}

pub fn nonce_quality_key(input: &str) -> String {
    format!("nqk-v1-{:016x}", fnv1a64(input.as_bytes()))
}

pub fn render_route_dump(preset: &RoutePreset, initial: &BTreeMap<String, String>) -> String {
    let effective = preset.effective_env(initial);
    let sites = classify_adder_sites(&effective);
    let nq_input = nonce_quality_input(&effective, &sites);
    let mut out = String::new();

    out.push_str("point_add_route_dump_v1\n");
    out.push_str("[route_entries]\n");
    for entry in &preset.entries {
        out.push_str(entry.kind.as_str());
        out.push(' ');
        out.push_str(&entry.name);
        if let Some(value) = &entry.value {
            out.push('=');
            out.push_str(value);
        }
        out.push('\n');
    }

    out.push_str("[effective_env]\n");
    for (name, value) in &effective {
        out.push_str(name);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }

    out.push_str("[solinas_cuccaro_adder_keys]\n");
    for site in &sites {
        out.push_str("point_add_phase=");
        out.push_str(site.point_add_phase);
        out.push_str(" SolinasCuccaroAdderKey=");
        out.push_str(&site.key.canonical());
        out.push_str(" axis_kind=");
        out.push_str(site.axis_kind.as_str());
        out.push_str(" proof_obligations(");
        out.push_str("mdd_node=");
        out.push_str(site.proof.mdd_node);
        out.push_str(";tla_model=");
        out.push_str(site.proof.tla_model);
        out.push_str(";z3_query=");
        out.push_str(site.proof.z3_query);
        out.push_str(";lean_theorem=");
        out.push_str(site.proof.lean_theorem);
        out.push(')');
        out.push_str(" note=");
        out.push_str(site.classification_note);
        out.push('\n');
    }

    out.push_str("[nonce_quality_key]\n");
    out.push_str("nonce_quality_key=");
    out.push_str(&nonce_quality_key(&nq_input));
    out.push('\n');
    out
}

pub fn render_patch_plan_report(preset: &RoutePreset) -> String {
    let forced: Vec<&RouteEntry> = preset
        .entries
        .iter()
        .filter(|entry| entry.kind == RouteEntryKind::Forced)
        .collect();
    let removed: Vec<&RouteEntry> = preset
        .entries
        .iter()
        .filter(|entry| entry.kind == RouteEntryKind::Removed)
        .collect();
    let effective = preset.effective_env(&BTreeMap::new());
    let sites = classify_adder_sites(&effective);
    let mut out = String::new();

    out.push_str("point_add_route_refactor_patch_plan_v1\n");
    out.push_str("[typed_preset_entries]\n");
    out.push_str("PointAddRoutePreset::accepted_cf310ec_defaults = current set_default_env stack before the hard q1185 block\n");
    out.push_str("PointAddRoutePreset::accepted_cf310ec_submission_pins = forced std::env::set_var block under the q1185 head11/implicit-zero comment\n");
    out.push_str("PointAddRoutePreset::accepted_cf310ec_removed_pins = std::env::remove_var entries that must clear caller env before build\n");
    out.push_str("PointAddRouteOverlay = caller/env overrides that are allowed only before submission pins\n");
    out.push_str(&format!(
        "counts default_if_absent={} forced={} removed={}\n",
        preset
            .entries
            .iter()
            .filter(|entry| entry.kind == RouteEntryKind::DefaultIfAbsent)
            .count(),
        forced.len(),
        removed.len()
    ));
    out.push_str("forced_pins=\n");
    for entry in &forced {
        if let Some(value) = &entry.value {
            out.push_str("  ");
            out.push_str(&entry.name);
            out.push('=');
            out.push_str(value);
            out.push('\n');
        }
    }
    out.push_str("removed_pins=\n");
    for entry in &removed {
        out.push_str("  ");
        out.push_str(&entry.name);
        out.push('\n');
    }

    out.push_str("[ported_shared_route_readers]\n");
    for item in PORTED_SHARED_ROUTE_READERS {
        out.push_str(item);
        out.push('\n');
    }

    out.push_str("[remaining_duplicated_env_readers]\n");
    for item in REMAINING_DUPLICATED_ENV_READERS {
        out.push_str(item);
        out.push('\n');
    }

    out.push_str("[conservative_or_unknown_solinas_cuccaro_sites]\n");
    for site in &sites {
        if site.axis_kind != AxisKind::ProvableExact {
            out.push_str(site.point_add_phase);
            out.push_str(" axis_kind=");
            out.push_str(site.axis_kind.as_str());
            out.push_str(" key=");
            out.push_str(&site.key.canonical());
            out.push_str(" note=");
            out.push_str(site.classification_note);
            out.push('\n');
        }
    }

    out.push_str("[first_behavior_preserving_src_touch_set]\n");
    for path in FIRST_PORT_TOUCH_SET {
        out.push_str(path);
        out.push('\n');
    }
    out.push_str(
        "excluded_remaining_arithmetic_files=src/point_add/arith/modular.rs,src/point_add/arith/multiply.rs\n",
    );
    out
}

const PORTED_SHARED_ROUTE_READERS: &[&str] = &[
    "DIALOG_GCD_ACTIVE_ITERATIONS circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_COMPARE_BITS circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_PA9024_COMPARE_SCHEDULE circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_PA9024_COMPARE_SCHEDULE_MARGIN circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_WIDTH_MARGIN circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_WIDTH_SLOPE_X1000 circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_BODY_CARRY_BAND_TRIMS circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_FOLD_CARRY_TRUNC_W circuit=src/point_add/arith/const_arith.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "KAL_DOUBLE_CARRY_TRUNC_W circuit=src/point_add/arith/const_arith.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "KAL_FOLD_CARRY_TRUNC_W circuit=src/point_add/arith/const_arith.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_FOLD_CARRY_TRUNC_STEP_WINDOWS circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_SPECIAL_FOLD_CARRY_TRUNC_STEP_WINDOWS circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_SPECIAL_UNDERFLOW_CLEAN_STEP_BITS circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_TOBITVECTOR_CSWAP_BODY_TRIM circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_TOBITVECTOR_SHIFT_BODY_TRIM circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_BINDER_NOTCH_* circuit=src/point_add/rounds/dialog/mod.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
    "DIALOG_GCD_K5_HEAD11_CODEC circuit=src/point_add/rounds/dialog/config.rs filter=src/point_add/dialog_gcd_classical_filter.rs",
];

const REMAINING_DUPLICATED_ENV_READERS: &[&str] = &[];

const FIRST_PORT_TOUCH_SET: &[&str] = &[
    "src/point_add/mod.rs",
    "src/point_add/route_config.rs",
    "src/point_add/route_preset.rs",
    "src/point_add/arith/const_arith.rs",
    "src/point_add/rounds/dialog/mod.rs",
    "src/point_add/rounds/dialog/config.rs",
    "src/point_add/dialog_gcd_classical_filter.rs",
];

fn enabled(env: &BTreeMap<String, String>, name: &str) -> bool {
    env.get(name).map(|value| value == "1").unwrap_or(false)
}

fn env_usize(env: &BTreeMap<String, String>, name: &str) -> Option<usize> {
    env.get(name).and_then(|value| value.parse::<usize>().ok())
}

fn positive_usize(env: &BTreeMap<String, String>, name: &str) -> bool {
    env_usize(env, name).map(|value| value > 0).unwrap_or(false)
}

fn has_map_or_positive(env: &BTreeMap<String, String>, name: &str) -> bool {
    env.get(name)
        .map(|value| !value.trim().is_empty() && value != "0")
        .unwrap_or(false)
}

fn fold_carry_model(env: &BTreeMap<String, String>) -> CarryModel {
    if positive_usize(env, "KAL_FOLD_CARRY_TRUNC_W") {
        CarryModel::TruncatedWindow
    } else {
        CarryModel::Full
    }
}

fn fold_streaming_enabled(env: &BTreeMap<String, String>) -> bool {
    let park_low = env_usize(env, "DIALOG_GCD_FOLD_PARK_LOW_CARRIES").unwrap_or(0);
    (enabled(env, "DIALOG_GCD_FOLD_HOST_STREAMED_CONTROL") && park_low >= 13)
        || (enabled(env, "DIALOG_GCD_FOLD_STREAM_CONTROLS") && park_low >= 12)
}

const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;
const OPS_MAGIC: &[u8; 8] = b"QECCOPS1";
const OPS_HEADER_BYTES: u64 = 16;
const OPS_RECORD_BYTES: u64 = 56;

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV1A64_PRIME);
    }
    hash
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    fnv1a64_update(FNV1A64_OFFSET, bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpsArtifactSummary {
    pub path: String,
    pub bytes: u64,
    pub op_count: Option<u64>,
    pub fnv1a64: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpsArtifactComparison {
    pub reference: OpsArtifactSummary,
    pub candidate: OpsArtifactSummary,
    pub byte_equal: bool,
}

impl OpsArtifactComparison {
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("ops_artifact_compare_v1\n");
        push_ops_summary(&mut out, "reference", &self.reference);
        push_ops_summary(&mut out, "candidate", &self.candidate);
        out.push_str("status=");
        out.push_str(if self.byte_equal { "match" } else { "mismatch" });
        out.push('\n');
        out
    }
}

fn push_ops_summary(out: &mut String, label: &str, summary: &OpsArtifactSummary) {
    out.push_str(label);
    out.push_str(" path=");
    out.push_str(&summary.path);
    out.push_str(" bytes=");
    out.push_str(&summary.bytes.to_string());
    out.push_str(" op_count=");
    match summary.op_count {
        Some(count) => out.push_str(&count.to_string()),
        None => out.push_str("invalid"),
    }
    out.push_str(" fnv1a64=");
    out.push_str(&format!("{:016x}", summary.fnv1a64));
    out.push('\n');
}

pub fn compare_ops_artifacts(
    reference: impl AsRef<Path>,
    candidate: impl AsRef<Path>,
) -> Result<OpsArtifactComparison, String> {
    let reference = reference.as_ref();
    let candidate = candidate.as_ref();
    let reference_summary = summarize_ops_artifact(reference)?;
    let candidate_summary = summarize_ops_artifact(candidate)?;
    let byte_equal =
        reference_summary.bytes == candidate_summary.bytes && files_equal(reference, candidate)?;

    Ok(OpsArtifactComparison {
        reference: reference_summary,
        candidate: candidate_summary,
        byte_equal,
    })
}

fn summarize_ops_artifact(path: &Path) -> Result<OpsArtifactSummary, String> {
    let mut file = File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let bytes = file
        .metadata()
        .map_err(|err| format!("metadata {}: {err}", path.display()))?
        .len();
    let mut hash = FNV1A64_OFFSET;
    let mut first = [0u8; OPS_HEADER_BYTES as usize];
    let mut first_len = 0usize;
    let mut buf = vec![0u8; 1024 * 1024];

    loop {
        let n = file
            .read(&mut buf)
            .map_err(|err| format!("read {}: {err}", path.display()))?;
        if n == 0 {
            break;
        }
        if first_len < first.len() {
            let take = (first.len() - first_len).min(n);
            first[first_len..first_len + take].copy_from_slice(&buf[..take]);
            first_len += take;
        }
        hash = fnv1a64_update(hash, &buf[..n]);
    }

    let op_count = if first_len == first.len() && &first[..OPS_MAGIC.len()] == OPS_MAGIC {
        let count = u64::from_le_bytes(
            first[OPS_MAGIC.len()..OPS_HEADER_BYTES as usize]
                .try_into()
                .unwrap(),
        );
        (OPS_HEADER_BYTES + count.saturating_mul(OPS_RECORD_BYTES) == bytes).then_some(count)
    } else {
        None
    };

    Ok(OpsArtifactSummary {
        path: path.display().to_string(),
        bytes,
        op_count,
        fnv1a64: hash,
    })
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, String> {
    let mut left_file =
        File::open(left).map_err(|err| format!("open {}: {err}", left.display()))?;
    let mut right_file =
        File::open(right).map_err(|err| format!("open {}: {err}", right.display()))?;
    let mut left_buf = vec![0u8; 1024 * 1024];
    let mut right_buf = vec![0u8; 1024 * 1024];

    loop {
        let left_n = left_file
            .read(&mut left_buf)
            .map_err(|err| format!("read {}: {err}", left.display()))?;
        let right_n = right_file
            .read(&mut right_buf)
            .map_err(|err| format!("read {}: {err}", right.display()))?;
        if left_n != right_n {
            return Ok(false);
        }
        if left_n == 0 {
            return Ok(true);
        }
        if left_buf[..left_n] != right_buf[..right_n] {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    const ROUTE_PRESET_RS: &str = include_str!("../../../src/point_add/route_preset.rs");

    fn current_preset() -> RoutePreset {
        parse_route_preset(ROUTE_PRESET_RS).expect("parse current point_add route")
    }

    fn nonce_key_for_env(env: &BTreeMap<String, String>) -> String {
        let sites = classify_adder_sites(env);
        nonce_quality_key(&nonce_quality_input(env, &sites))
    }

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "point_add_route_refactor_{name}_{}_{}.bin",
            std::process::id(),
            stamp
        ))
    }

    fn ops_fixture(op_count: u64, fill: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(OPS_MAGIC);
        bytes.extend_from_slice(&op_count.to_le_bytes());
        bytes.extend(std::iter::repeat(fill).take(op_count as usize * OPS_RECORD_BYTES as usize));
        bytes
    }

    #[test]
    fn parser_handles_multiline_calls_and_ignores_comments_and_strings() {
        let source = r#"
            fn configure_ecdsafail_submission_route() {
                // set_default_env("COMMENT_DEFAULT", "no");
                /* std::env::set_var("COMMENT_FORCED", "no"); */
                let _quoted = "std::env::remove_var(\"QUOTED_REMOVE\")";
                set_default_env(
                    "MULTI_DEFAULT",
                    "caller,keeps,this",
                );
                std::env::set_var(
                    "MULTI_FORCED",
                    "forced-value",
                );
                std::env::remove_var(
                    "MULTI_REMOVED",
                );
            }
        "#;

        let preset = parse_route_preset(source).expect("parse multiline route");

        assert_eq!(preset.entries.len(), 3);
        assert_eq!(
            preset.value_for(RouteEntryKind::DefaultIfAbsent, "MULTI_DEFAULT"),
            Some("caller,keeps,this")
        );
        assert_eq!(
            preset.value_for(RouteEntryKind::Forced, "MULTI_FORCED"),
            Some("forced-value")
        );
        assert!(preset.has_removed("MULTI_REMOVED"));
        assert!(!preset
            .entries
            .iter()
            .any(|entry| entry.name == "COMMENT_DEFAULT"));
        assert!(!preset
            .entries
            .iter()
            .any(|entry| entry.name == "COMMENT_FORCED"));
        assert!(!preset
            .entries
            .iter()
            .any(|entry| entry.name == "QUOTED_REMOVE"));
    }

    #[test]
    fn parser_rejects_malformed_route_assignments() {
        let missing_value = r#"
            fn configure_ecdsafail_submission_route() {
                set_default_env("BROKEN");
            }
        "#;
        let non_literal = r#"
            fn configure_ecdsafail_submission_route() {
                std::env::set_var("BROKEN", computed_value);
            }
        "#;
        let bad_name = r#"
            fn configure_ecdsafail_submission_route() {
                std::env::remove_var("not-an-env-name");
            }
        "#;

        assert!(parse_route_preset(missing_value).is_err());
        assert!(parse_route_preset(non_literal).is_err());
        assert!(parse_route_preset(bad_name).is_err());
    }

    #[test]
    fn parses_current_submission_route_contract() {
        let preset = current_preset();
        let declared_contract =
            parse_declared_adder_site_contract(ROUTE_PRESET_RS).expect("parse formal contract");

        assert!(preset.entries.len() > 100);
        assert_eq!(declared_contract.len(), 5);
        assert_eq!(
            preset.value_for(RouteEntryKind::Forced, "DIALOG_GCD_ACTIVE_ITERATIONS"),
            Some("258")
        );
        assert_eq!(
            preset.value_for(RouteEntryKind::Forced, "DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS"),
            Some("18")
        );
        assert_eq!(
            preset.value_for(RouteEntryKind::Forced, "SQUARE_ROW_MAX_SEG"),
            Some("158")
        );
        assert_eq!(
            preset.value_for(RouteEntryKind::Forced, "DIALOG_TAIL_NONCE"),
            Some("3452376")
        );
        assert_eq!(
            preset.value_for(
                RouteEntryKind::DefaultIfAbsent,
                "DIALOG_GCD_RAW_TOBITVECTOR_MATERIALIZED_SUB"
            ),
            Some("0")
        );
        assert!(!preset.entries.iter().any(|entry| entry.name == "1"));
        assert!(!preset
            .entries
            .iter()
            .any(|entry| entry.name.contains("safe lock")));
        assert!(preset.has_removed("DIALOG_GCD_APPLY_CHUNKED_F_CUTS"));
    }

    #[test]
    fn declared_formal_contract_matches_env_inference() {
        let preset = current_preset();
        let effective = preset.effective_env(&BTreeMap::new());
        let inferred = adder_site_contract_lines(&classify_adder_sites(&effective));
        let declared =
            parse_declared_adder_site_contract(ROUTE_PRESET_RS).expect("parse formal contract");

        assert_eq!(declared, inferred);
    }

    #[test]
    fn forced_pins_override_shell_env_but_defaults_do_not() {
        let preset = current_preset();
        let mut initial = BTreeMap::new();
        initial.insert("SKIP_ALT_SEED_CHECKS".to_string(), "0".to_string());
        initial.insert(
            "DIALOG_GCD_ACTIVE_ITERATIONS".to_string(),
            "999".to_string(),
        );
        initial.insert(
            "DIALOG_GCD_APPLY_CHUNKED_F_CUTS".to_string(),
            "1,2,3".to_string(),
        );

        let effective = preset.effective_env(&initial);

        assert_eq!(
            effective.get("SKIP_ALT_SEED_CHECKS").map(String::as_str),
            Some("0")
        );
        assert_eq!(
            effective
                .get("DIALOG_GCD_ACTIVE_ITERATIONS")
                .map(String::as_str),
            Some("258")
        );
        assert_eq!(
            effective
                .get("DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS")
                .map(String::as_str),
            Some("18")
        );
        assert!(!effective.contains_key("DIALOG_GCD_APPLY_CHUNKED_F_CUTS"));
    }

    #[test]
    fn route_dump_is_deterministic_and_contains_adder_keys() {
        let preset = current_preset();
        let left = render_route_dump(&preset, &BTreeMap::new());
        let right = render_route_dump(&preset, &BTreeMap::new());

        assert_eq!(left, right);
        assert!(left.contains("[solinas_cuccaro_adder_keys]"));
        assert!(left.contains(
            "point_add_phase=dialog_apply_special_fold SolinasCuccaroAdderKey=operation=fold"
        ));
        assert!(left.contains(
            "point_add_phase=round84_quotient_fold SolinasCuccaroAdderKey=operation=fold"
        ));
        assert!(left.contains("axis_kind="));
        assert!(left.contains("proof_obligations(mdd_node="));
        assert!(left.contains("[nonce_quality_key]\nnonce_quality_key=nqk-v1-"));
    }

    #[test]
    fn non_quality_irrelevant_env_does_not_change_nonce_key() {
        let preset = current_preset();
        let mut baseline_env = preset.effective_env(&BTreeMap::new());
        let baseline_key = nonce_key_for_env(&baseline_env);

        baseline_env.insert("TRACE_PEAK".to_string(), "1".to_string());
        let trace_key = nonce_key_for_env(&baseline_env);

        assert_eq!(baseline_key, trace_key);
    }

    #[test]
    fn nonce_quality_relevant_hazard_env_changes_nonce_key() {
        let preset = current_preset();
        let mut env = preset.effective_env(&BTreeMap::new());
        let baseline_key = nonce_key_for_env(&env);

        env.insert(
            "DIALOG_GCD_SPECIAL_OVERFLOW_CLEAN_STEP_BITS".to_string(),
            "113:20,131:20".to_string(),
        );
        let hazard_key = nonce_key_for_env(&env);

        assert_ne!(baseline_key, hazard_key);
    }

    #[test]
    fn patch_plan_report_lists_port_scope_and_remaining_duplicate_readers() {
        let preset = current_preset();
        let report = render_patch_plan_report(&preset);

        assert!(report.contains("[typed_preset_entries]"));
        assert!(report.contains("DIALOG_TAIL_NONCE=3452376"));
        assert!(report.contains("DIALOG_GCD_APPLY_CHUNKED_F_CUTS"));
        assert!(report.contains("[ported_shared_route_readers]"));
        assert!(report.contains("[remaining_duplicated_env_readers]"));
        assert!(report.contains("DIALOG_GCD_K5_HEAD11_CODEC"));
        assert!(!report
            .split("[remaining_duplicated_env_readers]\n")
            .nth(1)
            .unwrap()
            .split("[conservative_or_unknown_solinas_cuccaro_sites]")
            .next()
            .unwrap()
            .contains("DIALOG_GCD_K5_HEAD11_CODEC"));
        assert!(report.contains("src/point_add/route_config.rs"));
        assert!(report.contains("src/point_add/route_preset.rs"));
        assert!(report.contains("[conservative_or_unknown_solinas_cuccaro_sites]"));
    }

    #[test]
    fn ops_artifact_compare_reports_match_and_mismatch() {
        let left = unique_temp_path("left");
        let right = unique_temp_path("right");
        let third = unique_temp_path("third");
        fs::write(&left, ops_fixture(2, 0x11)).unwrap();
        fs::write(&right, ops_fixture(2, 0x11)).unwrap();
        fs::write(&third, ops_fixture(2, 0x12)).unwrap();

        let matched = compare_ops_artifacts(&left, &right).unwrap();
        assert!(matched.byte_equal);
        assert_eq!(matched.reference.op_count, Some(2));
        assert_eq!(matched.candidate.op_count, Some(2));
        assert!(matched.render().contains("status=match"));

        let mismatched = compare_ops_artifacts(&left, &third).unwrap();
        assert!(!mismatched.byte_equal);
        assert!(mismatched.render().contains("status=mismatch"));

        let _ = fs::remove_file(left);
        let _ = fs::remove_file(right);
        let _ = fs::remove_file(third);
    }
}
