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

use serde::{ser::SerializeStruct, Deserialize, Serialize};

use super::{
    errors::{BlockError, BlockResult},
    MerkleTree, Transaction,
};
use crate::utils;

#[derive(Debug, Clone, Deserialize)]
struct BlockHeader {
    previous_hash: String,
    merkle_root: Vec<u8>,
    timestamp: u64,
    difficulty: u32,
    nonce: u64,
}

impl Serialize for BlockHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("BlockHeader", 5)?;
        state.serialize_field("previous_hash", &self.previous_hash)?;
        state.serialize_field("merkle_root", &self.merkle_root)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.serialize_field("difficulty", &self.difficulty)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.end()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
    max_tx: usize,
}

impl Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Block", 3)?;
        state.serialize_field("header", &self.header)?;
        state.serialize_field("transactions", &self.transactions)?;
        state.serialize_field("max_tx", &self.max_tx)?;
        state.end()
    }
}

impl Block {
    pub fn new(previous_hash: &str, transactions: &[Transaction]) -> BlockResult<Self> {
        // hash transactions
        let transactions = transactions.to_vec();
        let hashes = transactions
            .iter()
            .map(|tx| tx.get_id().clone())
            .collect::<Vec<Vec<u8>>>();

        let merkle_tree = MerkleTree::build(&hashes)?;

        let header = BlockHeader {
            previous_hash: previous_hash.to_string(),
            merkle_root: merkle_tree.root(),
            timestamp: utils::unixtime_now(),
            difficulty: 4,
            nonce: 0,
        };

        Ok(Self {
            header,
            transactions: transactions.to_vec(),
            max_tx: 15, // in bitcoin, limits depend on block size & tx size
                        // 15 for now because this is only going to run locally with small number
                        //    of nodes
        })
    }

    /// returns hash of current block contents
    pub fn calculate_hash(&self) -> BlockResult<Vec<u8>> {
        let serialized = bincode::serialize(&self)?;
        Ok(utils::sha256(&serialized))
    }

    /// returns mined hash
    pub fn mine(&mut self) -> BlockResult<Vec<u8>> {
        let mut hash = self.calculate_hash()?;

        while !self.is_valid_hash(&hash) {
            self.header.nonce += 1;
            hash = self.calculate_hash()?;
        }

        Ok(hash)
    }

    /// checks if it satisfies difficulty (if hash string starts with # of diff 0s)
    fn is_valid_hash(&self, hash: &[u8]) -> bool {
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
            .map(|tx| tx.get_id().clone())
            .collect::<Vec<Vec<u8>>>();

        let mtl = MerkleTree::build(&hashes)?;

        if self.header.merkle_root != mtl.root() {
            return Err(BlockError::InvalidMerkleRoot);
        }

        if !self.is_valid_hash(&self.calculate_hash().unwrap()) {
            return Err(BlockError::InvalidPOW);
        }

        Ok(())
    }

    /// returns timestamp of current block
    pub fn get_timestamp(&self) -> u64 {
        self.header.timestamp
    }

    pub fn get_coinbase_tx(&self) -> BlockResult<Transaction> {
        match self.transactions.first() {
            Some(tx) => {
                if !tx.is_coinbase() {
                    Err(BlockError::from_str("failed to find coinbase tx"))
                } else {
                    Ok(tx.clone())
                }
            }
            None => Err(BlockError::from_str("block has no transactions")),
        }
    }

    pub fn get_transactions(&self) -> BlockResult<&Vec<Transaction>> {
        if self.transactions.is_empty() {
            return Err(BlockError::from_str("empty transactions"));
        }

        Ok(&self.transactions)
    }

    /// TODO: implement this
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn get_max_tx(&self) -> usize {
        self.max_tx
    }
}

#[cfg(test)]
mod block_tests {
    //use crate::utils::hash_to_string;

    use super::*;

    fn test_tx(tmsg: &str) -> Transaction {
        Transaction::coinbase(tmsg, 0).expect("failed to build a transaction")
    }

    #[test]
    fn new_block() {
        let blk = Block::new("coinbase", &[test_tx("coinbase")]).expect("failed to build a block");
        dbg!(blk);
    }

    #[test]
    fn get_hash() {
        let block = Block::new("", &[test_tx("coinbase")]).unwrap();
        dbg!(utils::hash_to_string(&block.calculate_hash().unwrap()));
    }

    #[test]
    fn mine() {
        let mut block = Block::new("genesis", &[test_tx("coinbase")]).unwrap();
        let hash = block.mine().unwrap();

        assert!(block.is_valid_hash(&hash));
    }

    #[test]
    fn validate() {
        let mut blk1 = Block::new("", &[test_tx("first")]).expect("failed to create a block");
        blk1.mine().unwrap();
        let blk1_hash = utils::hash_to_string(&blk1.calculate_hash().unwrap());
        let mut blk2 =
            Block::new(&blk1_hash, &[test_tx("second")]).expect("failed to create a block");

        blk2.mine().unwrap();

        assert!(blk2.validate(&blk1_hash, blk1.get_timestamp()).is_ok());
    }

    #[test]
    fn validate_wrong_prev_hash() {
        let mut blk1 = Block::new("", &[test_tx("first")]).expect("failed to create a block");
        blk1.mine().unwrap();
        let wrong_hash = "totally_wrong_hash";
        let mut blk2 =
            Block::new(wrong_hash, &[test_tx("second")]).expect("failed to create a block");
        blk2.mine().unwrap();

        assert!(matches!(
            blk2.validate(
                &utils::hash_to_string(&blk1.calculate_hash().unwrap()),
                blk1.get_timestamp()
            ),
            Err(BlockError::InvalidPreviousHash)
        ));
    }

    #[test]
    fn validate_wrong_timestamp() {
        let mut blk1 = Block::new("", &[test_tx("first")]).expect("failed to create a block");
        blk1.mine().unwrap();
        let blk1_hash = utils::hash_to_string(&blk1.calculate_hash().unwrap());
        let mut blk2 =
            Block::new(&blk1_hash, &[test_tx("second")]).expect("failed to create a block");
        blk2.mine().unwrap();

        assert!(matches!(
            blk2.validate(&blk1_hash, blk2.get_timestamp() + 1),
            Err(BlockError::InvalidTimeStamp)
        ));
    }
}
