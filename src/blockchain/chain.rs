use std::collections::HashMap;

use crate::utils;

use super::{
    errors::{BlockError, BlockResult},
    transaction::TxOutput,
    tx_pool::TxPool,
    Block, Transaction,
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

    pub fn from(blocks: Vec<Block>) -> BlockResult<Self> {
        if blocks.is_empty() {
            return Err(BlockError::from_str(
                "failed to construct a chain from blocks",
            ));
        }

        let mut chain = Self::new();

        chain.genesis(&blocks[0])?;
        chain.add_utxos(&blocks[0])?;

        let blocks_len = blocks.len();

        for block in blocks.iter().take(blocks_len).skip(1) {
            chain.add_blk(block)?;
            chain.add_utxos(block)?;
        }

        chain.blocks = blocks;

        Ok(chain)
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

        for txout in self.utxos.values() {
            if txout.get_address() == address {
                utxos.push(txout.clone());
            }
        }

        utxos
    }

    pub fn get_address_total_value(&self, address: &str) -> u64 {
        let mut total = 0;
        for txout in self.utxos.values() {
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

    /// always return Ok(..) - for genesis block, it will return "", else last hash
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

    pub fn get_blocks(&self) -> &Vec<Block> {
        &self.blocks
    }
}

#[cfg(test)]
mod chain_tests {
    use rand::Rng;

    use crate::{blockchain::errors::TxResult, wallet::Wallet};

    use super::*;

    // sending wallet have to contain some coin
    fn gen_test_txs(
        chain: &Chain,
        sending_wallet: &Wallet,
        wallets: Vec<Wallet>,
    ) -> TxResult<(Wallet, Vec<Transaction>)> {
        // random tx
        let mut rng = rand::thread_rng();
        let r: usize = rng.gen_range(0..100);
        let mut txs = vec![Transaction::coinbase(
            wallets[r].get_address(),
            chain.get_block_height(),
        )?];

        for _ in 0..10 {
            let mut r2: usize = rng.gen_range(0..100);

            while r == r2 {
                r2 = rng.gen_range(0..100);
            }

            let to_address = wallets[r2].get_address();
            let random_value: u64 = rng.gen_range(1..1000);

            let utxos = chain.get_utxos();
            let outputs = chain.get_address_txs(sending_wallet.get_address());
            let tx = Transaction::new(utxos, sending_wallet, outputs, to_address, random_value)?;

            txs.push(tx);
        }

        let wallet = wallets[r].clone();

        Ok((wallet, txs))
    }

    fn gen_test_chain() -> BlockResult<Chain> {
        let genesis_wallet = Wallet::new();
        let mut chain = Chain::new();
        let genesis_txs = vec![Transaction::coinbase(
            genesis_wallet.get_address(),
            chain.get_block_height(),
        )?];
        let mut genesis_blk = Block::new("", &genesis_txs)?;

        genesis_blk.mine()?;
        chain.genesis(&genesis_blk)?;

        // random wallets
        let mut wallets = vec![];

        for _ in 0..100 {
            wallets.push(Wallet::new());
        }

        // random txs
        let mut sent_wallet = genesis_wallet;
        for _ in 0..10 {
            let (wallet, txs) = gen_test_txs(&chain, &sent_wallet, wallets.clone())?;
            sent_wallet = wallet;

            let mut new_block = Block::new(&chain.get_last_block_hash_string()?, &txs)?;
            new_block.mine()?;
            chain.add_blk(&new_block)?;
        }

        Ok(chain)
    }

    #[test]
    fn new() {
        let _ = Chain::new();
    }

    #[test]
    fn add_blk() {
        let mut chain = Chain::new();

        let tx = Transaction::coinbase("first transaction", chain.get_block_height())
            .expect("failed to create a tx");
        let mut genesis_blk = Block::new("genesis", &[tx]).expect("failed to create a new block");

        genesis_blk.mine().unwrap();

        chain
            .genesis(&genesis_blk)
            .expect("failed to add a genesis block");

        let tx2 = Transaction::coinbase("second transaction", chain.get_block_height())
            .expect("failed to create a tx");
        let mut new_blk = Block::new(&chain.get_last_block_hash_string().unwrap(), &[tx2])
            .expect("failed to create a new block");

        new_blk.mine().unwrap();

        chain.add_blk(&new_blk).expect("failed to add a new block");

        dbg!(chain);
    }

    #[test]
    fn from_blocks() {
        let test_chain = gen_test_chain().expect("failed to gen test chain");
        let blocks = test_chain.blocks.clone();

        let new_chain = match Chain::from(blocks) {
            Ok(chain) => chain,
            Err(err) => panic!("{err}"),
        };

        // 1. matching utxos
        let orig_utxos = test_chain.get_utxos();
        let recon_utxos = new_chain.get_utxos();

        assert_eq!(
            orig_utxos.len(),
            recon_utxos.len(),
            "utxos length don't match"
        );

        // 2. check utxo elements
        for ((orig_txid, orig_idx), orig_output) in orig_utxos {
            let found = recon_utxos.iter().any(|((txid, idx), output)| {
                txid == orig_txid
                    && idx == orig_idx
                    && output.get_address() == orig_output.get_address()
                    && output.get_value() == orig_output.get_value()
            });
            assert!(found, "Missing or mismatched UTXO");
        }

        // 3. block hash
        for (i, (orig_block, recon_block)) in test_chain
            .blocks
            .iter()
            .zip(new_chain.blocks.iter())
            .enumerate()
        {
            assert_eq!(
                orig_block.calculate_hash().unwrap(),
                recon_block.calculate_hash().unwrap(),
                "Block hash mismatch at height {}",
                i
            );
        }
    }
}
