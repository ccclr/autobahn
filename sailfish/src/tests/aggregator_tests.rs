use super::*;
use crate::common::{committee, keys};

/*#[test]
fn add_vote() {
    let mut aggregator = Aggregator::new(committee());
    let result = aggregator.add_accept_vote(accept_vote());
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn make_qc() {
    let mut aggregator = Aggregator::new(committee());
    let mut keys = keys();
    let qc = qc();
    let hash = qc.digest();
    let view = qc.view;
    let round = qc.view_round;

    // Add 2f+1 votes to the aggregator and ensure it returns the cryptographic
    // material to make a valid QC.
    let (public_key, secret_key) = keys.pop().unwrap();
    let vote = AcceptVote::new_from_key(hash.clone(), view, round, public_key, &secret_key);
    let result = aggregator.add_accept_vote(vote.clone());

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let (public_key, secret_key) = keys.pop().unwrap();
    let vote = AcceptVote::new_from_key(hash.clone(), view, round, public_key, &secret_key);
    let result = aggregator.add_accept_vote(vote.clone());

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let (public_key, secret_key) = keys.pop().unwrap();
    let vote = AcceptVote::new_from_key(hash.clone(), view, round, public_key, &secret_key);

    match aggregator.add_accept_vote(vote) {
        Ok(Some(qc)) => assert!(qc.verify(&committee()).is_ok()),
        _ => assert!(false),
    }
}

#[test]
fn cleanup() {
    let mut aggregator = Aggregator::new(committee());

    // Add a vote and ensure it is in the aggregator memory.
    let result = aggregator.add_accept_vote(accept_vote());
    assert!(result.is_ok());
    assert_eq!(aggregator.votes_aggregators.len(), 1);
    assert!(aggregator.timeouts_aggregators.is_empty());

    // Clean up the aggregator.
    aggregator.cleanup(&2);
    assert!(aggregator.votes_aggregators.is_empty());
    assert!(aggregator.timeouts_aggregators.is_empty());
}*/
