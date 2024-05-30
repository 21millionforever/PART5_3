use crate::block::Block;
use crate::crypto::hash::{H256, Hashable};
use std::collections::HashMap; 

/// Whether the block is mined or received from the network
pub enum BlockOrigin {
    Mined,
    Received{delay_ms: u128},
}

pub struct Blockchain {
    hash_to_block: HashMap<H256, Block>,
    hash_to_height: HashMap<H256, u64>,
    tip: H256,
    difficulty: H256,
    orphan_buffer: HashMap<H256, Vec<Block>>,
    // below are used for experiments:
    pub hash_to_origin: HashMap<H256, BlockOrigin>,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {
        let genesis_block = Block::genesis();
        let genesis_hash = genesis_block.hash();
        let genesis_difficulty = genesis_block.header.difficulty;
        let mut hash_to_block = HashMap::new();
        hash_to_block.insert(genesis_hash, genesis_block);
        let mut hash_to_height = HashMap::new();
        hash_to_height.insert(genesis_hash, 0);
        Blockchain {
            hash_to_block,
            hash_to_height,
            tip: genesis_hash,
            difficulty: genesis_difficulty,
            orphan_buffer: HashMap::new(),
            hash_to_origin: HashMap::new(),
        }
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        let parent_hash = block.header.parent;
        let parent_height = *self.hash_to_height.get(&parent_hash).unwrap();
        let height = parent_height + 1;
        let block_hash = block.hash();
        self.hash_to_block.insert(block_hash, block.clone());
        self.hash_to_height.insert(block_hash, height);
        if height > *self.hash_to_height.get(&self.tip).unwrap() {
            self.tip = block_hash;
        }
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        self.tip
    }

    /// Get all the blocks' hashes along the longest chain
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut curr_hash = self.tip;
        let mut hashes_backward = vec![curr_hash];
        while *self.hash_to_height.get(&curr_hash).unwrap() > 0 { // while not genesis
            curr_hash = self.hash_to_block.get(&curr_hash).unwrap().header.parent;
            hashes_backward.push(curr_hash);
        }
        hashes_backward.into_iter().rev().collect()
    }

    pub fn get_block(&self, hash: &H256) -> &Block {
        self.hash_to_block.get(hash).unwrap()
    }

    pub fn contains_block(&self, hash: &H256) -> bool {
        self.hash_to_block.contains_key(hash)
    }

    /// Check if a block is consistent with PoW
    pub fn pow_validity_check(&self, block: &Block) -> bool {
        block.hash() <= block.header.difficulty && block.header.difficulty == self.difficulty
    }

    /// Check if a block's parent is in the blockchain
    pub fn parent_check(&self, block: &Block) -> bool {
        self.contains_block(&block.header.parent)
    }

    /// Add a PoW valid, parentless block to the orphan buffer
    pub fn add_to_orphan_buffer(&mut self, block: &Block) {
        self.orphan_buffer.entry(block.header.parent).or_insert(vec![]).push(block.clone());
    }

    /// Insert a PoW valid, parentful block into the blockchain, and recursively do all its children.
    /// `out_hashes` is used to store the hashes of all the blocks inserted.
    pub fn insert_recursively(&mut self, block: &Block, out_hashes: &mut Vec<H256>) {
        if self.contains_block(&block.hash()) {
            return;  // redundant item, skip
        }
        self.insert(block);
        out_hashes.push(block.hash());
        if self.orphan_buffer.contains_key(&block.hash()) {
            for child in self.orphan_buffer.remove(&block.hash()).unwrap() {
                self.insert_recursively(&child, out_hashes);
            }
        }
    }

    pub fn block_count(&self) -> usize {
        self.hash_to_block.len()
    }

    pub fn average_block_size(&self) -> usize {
        self.hash_to_block.values().map(|block| block.size()).sum::<usize>() / self.block_count()
    }

    pub fn block_delays_ms(&self) -> Vec<u128> {
        let mut delays: Vec<_> = self.hash_to_origin.values().filter_map(|origin| {
            match origin {
                BlockOrigin::Mined => None,
                BlockOrigin::Received{delay_ms} => Some(*delay_ms),
            }
        }).collect();
        delays.sort();
        delays
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }
    
    #[test]
    fn mp1_insert_chain() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let mut block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
        for _ in 0..50 {
            let h = block.hash();
            block = generate_random_block(&h);
            blockchain.insert(&block);
            assert_eq!(blockchain.tip(), block.hash());
        }
    }

    #[test]
    fn mp1_insert_3_fork_and_back() {
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block_1 = generate_random_block(&genesis_hash);
        blockchain.insert(&block_1);
        assert_eq!(blockchain.tip(), block_1.hash());
        let block_2 = generate_random_block(&block_1.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());
        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_1 = generate_random_block(&block_2.hash());
        blockchain.insert(&fork_block_1);
        assert_eq!(blockchain.tip(), block_3.hash());
        let fork_block_2 = generate_random_block(&fork_block_1.hash());
        blockchain.insert(&fork_block_2);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_4 = generate_random_block(&block_3.hash());
        blockchain.insert(&block_4);
        assert_eq!(blockchain.tip(), fork_block_2.hash());
        let block_5 = generate_random_block(&block_4.hash());
        blockchain.insert(&block_5);
        assert_eq!(blockchain.tip(), block_5.hash());
    }
}
