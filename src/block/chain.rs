use crate::utils;

use super::{block::Block, errors::BlockResult};

#[derive(Debug)]
pub struct Chain {
    blocks: Vec<Block>,
    tx_pool: Vec<String>, // TODO: String for now -> need to change this to Transactions
}

impl Chain {
    pub fn new() -> BlockResult<Self> {
        // make a gensis block
        let blk = Block::new("genesis", &vec!["genesis transaction".to_string()])?;

        Ok(Self {
            blocks: vec![blk],
            tx_pool: vec![],
        })
    }

    pub fn add_tx(&mut self, tx: &str) {
        self.tx_pool.push(tx.to_string());
    }

    pub fn add_blk(&mut self, blk: &Block) -> BlockResult<()> {
        let last_block = self
            .blocks
            .last()
            .expect("should have 1 block in running chain");

        let prev_hash = utils::hash_to_string(&last_block.calculate_hash());

        blk.validate(&prev_hash, last_block.get_timestamp())?;

        self.blocks.push(blk.clone());

        Ok(())
    }

    pub fn get_last_block_hash_string(&self) -> String {
        utils::hash_to_string(&self.blocks.last().expect("").calculate_hash())
    }

    pub fn get_last_block_timestamp(&self) -> u64 {
        self.blocks.last().expect("").get_timestamp()
    }
}

#[cfg(test)]
mod chain_tests {
    use super::*;

    #[test]
    fn new() {
        let _ = Chain::new();
    }

    #[test]
    fn add_blk() {
        let mut chain = Chain::new().expect("failed to create a chain");
        let mut new_blk = Block::new(
            &chain.get_last_block_hash_string(),
            &vec!["hello".to_string(), "world".to_string()],
        )
        .expect("failed to create a new block");

        new_blk.mine();

        chain.add_blk(&new_blk).expect("failed to add a new block");
        dbg!(chain);
    }
}
