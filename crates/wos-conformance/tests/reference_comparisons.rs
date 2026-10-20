// Rust guideline compliant 2026-02-21
//! Executes declared native members rather than supplying aggregate observations.
use base64::Engine;
use integrity_canonical::{DigestAlgorithm, compute_digest};
use serde_json::{Value, json};
use wos_conformance::studio_api::{
    PairedApprovalRateRequest, compare_paired_approval_rates, paired_metric_descriptor,
};

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{}", compute_digest(bytes, DigestAlgorithm::Sha256))
}
fn request(biased: usize) -> PairedApprovalRateRequest {
    let workflow = json!({"$wosWorkflow":"1.0","url":"urn:test:cohort","version":"1.0.0","title":"Paired native control","impactLevel":"operational","actors":[{"id":"worker","type":"human"}],"lifecycle":{"initialState":"start","states":{
      "start":{"type":"atomic","transitions":[{"event":"decide","guard":"caseFile.approve","target":"favorable"},{"event":"decide","guard":"caseFile.bias","target":"bias"},{"event":"decide","target":"adverse"}]},
      "bias":{"type":"atomic","transitions":[{"event":"finish","guard":"caseFile.category = 'left'","target":"favorable"},{"event":"finish","target":"adverse"}]},"favorable":{"type":"final"},"adverse":{"type":"final"}}}});
    let source = b"Illustrative software-fixture policy: strict five percentage points.";
    let review=b"Explicit independent engineering review of the paired inputs and selected strict threshold; no population or legal claim.";
    let evidence = |id: &str, role: &str, bytes: &[u8]| json!({"id":id,"role":role,"sha256":hash(bytes),"bytesBase64":base64::engine::general_purpose::STANDARD.encode(bytes)});
    let adapters = json!({"scope":"softwareFixture","humanActorBindings":[],"trustedReceiptSigners":[],"noticeTransport":{"id":"fixture-transport","scope":"softwareFixture","mode":"sameInstantQualifiedReceipt","retainedResultsIdentity":hash(b"isolated-cohort-fixture-store"),"failureSchedule":[]},"retainedEvidence":[evidence("source","source",source),evidence("review","review",review)],"templates":[],"documentSigningKeys":[],"fixtureSigningCapabilities":[],"invocations":[]});
    let pairs:Vec<_>=(0..100).map(|index|{
        let member=|side:&str| {
            let fixture=json!({"id":format!("pair-{index}"),"rule":"paired-control","description":"Actual matched native member","initial_case_state":{"approve":index<40,"bias":index>=40 && index<40+biased,"category":side},"event_sequence":[{"event":"decide","actor":"worker"},{"event":"finish","actor":"worker"}]});
            json!({"id":format!("{side}-{index}"),"request":{"candidateBytes":serde_json::to_vec(&workflow).unwrap(),"dependencies":[],"fixtureJson":fixture.to_string(),"clockStartMs":0,"requiredObservations":["configuration"],"expectations":[]},"adapters":adapters})
        };
        json!({"left":member("left"),"right":member("right")})
    }).collect();
    let mut request:PairedApprovalRateRequest=serde_json::from_value(json!({"pairs":pairs,"protectedCasePointer":"/category","leftValue":"left","rightValue":"right","thresholdPercentagePoints":5,"sourceRefs":[{"sourceVersion":"source","sectionAnchor":"fixture-threshold","retainedSha256":hash(source)}],"review":{"id":"review","sha256":hash(review)},"favorableStatePointer":"/lifecycle/states/favorable","adverseStatePointer":"/lifecycle/states/adverse"})).unwrap();
    review_metric(&mut request);
    request
}
fn review_metric(request: &mut PairedApprovalRateRequest) {
    let descriptor = paired_metric_descriptor(request).unwrap();
    let bytes=serde_json::to_vec(&json!({"scope":"softwareFixture","decision":"accepted","reviewer":{"kind":"model","id":"synthetic-comparison-test-reviewer","method":"Explicit synthetic test review of known authored200member inputs and strict source threshold"},"reviewedAt":"2026-10-19T00:00:00Z","rationale":"Synthetic software test explicitly reviews paired construction, threshold and selected actual terminal ownership; no legal/population claim.","descriptor":descriptor})).unwrap();
    request.review.sha256 = hash(&bytes);
    for pair in &mut request.pairs {
        for member in [&mut pair.left, &mut pair.right] {
            let evidence = member
                .adapters
                .retained_evidence
                .iter_mut()
                .find(|item| item.id == "review")
                .unwrap();
            evidence.sha256 = hash(&bytes);
            evidence.bytes_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        }
    }
}
fn record(name: &str, value: &impl serde::Serialize) {
    if let Ok(directory) = std::env::var("NATIVE_COMPARISON_EVIDENCE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn actual_two_hundred_members_preserve_strict_threshold_and_full_results() {
    let four = compare_paired_approval_rates(&request(4));
    record("four-pass", &four);
    assert_eq!(four.status, "pass", "{:?}", four.refusals);
    assert_eq!(
        (four.approved_left, four.approved_right),
        (Some(44), Some(40))
    );
    assert_eq!(four.members.len(), 200);
    assert!(four.members.iter().all(|member| {
        member
            .result
            .as_ref()
            .is_some_and(|result| !result.attempts.is_empty())
    }));
    let five = compare_paired_approval_rates(&request(5));
    record("five-fail", &five);
    assert_eq!(five.status, "fail", "{:?}", five.refusals);
    assert_eq!(
        (five.approved_left, five.approved_right),
        (Some(45), Some(40))
    );
    assert_eq!(five.members.len(), 200);
}
#[test]
fn missing_duplicate_or_nonpaired_members_refuse_before_metric() {
    let mut missing = request(0);
    missing.pairs.pop();
    assert_eq!(compare_paired_approval_rates(&missing).status, "refused");
    let mut duplicate = request(0);
    duplicate.pairs[99].right.id = duplicate.pairs[0].left.id.clone();
    assert_eq!(compare_paired_approval_rates(&duplicate).status, "refused");
    let mut changed = request(0);
    let mut fixture: Value =
        serde_json::from_str(&changed.pairs[0].right.request.fixture_json).unwrap();
    fixture["initial_case_state"]["approve"] = json!(false);
    changed.pairs[0].right.request.fixture_json = fixture.to_string();
    assert_eq!(compare_paired_approval_rates(&changed).status, "refused");
    let mut source = request(0);
    source.source_refs[0].retained_sha256 = "0".repeat(64);
    assert_eq!(compare_paired_approval_rates(&source).status, "refused");
}
#[test]
fn incomplete_or_failed_actual_members_never_count_as_adverse() {
    let mut incomplete = request(0);
    for pair in &mut incomplete.pairs {
        for member in [&mut pair.left, &mut pair.right] {
            let mut fixture: Value = serde_json::from_str(&member.request.fixture_json).unwrap();
            fixture["event_sequence"] = json!([{"event":"no-op","actor":"worker"}]);
            member.request.fixture_json = fixture.to_string();
        }
    }
    review_metric(&mut incomplete);
    let result = compare_paired_approval_rates(&incomplete);
    record("incomplete-refused", &result);
    assert_eq!(result.status, "refused");
    assert_eq!(result.members.len(), 200);
    assert_eq!(result.approved_left, None);
    assert_eq!(result.approved_right, None);
    assert!(
        result
            .members
            .iter()
            .all(|member| member.result.is_some() && member.classification.is_none())
    );
    let mut failure = request(0);
    for pair in &mut failure.pairs {
        for member in [&mut pair.left, &mut pair.right] {
            member.request.expectations.push(
                serde_json::from_value(
                    json!({"pointer":"/finalSnapshot/caseState/approve","expected":"wrong type"}),
                )
                .unwrap(),
            );
        }
    }
    review_metric(&mut failure);
    let result = compare_paired_approval_rates(&failure);
    record("failed-observation-refused", &result);
    assert_eq!(result.status, "refused");
    assert_eq!(result.members.len(), 200);
    assert_eq!(result.approved_left, None);
}

#[test]
fn existing_review_cannot_accept_changed_threshold_outcome_or_member_inputs() {
    let mut threshold = request(4);
    threshold.threshold_percentage_points = 6;
    assert_eq!(compare_paired_approval_rates(&threshold).status, "refused");
    let mut outcome = request(4);
    std::mem::swap(
        &mut outcome.favorable_state_pointer,
        &mut outcome.adverse_state_pointer,
    );
    assert_eq!(compare_paired_approval_rates(&outcome).status, "refused");
    let mut changed = request(4);
    let pair = &mut changed.pairs[0];
    for member in [&mut pair.left, &mut pair.right] {
        member.request.clock_start_ms = 1;
    }
    assert_eq!(compare_paired_approval_rates(&changed).status, "refused");
}

fn signed_reference() -> (
    wos_conformance::studio_api::ReplayRequest,
    wos_conformance::studio_api::ReferenceReplayAdapters,
) {
    let input: Value =
        serde_json::from_str(include_str!("fixtures/reference-due-process-replay.json")).unwrap();
    (
        serde_json::from_value(input["request"].clone()).unwrap(),
        serde_json::from_value(input["adapters"].clone()).unwrap(),
    )
}
fn review_actual_notice(
    session: &wos_conformance::studio_api::NativeReplaySession,
) -> wos_conformance::studio_api::RenderedNoticeReviewAssertion {
    use wos_conformance::studio_api::*;
    let selector:NativeSelector=serde_json::from_value(json!({"attempt":{"eventIndex":2,"kind":"governanceOperation","occurrence":0},"subject":{"kind":"notice","target":{"bindingRef":"adverse","firingOccurrence":0}}})).unwrap();
    let NativeResolution::Resolved { pointer } = resolve_observation(session.result(), &selector)
    else {
        panic!("actual signed notice must resolve")
    };
    let actual = serde_json::to_value(session.result()).unwrap();
    let notice = actual.pointer(&pointer).unwrap();
    let variants: Vec<ReviewedNoticeVariant> = notice["variants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| ReviewedNoticeVariant {
            locale: variant["locale"].as_str().unwrap().into(),
            body_sha256: variant["bodySha256"].as_str().unwrap().into(),
        })
        .collect();
    assert!(
        notice["variants"]
            .as_array()
            .unwrap()
            .iter()
            .all(|variant| variant["body"]
                .as_str()
                .is_some_and(|body| !body.is_empty()))
    );
    let review = json!({"scope":"engineeringFixture","reviewer":{"kind":"model","id":"synthetic-native-test-reviewer","method":"Explicit synthetic engineering review of immutable actual test fixture content; not legal, human or accessibility certification"},"reviewedAt":"2026-10-19T00:00:00Z","artifactSha256":notice["artifactSha256"],"variants":variants,"claims":["plainLanguageEngineeringReview"],"observations":{"plainLanguageEngineeringReview":{"verdict":"supported","observations":"The actual rendered synthetic fixture body is nonempty and inspected by this explicit test fixture review. No substantive human judgment is claimed."}},"limitations":["Human language equivalence, Section508, FNS approval, legal sufficiency and publication approval remain unqualified."]});
    let review_bytes = serde_json::to_vec(&review).unwrap();
    RenderedNoticeReviewAssertion {
        selector,
        artifact_sha256: notice["artifactSha256"].as_str().unwrap().into(),
        variants,
        required_claims: vec![EngineeringClaim::PlainLanguageEngineeringReview],
        review_sha256: hash(&review_bytes),
        review_bytes,
        comparison_clock_ms: 1792368000000,
    }
}
#[test]
fn actual_signed_notice_review_is_bound_to_same_run_bytes_and_explicit_attribution() {
    use wos_conformance::studio_api::*;
    let (request, adapters) = signed_reference();
    let prepared = prepare_reference_replay(request.clone(), adapters.clone()).unwrap();
    let session = begin_reference_replay(prepared.clone()).unwrap();
    assert!(session.result().passed);
    let assertion = review_actual_notice(&session);
    let pass = compare_rendered_notice(&session, &prepared, &assertion);
    record("same-session-review-pass", &pass);
    assert_eq!(pass.status, "pass", "{:?}", pass.refusals);
    assert_eq!(pass.scope, "engineeringFixture");
    assert_eq!(pass.reviewer.unwrap().kind, "model");
    assert!(pass.unqualified_claims.contains(&"section508Review".into()));
    let mut changed = request;
    changed.clock_start_ms += 1;
    let current = prepare_reference_replay(changed, adapters).unwrap();
    assert_eq!(
        compare_rendered_notice(&session, &current, &assertion).status,
        "refused"
    );
    for (name, mutated) in [
        ("artifact", {
            let mut value = assertion.clone();
            value.artifact_sha256 = "sha256:".to_string() + &"0".repeat(64);
            value
        }),
        ("variant", {
            let mut value = assertion.clone();
            value.variants[0].body_sha256 = "sha256:".to_string() + &"0".repeat(64);
            value
        }),
        ("missing-variant", {
            let mut value = assertion.clone();
            value.variants.clear();
            value
        }),
        ("review-bytes", {
            let mut value = assertion.clone();
            value.review_bytes.push(b' ');
            value
        }),
        ("comparison-clock", {
            let mut value = assertion.clone();
            value.comparison_clock_ms = 0;
            value
        }),
    ] {
        let result = compare_rendered_notice(&session, &prepared, &mutated);
        record(name, &result);
        assert_eq!(result.status, "refused", "{name}");
    }
    for (pointer, value, expected) in [
        (
            "/observations/plainLanguageEngineeringReview/verdict",
            json!("refuted"),
            "fail",
        ),
        (
            "/observations/plainLanguageEngineeringReview/verdict",
            json!("inconclusive"),
            "refused",
        ),
        ("/reviewedAt", json!("1969-12-31T23:59:59Z"), "refused"),
        ("/reviewedAt", json!("2027-01-01T00:00:00Z"), "refused"),
        ("/reviewer/kind", json!("human"), "refused"),
        ("/reviewedAt", json!("2026-10-19T00:00:00+00:00"), "refused"),
        ("/reviewedAt", json!("2026-10-19T00:00:00.000Z"), "refused"),
        ("/reviewedAt", json!("2026-10-19T00:00:00.0001Z"), "refused"),
        ("/limitations", json!([]), "refused"),
        ("/claims", json!([]), "refused"),
    ] {
        let mut mutated = assertion.clone();
        let mut review: Value = serde_json::from_slice(&mutated.review_bytes).unwrap();
        *review.pointer_mut(pointer).unwrap() = value;
        mutated.review_bytes = serde_json::to_vec(&review).unwrap();
        mutated.review_sha256 = hash(&mutated.review_bytes);
        assert_eq!(
            compare_rendered_notice(&session, &prepared, &mutated).status,
            expected,
            "{pointer}"
        );
    }
    let another = begin_reference_replay(prepared.clone()).unwrap();
    let second = review_actual_notice(&another);
    assert_ne!(
        assertion.artifact_sha256, second.artifact_sha256,
        "actual native generated linkage distinguishes these artifacts"
    );
    assert_eq!(
        compare_rendered_notice(&another, &prepared, &assertion).status,
        "refused"
    );
}
