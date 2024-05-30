use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::blockchain::Blockchain;
use crate::crypto::hash::Hashable;
use crate::blockchain::BlockOrigin;

use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(hashes) => {
                    debug!("NewBlockHashes: {:?}", hashes);
                    let blockchain = self.blockchain.lock().unwrap();
                    let missing_hashes: Vec<_> = hashes.into_iter()
                        .filter(|hash| !blockchain.contains_block(hash))
                        .collect();
                    if !missing_hashes.is_empty() {
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                }
                Message::GetBlocks(hashes) => {
                    debug!("GetBlocks: {:?}", hashes);
                    let blockchain = self.blockchain.lock().unwrap();
                    let blocks: Vec<_> = hashes.iter()
                        .filter(|hash| blockchain.contains_block(hash))
                        .map(|hash| blockchain.get_block(hash).clone())
                        .collect();
                    if !blocks.is_empty() {
                        peer.write(Message::Blocks(blocks));
                    }
                }
                Message::Blocks(blocks) => {
                    debug!("Blocks: {:?}", blocks);
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    let mut blockchain = self.blockchain.lock().unwrap();
                    let mut relay_hashes = Vec::new();
                    let mut missing_hashes = Vec::new();
                    for block in blocks {
                        // For experiment: record the block delay; don't count redundant or self-mined blocks:
                        blockchain.hash_to_origin.entry(block.hash())
                            .or_insert(BlockOrigin::Received{ delay_ms: now - block.header.timestamp });
                        // Regular processing:
                        if blockchain.contains_block(&block.hash()) {
                            continue;
                        }
                        if !blockchain.pow_validity_check(&block) {
                            warn!("PoW check failed");
                            continue;
                        }
                        if !blockchain.parent_check(&block) {
                            blockchain.add_to_orphan_buffer(&block);
                            missing_hashes.push(block.header.parent);
                            continue;
                        }
                        blockchain.insert_recursively(&block, &mut relay_hashes);
                    }
                    if !missing_hashes.is_empty() {
                        peer.write(Message::GetBlocks(missing_hashes));
                    }
                    if !relay_hashes.is_empty() {
                        self.server.broadcast(Message::NewBlockHashes(relay_hashes));
                    }
                }
            }
        }
    }
}
