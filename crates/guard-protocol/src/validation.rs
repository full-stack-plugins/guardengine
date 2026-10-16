use crate::{
    Action, ApprovalIntent, Coverage, Decision, ExecutionStatus, FactType, ObjectFormat,
    ProtocolDocument, SubjectKind, SubjectSnapshot, Verdict,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::BTreeSet;

pub(crate) fn valid_utc(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20
        || b.len() > 30
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[b.len() - 1] != b'Z'
    {
        return false;
    }
    for (n, c) in b.iter().take(19).enumerate() {
        if !matches!(n, 4 | 7 | 10 | 13 | 16) && !c.is_ascii_digit() {
            return false;
        }
    }
    if b.len() > 20
        && (b[19] != b'.' || b.len() < 22 || !b[20..b.len() - 1].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    let number = |start, end| {
        s.get(start..end)
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let (year, month, day) = (number(0, 4), number(5, 7), number(8, 10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    year > 0
        && day > 0
        && day <= max
        && number(11, 13) <= 23
        && number(14, 16) <= 59
        && number(17, 19) <= 59
}
fn time_key(s: &str) -> String {
    let mut key = s[..19].to_owned();
    let fraction = if s.len() > 20 {
        &s[20..s.len() - 1]
    } else {
        ""
    };
    key.push_str(fraction);
    for _ in fraction.len()..9 {
        key.push('0');
    }
    key
}
pub(crate) fn compare_time(
    left: &crate::UtcTimestamp,
    right: &crate::UtcTimestamp,
) -> std::cmp::Ordering {
    time_key(left.as_str()).cmp(&time_key(right.as_str()))
}
fn period(start: &crate::UtcTimestamp, end: &crate::UtcTimestamp) -> Result<(), String> {
    require(
        compare_time(start, end).is_lt(),
        "time window must have positive duration",
    )
}
fn require(ok: bool, reason: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(reason.into()) }
}
fn unique<'a>(values: impl IntoIterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        require(seen.insert(value), label)?;
    }
    Ok(())
}
fn rule(v: &crate::RuleDefinition) -> Result<(), String> {
    require(
        v.waiver_policy.max_age_seconds <= 9007199254740991,
        "waiver lifetime exceeds safe integer range",
    )?;
    unique(
        v.required_evidence.iter().map(|s| s.as_str()),
        "duplicate required evidence",
    )?;
    require(
        !v.waiver_policy.allowed
            || (v.waiver_policy.required_role.is_some() && v.waiver_policy.max_age_seconds > 0),
        "waiver requires role and positive lifetime",
    )
}
fn obligations(v: &[crate::Obligation]) -> Result<(), String> {
    require(!v.is_empty(), "obligations must not be empty")?;
    unique(
        v.iter().map(|o| o.obligation_id.as_str()),
        "duplicate obligation id",
    )?;
    for o in v {
        require(
            !o.targets.is_empty(),
            "obligation requires exact target set",
        )?;
        target_set(&o.targets)?;
        unique(
            o.required_evidence.iter().map(|s| s.as_str()),
            "duplicate required evidence",
        )?;
    }
    Ok(())
}
fn contract_obligations(values: &[crate::ContractObligation]) -> Result<(), String> {
    require(!values.is_empty(), "contract obligations must not be empty")?;
    unique(
        values.iter().map(|o| o.obligation_id.as_str()),
        "duplicate obligation id",
    )?;
    for obligation in values {
        require(
            !obligation.targets.is_empty(),
            "contract obligation requires target selections",
        )?;
        let mut targets = BTreeSet::new();
        for target in &obligation.targets {
            // 同一选择范围不能通过不同 protectedDigest 重复出现，避免绑定含糊。
            let identity =
                serde_json::to_vec(&(&target.repository_id, &target.path, &target.selector))
                    .map_err(|e| e.to_string())?;
            require(
                targets.insert(identity),
                "duplicate contract target selection",
            )?;
        }
        unique(
            obligation.required_evidence.iter().map(|e| e.as_str()),
            "duplicate required evidence",
        )?;
    }
    Ok(())
}
fn target_set(targets: &[crate::ExactTarget]) -> Result<BTreeSet<Vec<u8>>, String> {
    let mut set = BTreeSet::new();
    for target in targets {
        let bytes = serde_json::to_vec(target).map_err(|e| e.to_string())?;
        require(set.insert(bytes), "duplicate exact target")?;
    }
    Ok(set)
}
fn span(s: &crate::SourceSpan) -> Result<(), String> {
    require(
        s.start_line > 0
            && s.start_column > 0
            && s.end_line > 0
            && s.end_column > 0
            && (s.end_line, s.end_column) >= (s.start_line, s.start_column),
        "invalid source span",
    )
}
fn proof(p: &crate::AuthorityProof) -> Result<(), String> {
    let value = STANDARD
        .decode(p.signature.as_str())
        .map_err(|_| "invalid base64 signature")?;
    require(
        value.len() == 64 && STANDARD.encode(&value) == p.signature.as_str(),
        "Ed25519 signature must be canonical 64 bytes",
    )
}
pub(crate) fn validate(doc: &ProtocolDocument) -> Result<(), String> {
    match doc {
        ProtocolDocument::RuleDefinition(v) => rule(v)?,
        ProtocolDocument::ContractSnapshot(v) => {
            require(
                !v.rules.is_empty() && !v.source_refs.is_empty(),
                "contract needs rules and source refs",
            )?;
            unique(
                v.rules.iter().map(|r| r.rule_id.as_str()),
                "duplicate rule id",
            )?;
            for r in &v.rules {
                rule(r)?;
            }
            contract_obligations(&v.obligations)?;
            let ids = v
                .rules
                .iter()
                .map(|r| r.rule_id.as_str())
                .collect::<BTreeSet<_>>();
            require(
                v.obligations
                    .iter()
                    .all(|o| ids.contains(o.rule_id.as_str())),
                "obligation references unknown rule",
            )?;
        }
        ProtocolDocument::TaskBinding(v) => {
            require(
                !v.requirement_refs.is_empty()
                    && !v.scope.repository_ids.is_empty()
                    && !v.scope.actions.is_empty(),
                "task requires requirement and scope",
            )?;
            unique(
                v.scope.repository_ids.iter().map(|s| s.as_str()),
                "duplicate scope repository",
            )?;
            unique(
                v.depends_on.iter().map(|s| s.as_str()),
                "duplicate dependency task",
            )?;
            require(
                !v.depends_on.contains(&v.task_id),
                "task cannot depend on itself",
            )?;
        }
        ProtocolDocument::SubjectSnapshot(subject) => match subject {
            SubjectSnapshot::Git(v) => {
                let oid_len = if v.object_format == ObjectFormat::Sha1 {
                    40
                } else {
                    64
                };
                require(
                    [&v.base, &v.head, &v.tree, &v.expected_target_oid]
                        .into_iter()
                        .flatten()
                        .all(|s| s.as_str().len() == oid_len),
                    "Git OID length disagrees with object format",
                )?;
                match v.kind {
                    SubjectKind::Worktree => require(
                        v.worktree_id.is_some() && v.diff_digest.is_some(),
                        "worktree needs worktreeId and diffDigest",
                    )?,
                    SubjectKind::Index => require(
                        v.worktree_id.is_some()
                            && v.head.is_some()
                            && v.tree.is_some()
                            && v.diff_digest.is_some(),
                        "index needs worktreeId head tree diffDigest",
                    )?,
                    SubjectKind::Commit => require(
                        v.head.is_some() && v.tree.is_some(),
                        "commit needs head and tree",
                    )?,
                    SubjectKind::MergeCandidate => require(
                        v.base.is_some()
                            && v.head.is_some()
                            && v.tree.is_some()
                            && v.target_ref.is_some()
                            && v.expected_target_oid.is_some()
                            && v.diff_digest.is_some(),
                        "merge candidate needs base head tree target expectedTargetOid diffDigest",
                    )?,
                }
            }
            SubjectSnapshot::ReleaseArtifact(v) => {
                let oid_len = if v.source.object_format == ObjectFormat::Sha1 {
                    40
                } else {
                    64
                };
                require(
                    v.source.commit.as_str().len() == oid_len
                        && v.source.tree.as_str().len() == oid_len,
                    "release source OIDs disagree with object format",
                )?;
                require(
                    v.artifact.size <= 9007199254740991,
                    "release artifact size must be a safe integer",
                )?;
            }
        },
        ProtocolDocument::ProducerIdentity(_) => {}
        ProtocolDocument::VerificationPlan(v) => {
            obligations(&v.obligations)?;
            require(
                v.budget.timeout_ms > 0
                    && v.budget.timeout_ms <= 9007199254740991
                    && v.budget.max_output_bytes > 0
                    && v.budget.max_output_bytes <= 9007199254740991
                    && v.budget.max_concurrency > 0,
                "plan budget must be positive",
            )?;
            let expected = target_set(&v.targets)?;
            let union = v
                .obligations
                .iter()
                .flat_map(|o| o.targets.iter())
                .map(serde_json::to_vec)
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(|e| e.to_string())?;
            require(
                expected == union,
                "plan target set must equal frozen obligation target union",
            )?;
        }
        ProtocolDocument::FactEnvelope(v) => {
            let flags = (
                v.symbol.is_some(),
                v.edge.is_some(),
                v.source_span.is_some(),
            );
            let correct = match v.fact_type {
                FactType::Symbol => (true, false, false),
                FactType::Edge => (false, true, false),
                FactType::SourceSpan => (false, false, true),
            };
            require(flags == correct, "fact payload disagrees with factType")?;
            if let Some(s) = &v.symbol {
                span(&s.source_span)?;
            }
            if let Some(e) = &v.edge {
                span(&e.from.source_span)?;
                span(&e.to.source_span)?;
            }
            if let Some(s) = &v.source_span {
                span(s)?;
            }
        }
        ProtocolDocument::GuardResult(v) => {
            let e = &v.execution;
            let a = &v.assessment;
            if let (Some(start), Some(end)) = (&e.started_at, &e.finished_at) {
                require(!compare_time(start, end).is_gt(), "execution time reverses")?;
            }
            if e.status == ExecutionStatus::Completed {
                require(
                    e.started_at.is_some() && e.finished_at.is_some() && e.exit_code.is_some(),
                    "completed execution needs times and exit code",
                )?;
            }
            if e.status == ExecutionStatus::NotRun {
                require(
                    e.exit_code.is_none() && e.started_at.is_none() && e.finished_at.is_none(),
                    "not-run execution cannot claim run metadata",
                )?;
            }
            if a.verdict == Verdict::Satisfied {
                require(
                    e.status == ExecutionStatus::Completed
                        && a.coverage == Coverage::Complete
                        && a.finding_ids.is_empty(),
                    "satisfied requires complete execution and coverage without findings",
                )?;
            }
            if a.verdict == Verdict::Violation {
                require(!a.finding_ids.is_empty(), "violation must retain findings")?;
            }
            if a.verdict == Verdict::NotApplicable {
                require(
                    !a.applicability_evidence_refs.is_empty(),
                    "not-applicable requires applicability evidence",
                )?;
            }
            require(
                v.evidence_refs.iter().all(|e| e.size <= 9007199254740991),
                "evidence size exceeds safe integer range",
            )?;
            unique(
                v.evidence_refs.iter().map(|e| e.digest.as_str()),
                "duplicate evidence ref",
            )?;
        }
        ProtocolDocument::GateDecision(v) => {
            period(&v.evaluated_at, &v.expires_at)?;
            if matches!(v.decision, Decision::Allow | Decision::AllowWithExceptions) {
                require(
                    v.missing_obligations.is_empty() && !v.result_digests.is_empty(),
                    "allow requires results and no missing obligations",
                )?;
                if v.decision == Decision::Allow {
                    require(
                        v.active_violations.is_empty(),
                        "allow cannot erase active violations",
                    )?;
                } else {
                    require(
                        !v.approval_refs.is_empty(),
                        "exceptions require approval references",
                    )?;
                }
            }
            unique(
                v.result_digests.iter().map(|d| d.as_str()),
                "duplicate result digest",
            )?;
        }
        ProtocolDocument::ApprovalReceipt(v) => {
            period(&v.issued_at, &v.expires_at)?;
            proof(&v.proof)?;
            require(
                v.intent == v.subject_binding.kind
                    && v.scope_digest == v.subject_binding.scope_digest,
                "approval binding disagrees with intent or scope",
            )?;
            require(
                v.intent == ApprovalIntent::RequirementScope
                    || v.subject_binding.subject_digest.is_some(),
                "candidate acceptance and operation approval must bind subject",
            )?;
        }
        ProtocolDocument::ExecutionGrant(v) => {
            period(&v.issued_at, &v.expires_at)?;
            proof(&v.issuer_proof)?;
            require(
                v.action == Action::BranchCreate || v.expected_target_oid.is_some(),
                "grant requires expected target OID except branch creation",
            )?;
        }
    }
    Ok(())
}

/// 协议整数字面量保留精确身份，浮点参数使用 RFC8785 的 IEEE754 解释。
pub(crate) fn safe_integer_literals(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Number(n) => {
            if n.as_i64()
                .is_some_and(|n| !(-9007199254740991..=9007199254740991).contains(&n))
                || n.as_u64().is_some_and(|n| n > 9007199254740991)
            {
                return Err("unsafe integer identity; encode as a string".into());
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                safe_integer_literals(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                safe_integer_literals(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}
