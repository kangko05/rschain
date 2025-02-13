/*
 * 1. Block Header
 *  - version (option)
 *  - previous_hash
 *  - merkle root
 *  - timestamp (unix)
 *  - difficulty
 *  - nonce
 *
 * 2. Block Body
 *  - transactions (POW)
 */

use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    errors::{BlockError, BlockResult},
    merkle_tree::MerkleTree,
};
use crate::utils;

#[derive(Debug, Clone)]
struct BlockHeader {
    previous_hash: String,
    merkle_root: Vec<u8>,
    timestamp: u64,
    difficulty: u32,
    nonce: u64,
}

#[derive(Debug, Clone)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<String>, // TODO: string for now -> Transaction
}

impl Block {
    pub fn new(previous_hash: &str, transactions: &Vec<String>) -> BlockResult<Self> {
        // hash transactions
        let hashes = transactions
            .iter()
            .map(|s| utils::sha256(s.as_bytes()))
            .collect::<Vec<_>>();

        let merkle_tree = MerkleTree::build(&hashes)?;

        let header = BlockHeader {
            previous_hash: previous_hash.to_string(),
            merkle_root: merkle_tree.root(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            difficulty: 4,
            nonce: 0,
        };

        Ok(Self {
            header,
            transactions: transactions.to_vec(),
        })
    }

    /// returns hash of current block contents
    pub fn calculate_hash(&self) -> Vec<u8> {
        let contents = format!(
            "{}{}{}{}{}",
            self.header.previous_hash,
            utils::hash_to_string(&self.header.merkle_root),
            self.header.timestamp,
            self.header.difficulty,
            self.header.nonce,
        );

        utils::sha256(contents.as_bytes())
    }

    pub fn mine(&mut self) -> Vec<u8> {
        let mut hash = self.calculate_hash();

        while !self.is_valid_hash(&hash) {
            self.header.nonce += 1;
            hash = self.calculate_hash();
        }

        hash
    }

    /// checks if it satisfies difficulty (if hash string starts with # of diff 0s)
    fn is_valid_hash(&self, hash: &Vec<u8>) -> bool {
        let hash_str = utils::hash_to_string(hash);
        let difficulty = self.header.difficulty as usize;
        "0".repeat(difficulty) == hash_str[..difficulty]
    }

    /// validate mined block (new block)
    /// timestamp - if timestamp is older than previous one
    /// prev_hash - if prev_hash matches with mined one
    /// merkle root - if current transactions form a valid merkle root
    /// pow - if mined hash matches with difficulty
    pub fn validate(&self, previous_hash: &str, previous_timestamp: u64) -> BlockResult<()> {
        // timestamp
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        if self.header.timestamp > now || self.header.timestamp < previous_timestamp {
            return Err(BlockError::InvalidTimeStamp);
        }

        // prev_hash
        if self.header.previous_hash != previous_hash {
            return Err(BlockError::InvalidPreviousHash);
        }

        // merkle root
        let hashes = self
            .transactions
            .iter()
            .map(|s| utils::sha256(s.as_bytes()))
            .collect::<Vec<_>>();

        let mtl = MerkleTree::build(&hashes)?;

        if self.header.merkle_root != mtl.root() {
            return Err(BlockError::InvalidMerkleRoot);
        }

        if !self.is_valid_hash(&self.calculate_hash()) {
            return Err(BlockError::InvalidPOW);
        }

        Ok(())
    }

    /// returns timestamp of current block
    pub fn get_timestamp(&self) -> u64 {
        self.header.timestamp
    }
}

#[cfg(test)]
mod block_tests {
    //use crate::utils::hash_to_string;

    use super::*;

    #[test]
    fn new_block() {
        let _ = Block::new("", &vec![String::from("a")]);
    }

    #[test]
    fn get_hash() {
        let block = Block::new("", &vec![String::from("a")]).unwrap();
        dbg!(utils::hash_to_string(&block.calculate_hash()));
    }

    #[test]
    fn mine() {
        let mut block =
            Block::new("genesis", &vec!["hello".to_string(), "world".to_string()]).unwrap();
        let hash = block.mine();

        assert!(block.is_valid_hash(&hash));
    }

    #[test]
    fn validate() {}
}
