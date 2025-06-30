#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use crate::{DagError, Height};
// Copyright(C) Facebook, Inc. and its affiliates.
use crate::error::DagResult;
use crate::header_waiter::WaiterMessage;
use crate::messages::{Certificate, ConsensusMessage, Header, Proposal};
use config::Committee;
use crypto::Hash as _;
use crypto::{Digest, PublicKey};
use log::debug;
use std::collections::HashMap;
use store::Store;
use tokio::sync::mpsc::{Receiver, Sender};

/// The `Synchronizer` checks if we have all batches and parents referenced by a header. If we don't, it sends
/// a command to the `Waiter` to request the missing data.
#[derive(Clone)]
pub struct Synchronizer {
    /// The public key of this primary.
    name: PublicKey,
    /// The persistent storage.
    store: Store,
    /// Send commands to the `HeaderWaiter`.
    tx_header_waiter: Sender<WaiterMessage>,
    /// Send commands to the `CertificateWaiter`.
    tx_certificate_waiter: Sender<Certificate>,
    /// Genesis header
    genesis_headers: HashMap<PublicKey, Header>,
    // Keeps track of the latest heights for each lane, which is necessary for fast sync
    last_fast_sync_heights: HashMap<PublicKey, Height>,
    // Whether to use fast sync
    use_fast_sync: bool,
}

impl Synchronizer {
    pub fn new(
        name: PublicKey,
        committee: &Committee,
        store: Store,
        tx_header_waiter: Sender<WaiterMessage>,
        tx_certificate_waiter: Sender<Certificate>,
        use_fast_sync: bool,
    ) -> Self {
        Self {
            name,
            store,
            tx_header_waiter,
            tx_certificate_waiter,
            genesis_headers: Header::genesis_headers(committee),
            last_fast_sync_heights: committee.authorities.keys().map(|x| (*x, 1)).collect(),
            use_fast_sync,
        }
    }

    /// Returns `true` if we have all transactions of the payload. If we don't, we return false,
    /// synchronize with other nodes (through our workers), and re-schedule processing of the
    /// header for when we will have its complete payload.
    pub async fn missing_payload(&mut self, header: &Header, force_sync: bool) -> DagResult<bool> {
        // We don't store the payload of our own workers.
        if header.author == self.name {
            return Ok(false);
        }

        let mut missing = HashMap::new();
        for (digest, worker_id) in header.payload.iter() {
            // Check whether we have the batch. If one of our worker has the batch, the primary stores the pair
            // (digest, worker_id) in its own storage. It is important to verify that we received the batch
            // from the correct worker id to prevent the following attack:
            //      1. A Bad node sends a batch X to 2f good nodes through their worker #0.
            //      2. The bad node proposes a malformed block containing the batch X and claiming it comes
            //         from worker #1.
            //      3. The 2f good nodes do not need to sync and thus don't notice that the header is malformed.
            //         The bad node together with the 2f good nodes thus certify a block containing the batch X.
            //      4. The last good node will never be able to sync as it will keep sending its sync requests
            //         to workers #1 (rather than workers #0). Also, clients will never be able to retrieve batch
            //         X as they will be querying worker #1.
            let key = [digest.as_ref(), &worker_id.to_le_bytes()].concat();
            if self.store.read(key).await?.is_none() {
                debug!("Missing Digest: {}, Author: {}. Name: {}. Round {}", digest, header.author, self.name, header.height);
                missing.insert(digest.clone(), *worker_id);
            }
        }

        if missing.is_empty() {
            return Ok(false);
        }

        self.tx_header_waiter
            .send(WaiterMessage::SyncBatches(missing, header.clone(), force_sync))
            .await
            .expect("Failed to send sync batch request");
        Ok(true)
    }

    pub async fn fetch_header(&mut self, header_digest: Digest) -> DagResult<()> {
        self.tx_header_waiter
            .send(WaiterMessage::SyncHeader(header_digest))
            .await
            .expect("Failed to send sync special parent request");
        Ok(())
    }

