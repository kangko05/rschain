use std::collections::HashMap;

use crate::utils;

use super::{
    errors::{BlockError, BlockResult},
    transaction::TxOutput,
    tx_pool::TxPool,
    Transaction,
};

#[derive(Debug)]
pub struct Chain {
    blocks: Vec<Block>,
    tx_pool: TxPool,
    utxos: HashMap<(Vec<u8>, u32), TxOutput>, // key-(tx id, output index), val-output
}

impl Chain {
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            tx_pool: TxPool::new(),
            utxos: HashMap::new(),
        }
    }

    pub fn add_tx(&mut self, tx: &Transaction) {
        self.tx_pool.add_one(tx);
    }

    pub fn add_tx_many(&mut self, txs: &Vec<Transaction>) {
        self.tx_pool.add_many(txs);
    }

    /// be careful with adding genesis block because its not validating the input block
    /// Just check if chain is empty and block contains at least one transaction
    pub fn genesis(&mut self, blk: &Block) -> BlockResult<()> {
        if !self.blocks.is_empty() {
            return Err(BlockError::from_str("chain already initialized"));
        }

        if blk.is_empty() {
            return Err(BlockError::EmptyBlock);
        }

        let coinbase_tx = blk.get_coinbase_tx()?;
        let outputs = coinbase_tx.get_outputs();

        for (idx, output) in outputs.iter().enumerate() {
            self.utxos
                .insert((coinbase_tx.get_id().clone(), idx as u32), output.clone());
        }

        self.blocks.push(blk.clone());

        Ok(())
    }

    pub fn add_blk(&mut self, blk: &Block) -> BlockResult<()> {
        if blk.is_empty() {
            return Err(BlockError::EmptyBlock);
        }

        let last_block = self
            .blocks
            .last()
            .expect("at least 1 block must be present on running chain");

        let prev_hash = utils::hash_to_string(&last_block.calculate_hash()?);
        blk.validate(&prev_hash, last_block.get_timestamp())?;

        self.add_utxos(blk)?;
        self.blocks.push(blk.clone());

        Ok(())
    }

    fn add_utxos(&mut self, blk: &Block) -> BlockResult<()> {
        // remove outputs used for inputs
        {
            let inputs = blk
                .get_transactions()?
                .iter()
                .map(|tx| tx.get_inputs().clone())
                .collect::<Vec<_>>()
                .concat();

            for input in inputs {
                self.utxos
                    .remove(&(input.get_tx_id().clone(), input.get_output_index()));
            }
        }

        // add resulted outputs
        {
            let txs = blk
                .get_transactions()?
                .iter()
                .map(|tx| (tx.get_id().clone(), tx.get_outputs().clone()))
                .collect::<Vec<(Vec<u8>, Vec<TxOutput>)>>();

            for (tx_id, outputs) in txs {
                for (idx, output) in outputs.iter().enumerate() {
                    let key = (tx_id.clone(), idx as u32);
                    let val = output.clone();

                    self.utxos.insert(key, val);
                }
            }
        }

        Ok(())
    }

    // TODO: do i need to return the whole transaction? or do i even need this?
    pub fn get_address_utxos(&self, address: &str) -> Vec<TxOutput> {
        let mut utxos = vec![];
        for (_, txout) in &self.utxos {
            if txout.get_address() == address {
                utxos.push(txout.clone());
            }
        }

        utxos
    }

    pub fn get_address_total_value(&self, address: &str) -> u64 {
        let mut total = 0;
        for (_, txout) in &self.utxos {
            if txout.get_address() == address {
                total += txout.get_value();
            }
        }

        total
    }

    pub fn get_address_txs(&self, address: &str) -> Vec<(Vec<u8>, u32)> {
        let mut txs = vec![];
        for (tx_key, txout) in &self.utxos {
            if txout.get_address() == address {
                txs.push(tx_key.clone());
            }
        }

        txs
    }

    pub fn get_last_block_hash_string(&self) -> BlockResult<String> {
        match self.blocks.last() {
            Some(blk) => Ok(utils::hash_to_string(&blk.calculate_hash()?)),
            None => Ok("".to_string()),
        }
    }

    pub fn get_last_block_timestamp(&self) -> u64 {
        match self.blocks.last() {
            Some(blk) => blk.get_timestamp(),
            None => 0,
        }
    }

    pub fn get_utxos(&self) -> &HashMap<(Vec<u8>, u32), TxOutput> {
        &self.utxos
    }

    pub fn get_block_height(&self) -> u64 {
        self.blocks.len() as u64
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
        let mut chain = Chain::new();

        let tx = Transaction::coinbase("first transaction", chain.get_block_height())
            .expect("failed to create a tx");
        let mut genesis_blk =
            Block::new("genesis", &vec![tx]).expect("failed to create a new block");

        genesis_blk.mine().unwrap();

        chain
            .genesis(&genesis_blk)
            .expect("failed to add a genesis block");

        let tx2 = Transaction::coinbase("second transaction", chain.get_block_height())
            .expect("failed to create a tx");
        let mut new_blk = Block::new(&chain.get_last_block_hash_string().unwrap(), &vec![tx2])
            .expect("failed to create a new block");

        new_blk.mine().unwrap();

        chain.add_blk(&new_blk).expect("failed to add a new block");

        dbg!(chain);
    }
}
