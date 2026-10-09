use guardengine::integration::attempt_store::*;
fn digest(c: char) -> String {
    format!("sha256:{}", c.to_string().repeat(64))
}
fn target(id: &str) -> Target {
    Target {
        repo_id: "repo".into(),
        task_id: "task".into(),
        requirement_ids: vec![id.into()],
    }
}
fn record(id: &str, target: Target, generation: u64, eligible: bool) -> AttemptRecord {
    AttemptRecord {
        run_id: id.into(),
        target,
        generation,
        content_digest: digest('a'),
        envelope_digest: digest('b'),
        eligible,
    }
}
#[test]
fn late_pass_does_not_replace_current_block() {
    let mut s = InMemoryAttemptStore::default();
    let t = target("R1");
    assert_eq!(s.advance(t.clone(), 0, digest('a'), "old".into()), Ok(1));
    assert_eq!(s.advance(t.clone(), 1, digest('a'), "new".into()), Ok(2));
    s.append(record("new", t.clone(), 2, false)).unwrap();
    s.publish("new").unwrap();
    s.append(record("old", t.clone(), 1, true)).unwrap();
    assert_eq!(s.publish("old"), Err(StoreError::Stale));
    assert!(!s.current(&t).unwrap().eligible);
    assert_eq!(s.history(&t).len(), 2);
    assert_eq!(
        s.advance(t.clone(), 2, digest('a'), "recheck".into()),
        Ok(3)
    );
    assert!(s.current(&t).is_none());
    assert_eq!(s.publish("new"), Err(StoreError::Stale));
}
#[test]
fn targets_isolate_requirements_repo_and_task() {
    let mut s = InMemoryAttemptStore::default();
    let mut targets = [target("R1"), target("R2"), target("R1"), target("R1")];
    targets[2].repo_id = "other".into();
    targets[3].task_id = "other".into();
    for (i, t) in targets.iter().enumerate() {
        let id = format!("run-{i}");
        s.advance(t.clone(), 0, digest('a'), id.clone()).unwrap();
        s.append(record(&id, t.clone(), 1, i == 0)).unwrap();
        s.publish(&id).unwrap();
    }
    for (i, t) in targets.iter().enumerate() {
        assert_eq!(s.current(t).unwrap().eligible, i == 0);
        assert_eq!(s.history(t).len(), 1);
    }
}
#[test]
fn replay_is_idempotent_but_collision_and_unregistered_records_reject() {
    let mut s = InMemoryAttemptStore::default();
    let t = target("R1");
    let r = record("one", t.clone(), 1, true);
    assert_eq!(s.append(r.clone()), Err(StoreError::UnknownAttempt));
    s.advance(t.clone(), 0, digest('a'), "one".into()).unwrap();
    assert_eq!(s.append(r.clone()), Ok(AppendOutcome::Inserted));
    assert_eq!(s.append(r.clone()), Ok(AppendOutcome::IdenticalReplay));
    s.publish("one").unwrap();
    s.publish("one").unwrap();
    let mut conflict = r.clone();
    conflict.eligible = false;
    assert_eq!(s.append(conflict), Err(StoreError::Conflict));
    assert_eq!(
        s.advance(target("R2"), 0, digest('a'), "one".into()),
        Err(StoreError::Conflict)
    );
    assert_eq!(
        s.advance(t.clone(), 0, digest('a'), "two".into()),
        Err(StoreError::Stale)
    );
    assert_eq!(s.history(&t), vec![&r]);
}
#[test]
fn cas_binds_registered_content_target_and_generation() {
    for variant in 0..3 {
        let mut s = InMemoryAttemptStore::default();
        let t = target("R1");
        s.advance(t.clone(), 0, digest('a'), "one".into()).unwrap();
        let mut r = record("one", t, 1, true);
        match variant {
            0 => r.target = target("R2"),
            1 => r.content_digest = digest('c'),
            _ => r.generation = 2,
        };
        assert_eq!(s.append(r), Err(StoreError::Conflict));
    }
    let mut s = InMemoryAttemptStore::default();
    let mut t = target("R1");
    t.requirement_ids = vec!["R2".into(), "R1".into()];
    assert_eq!(
        s.advance(t, 0, digest('a'), "one".into()),
        Err(StoreError::Invalid)
    );
    assert_eq!(
        s.advance(target("R1"), 0, "invalid".into(), "one".into()),
        Err(StoreError::Invalid)
    );
    assert_eq!(s.publish("missing"), Err(StoreError::UnknownAttempt));
}