    /// Returns the proposals of a consensus message if we have them all. If at least one parent is missing,
    /// we return an empty vector, synchronize with other nodes, and re-schedule processing
    /// of the header for when we will have all the parents.
    pub async fn get_proposals(&mut self, consensus_message: &ConsensusMessage, delivered_header: &Header) -> DagResult<Vec<Header>> { 
        let mut missing = Vec::new();
        let mut proposals_vector = Vec::new();
        let mut missing_proposals = false;
        //println!("getting proposals");

        match consensus_message {
            ConsensusMessage::Prepare { slot: _, view: _, tc: _, qc_ticket: _, proposals } => {
                for (pk, proposal) in proposals {
                    //println!("proposal inside prepare");

                    if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
                        proposals_vector.push(self.genesis_headers.get(&pk).unwrap().clone());
                        continue;
                    }

                    if proposal.header_digest == delivered_header.digest() {
                        proposals_vector.push(delivered_header.clone());
                        continue;
                    }

                    match self.store.read(proposal.header_digest.to_vec()).await? {
                        Some(header) => {
                            //println!("in some case");
                            proposals_vector.push(bincode::deserialize(&header)?);
                            //println!("after adding to proposal vector");
                        },
                        None => {
                            missing_proposals = true;
                            if self.use_fast_sync  {
                                let lower_bound = self.last_fast_sync_heights.get(pk).unwrap().clone();
                                //if proposal.height > lower_bound {
                                missing.push((*pk, proposal.clone(), lower_bound));
                                self.last_fast_sync_heights.insert(*pk, proposal.height);
                                //}
                            } else {
                                missing.push((*pk, proposal.clone(), 1));
                            }
                        },
                    }
                }
            },
            ConsensusMessage::Confirm { slot: _, view: _, qc: _, proposals } => {
                for (pk, proposal) in proposals {

                    if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
                        proposals_vector.push(self.genesis_headers.get(&pk).unwrap().clone());
                        continue;
                    }


                    match self.store.read(proposal.header_digest.to_vec()).await? {
                        Some(header) => proposals_vector.push(bincode::deserialize(&header)?),
                        None => {
                            missing_proposals = true;
                            if self.use_fast_sync  {
                                let lower_bound = self.last_fast_sync_heights.get(pk).unwrap().clone();
                                //if proposal.height > lower_bound {
                                missing.push((*pk, proposal.clone(), lower_bound));
                                self.last_fast_sync_heights.insert(*pk, proposal.height);
                                //}
                            } else {
                                missing.push((*pk, proposal.clone(), 1));
                            }
                        },
                    }
                }
            },
            ConsensusMessage::Commit { slot: _, view: _, qc: _, proposals } => {
                for (pk, proposal) in proposals {
                    if proposal.height == 0 {
                        continue;
                    }
                    if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
                        proposals_vector.push(self.genesis_headers.get(&pk).unwrap().clone());
                        continue;
                    }

                    match self.store.read(proposal.header_digest.to_vec()).await? {
                        Some(header) => proposals_vector.push(bincode::deserialize(&header)?),
                        None => {
                            missing_proposals = true;
                            if self.use_fast_sync  {
                                let lower_bound = self.last_fast_sync_heights.get(pk).unwrap().clone();
                                //if proposal.height > lower_bound {
                                missing.push((*pk, proposal.clone(), lower_bound));
                                self.last_fast_sync_heights.insert(*pk, proposal.height);
                                //}
                            } else {
                                missing.push((*pk, proposal.clone(), 1));
                            }
                        },
                    }
                }
            },
        }

        if missing.is_empty() && !missing_proposals {
            //println!("Have all proposals");
            debug!("have all proposals and their ancestors");
            return Ok(proposals_vector);
        }

        //println!("sending to header waiter");
        debug!("Triggering sync for proposals");
        debug!("missing proposals are {:?}", missing);
        if !missing.is_empty() {
            self.tx_header_waiter
                .send(WaiterMessage::SyncProposals(missing, consensus_message.clone(), delivered_header.clone()))
                .await
                .expect("Failed to send sync parents request");
        }
        
        Ok(Vec::new())
    }

    /// Returns the proposals of a consensus message if we have them all. If at least one parent is missing,
    /// we return an empty vector, synchronize with other nodes, and re-schedule processing
    /// of the header for when we will have all the parents.
    pub async fn optimistic_tips_ready(&mut self, consensus_message: &ConsensusMessage, delivered_header: &Header) -> DagResult<bool> { 
        let mut missing = Vec::new();
        let mut missing_proposals = false;
        //println!("getting proposals");

        match consensus_message {
            ConsensusMessage::Prepare { slot: _, view: _, tc: _, qc_ticket: _, proposals } => {
                for (pk, proposal) in proposals {
                    //println!("proposal inside prepare");

                    if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
                        continue;
                    }

                    if proposal.header_digest == delivered_header.digest() {
                        continue;
                    }

                    let mut optimistic_key = proposal.header_digest.to_vec();
                    optimistic_key.push(1);
                    debug!("synchronizer optimistic key is {:?}", optimistic_key);
                    debug!("synchronizer optimistic key length is {:?}", optimistic_key.len());
                    match self.store.read(optimistic_key).await? {
                        Some(dummy_value) => {
                            debug!("success readiny optimistic key");
                        },
                        
                        None => {
                            missing_proposals = true;
                            if self.use_fast_sync  {
                                let lower_bound = self.last_fast_sync_heights.get(pk).unwrap().clone();
                                debug!("optimistic tip lower bound is {}", lower_bound);
                                if proposal.height > lower_bound {
                                    missing.push((*pk, proposal.clone(), lower_bound));
                                    self.last_fast_sync_heights.insert(*pk, proposal.height);
                                }
                            } else {
                                missing.push((*pk, proposal.clone(), 1));
                            }
                        },
                    }
                }
            },
            _ => {},
        }

        if missing.is_empty() && !missing_proposals {
            //println!("Have all proposals");
            debug!("have all proposals and their ancestors");
            return Ok(true);
        }

        //println!("sending to header waiter");
        debug!("Triggering sync for optimistic tips");
        debug!("missing tips are {:?}", missing);
        if !missing.is_empty() {
            self.tx_header_waiter
                .send(WaiterMessage::SyncProposals(missing, consensus_message.clone(), delivered_header.clone()))
                .await
                .expect("Failed to send sync parents request");
        }
        
        Ok(false)
    }

    // pub async fn sync_proposals(&mut self, consensus_message: &ConsensusMessage) -> DagResult<bool> {
    //     let mut missing = Vec::new();
    //     //println!("synchronizing on proposals");

    //     match consensus_message {
    //         ConsensusMessage::Prepare { slot: _, view: _, tc: _, qc_ticket: _, proposals } => {
    //             for (pk, proposal) in proposals {
    //                 //println!("proposal inside prepare");

    //                 if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
    //                     continue;
    //                 }

    //                 match self.store.read(proposal.header_digest.to_vec()).await? {
    //                     Some(header) => {},
    //                     None => missing.push(proposal.clone()),
    //                 }
    //             }
    //         },
    //         ConsensusMessage::Confirm { slot: _, view: _, qc: _, proposals } => {
    //             for (pk, proposal) in proposals {

    //                 if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
    //                     continue;
    //                 }

    //                 match self.store.read(proposal.header_digest.to_vec()).await? {
    //                     Some(header) => {},
    //                     None => missing.push(proposal.clone()),
    //                 }
    //             }

    //             //Start async sync.
    //             if !missing.is_empty() {
    //                 self.tx_header_waiter
    //                 .send(WaiterMessage::SyncProposalsCAsync(missing))
    //                 .await
    //                 .expect("Failed to send sync parents request");
    //                 return Ok(false);
    //             }
    //         },
    //         ConsensusMessage::Commit { slot: _, view: _, qc: _, proposals } => {
    //             for (pk, proposal) in proposals {

    //                 if proposal.header_digest == self.genesis_headers.get(&pk).unwrap().digest() {
    //                     continue;
    //                 }

    //                 match self.store.read(proposal.header_digest.to_vec()).await? {
    //                     Some(header) => {},
    //                     None => missing.push(proposal.clone()),
    //                 }
    //             }

    //             //Start sync with loopback
    //             if !missing.is_empty() {
    //                 self.tx_header_waiter
    //                 .send(WaiterMessage::SyncProposalsC(missing, consensus_message.clone()))
    //                 .await
    //                 .expect("Failed to send sync parents request");
    //                 return Ok(false);
    //             }
    //         },
    //     }

    //     Ok(true)
    // }

    pub async fn get_all_headers_for_proposal(
        &mut self,
        proposal: Proposal,
        stop_height: Height,
    ) -> DagResult<Vec<Header>> {
        // The list of blocks for this proposal
        let mut ancestors: Vec<Header> = Vec::new();

        // NOTE: Before calling, must check if proposal is ready, assumes that proposal is ready
        // before calling
        debug!("proposal height is {:?}", proposal.height);
        let mut header: Header = self.get_header(proposal.header_digest).await.expect("already synced should have header").unwrap();

        // Otherwise we have the header and all of its ancestors
        let mut current_height = proposal.height;
        while current_height > stop_height {
            debug!("current height is {:?}, stop height is {:?}", current_height, stop_height);
            ancestors.push(header.clone());
            header = self.get_parent_header(&header).await?.expect("should have parent by now");
            current_height = header.height();
        }

        Ok(ancestors)
    }

    pub async fn get_parent_header(&mut self, header: &Header) -> DagResult<Option<Header>> {
        if header.parent_cert.header_digest == self.genesis_headers.get(&header.author).unwrap().digest() {
            return Ok(Some(self.genesis_headers.get(&header.author).unwrap().clone()));
        }

        let parent = header.parent_cert.header_digest.clone();
        match self.store.read(parent.to_vec()).await? {
            Some(bytes) => {
                debug!("fast sync header height is {}, last height is {}", header.height(), self.last_fast_sync_heights.get(&header.author).unwrap().clone());
                // Update latest height for fast sync
                if self.last_fast_sync_heights.get(&header.author).unwrap().clone() < header.height() - 1 {
                    self.last_fast_sync_heights.insert(header.author, header.height() - 1);
                }
                Ok(Some(bincode::deserialize(&bytes)?))
            },
            
            None => {
                let lower_bound = self.last_fast_sync_heights.get(&header.author).unwrap().clone();
                // Already sent a fast sync request that subsumes this request
                /*if self.use_fast_sync && header.height() - 1 <= lower_bound {
                   return Ok(None)
                }*/

                debug!("not in store fast sync header height is {}, last height is {}", header.height(), lower_bound);

                // Update fast sync heights
                self.last_fast_sync_heights.insert(header.author, header.height() - 1);

                self.tx_header_waiter
                    .send(WaiterMessage::SyncParent(parent, header.clone(), lower_bound))
                    .await
                    .expect("Failed to send sync parent request");
                
                Ok(None)
            }
        }
    }

    pub async fn get_header(&mut self, header_digest: Digest) -> DagResult<Option<Header>> {
        match self.store.read(header_digest.to_vec()).await? {
            Some(bytes) => {
                debug!("get_header: in the store");
                Ok(Some(bincode::deserialize(&bytes)?))
            },
            None => {
                debug!("get_header not in the store");
                Ok(None)
            }
        }
    }

}
