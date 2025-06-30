use super::*;
use crate::primary;
use crate::error::{ConsensusError};
use crypto::generate_keypair;
use rand::rngs::StdRng;
use rand::SeedableRng as _;
use tokio::sync;


//CONSENSUS MESSAGE TESTS
//TODO: Add TC tests.

/*#[tokio::test]
async fn verify_valid_accept_vote() {
    assert!(accept_vote().verify(&committee()).is_ok());
}

#[tokio::test]
async fn verify_accept_vote_unknown_authority() {
    // Create Accept Vote with unknown authority.
    let mut rng = StdRng::from_seed([1; 32]);
    let (unknown_pub, unknown_priv) = generate_keypair(&mut rng);
   
    let accept_vote = AcceptVote::new_from_key(special_header().digest(), 1, 1, unknown_pub, &unknown_priv);

    // Verify the QC.
    match accept_vote.verify(&committee()) {
        Err(ConsensusError::UnknownAuthority(name)) => assert_eq!(name, unknown_pub),
        _ => assert!(false),
    }
}


#[tokio::test]
async fn verify_valid_qc() {
    assert!(qc().verify(&committee()).is_ok());
}

#[tokio::test]
async fn verify_valid_fast_qc() {
    assert!(fast_qc().verify(&committee()).is_ok());
}


#[tokio::test]
async fn verify_qc_authority_reuse() {
    // Modify QC to reuse one authority.
    let mut qc = qc();
    let _ = qc.votes.pop();
    let vote = qc.votes[0].clone();
    qc.votes.push(vote.clone());

    // Verify the QC.
    match qc.verify(&committee()) {
        Err(ConsensusError::AuthorityReuse(name)) => assert_eq!(name, vote.0),
        _ => assert!(false),
    }
}

#[tokio::test]
async fn verify_qc_unknown_authority() {
    let mut qc = qc();

    // Modify QC to add one unknown authority.
    let mut rng = StdRng::from_seed([1; 32]);
    let (unknown, _) = generate_keypair(&mut rng);
    let (_, sig) = qc.votes.pop().unwrap();
    qc.votes.push((unknown, sig));

    // Verify the QC.
    match qc.verify(&committee()) {
        Err(ConsensusError::UnknownAuthority(name)) => assert_eq!(name, unknown),
        _ => assert!(false),
    }
}

#[tokio::test]
async fn verify_qc_insufficient_stake() {
    // Modify QC to remove one authority.
    let mut qc = qc();
    let _ = qc.votes.pop();

    // Verify the QC.
    match qc.verify(&committee()) {
        Err(ConsensusError::QCRequiresQuorum) => assert!(true),
        _ => assert!(false),
    }
}*/
