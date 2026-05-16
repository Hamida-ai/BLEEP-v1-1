use bleep_governance::live_governance::{GovernableParam, GovernanceConfig, LiveGovernanceEngine, Vote};

#[test]
fn export_import_state_roundtrip() {
    let mut eng = LiveGovernanceEngine::new(GovernanceConfig::default(), 1_000);
    let pid = eng
        .submit(
            "alice",
            "Test Export",
            "Testing export/import",
            Some(GovernableParam::FeeBurnBps(2000)),
            eng.config.min_deposit,
        )
        .expect("submit");

    // cast a vote
    eng.vote(pid, "voter1", Vote::Yes, eng.config.total_staked / 10)
        .expect("vote");

    // export
    let json = eng.export_state().expect("export");

    // create new engine and import
    let mut eng2 = LiveGovernanceEngine::new(GovernanceConfig::default(), 1_000);
    eng2.import_state(&json).expect("import");

    let p1 = eng.proposal(pid).expect("orig proposal");
    let p2 = eng2.proposal(pid).expect("imported proposal");

    assert_eq!(p1.title, p2.title);
    assert_eq!(p1.param_change.as_ref().map(|p| p.name()), p2.param_change.as_ref().map(|p| p.name()));
    assert_eq!(p1.yes_votes, p2.yes_votes);
}
