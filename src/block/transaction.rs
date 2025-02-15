use std::collections::HashMap;

use secp256k1::{ecdsa::Signature, Message, PublicKey, Secp256k1};
use serde::{ser::SerializeStruct, Serialize};

use crate::{utils, wallet::Wallet};

use super::errors::{TxError, TxResult};

/*
 * Transaction
 * 1. coinbase tx - create tx without txin, only txout to miner, with fixed amount of reward
 * 2. general tx
 *  - verify values and input signatures
 *  - create tx inputs: with associated outputs (where crypto will be coming from), input
 *  signature, public key
 *  - create tx outputs: to receiver, miner, and sender(output with remaining value)
 */

// these values are changing in real application
// but since this project will be focusing on learning so fixed
// both in satoshi (1btc = 1e9)
const MINING_REWARD: u64 = 5_000_000_000;
const FEE_RATE: u64 = 20;

#[derive(Clone, Debug)]
pub struct TxInput {
    tx_id: Vec<u8>, // tx if for outputs (which one to use)
    output_index: u32,
    signature: Option<Signature>,
    public_key: PublicKey,
}

impl TxInput {
    pub fn get_tx_id(&self) -> &Vec<u8> {
        &self.tx_id
    }

    pub fn get_output_index(&self) -> u32 {
        self.output_index
    }
}

impl Serialize for TxInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TxInput", 0)?;
        state.serialize_field("tx_id", &self.tx_id)?;
        state.serialize_field("output_index", &self.output_index)?;
        match self.signature {
            Some(signature) => {
                state.serialize_field("signature", &signature.serialize_compact().to_vec())?
            }
            None => state.serialize_field("signature", &Vec::<u8>::new())?,
        }
        state.serialize_field("public_key", &self.public_key.serialize().to_vec())?;
        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct TxOutput {
    value: u64,
    address: String,
}

impl TxOutput {
    pub fn new(value: u64, address: &str) -> Self {
        Self {
            value,
            address: address.to_string(),
        }
    }

    pub fn get_value(&self) -> u64 {
        self.value
    }
    pub fn get_address(&self) -> &String {
        &self.address
    }
}

impl Serialize for TxOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("TxOutput", 2)?;
        state.serialize_field("value", &self.value)?;
        state.serialize_field("address", &self.address)?;
        state.end()
    }
}

#[derive(Clone, Debug)]
pub struct Transaction {
    id: Vec<u8>, // hash
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    block_height: u64,
    fee: u64,
}

impl Serialize for Transaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Transaction", 4)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("inputs", &self.inputs)?;
        state.serialize_field("outputs", &self.outputs)?;
        state.serialize_field("block_height", &self.block_height)?;
        state.end()
    }
}

impl Transaction {
    /// 1. create tx inputs
    /// 2. create tx outputs
    /// 3. verify input signature
    /// 4. combine them altogether in Transaction struct
    pub fn new(
        utxos: &HashMap<(Vec<u8>, u32), TxOutput>,
        wallet: &Wallet,                   // wallet sending from
        from_outputs: Vec<(Vec<u8>, u32)>, // txoutput_id, index of txout_id
        to_addrs: &str,
        value: u64,
    ) -> TxResult<Self> {
        let utxo_value = Self::verify_sending_value(&from_outputs, utxos, value)?;

        let inputs = from_outputs
            .iter()
            .map(|(tx_id, out_idx)| TxInput {
                tx_id: tx_id.clone(),
                output_index: *out_idx,
                public_key: wallet.get_pub_key(),
                signature: None,
            })
            .collect::<Vec<TxInput>>();

        let out_to_receiver = TxOutput::new(value, to_addrs);
        let fee = Self::calcuate_fee(&inputs, &out_to_receiver)?;
        let out_to_sender = TxOutput::new(utxo_value - value - fee, wallet.get_address());

        let outputs = vec![out_to_sender, out_to_receiver];

        let mut tx = Self {
            id: vec![],
            inputs,
            outputs,
            block_height: 0,
            fee,
        };

        tx.id = utils::sha256(&bincode::serialize(&tx)?);

        let secp = Secp256k1::new();

        let msg: Message = Message::from_digest(tx.id.clone().try_into()?);

        for input in &mut tx.inputs {
            let input_sign = Self::sign(&msg, &wallet)?;
            secp.verify_ecdsa(&msg, &input_sign, &wallet.get_pub_key())?;
            input.signature = Some(input_sign);
        }

        Ok(tx)
    }

    fn sign(msg: &Message, wallet: &Wallet) -> TxResult<Signature> {
        let secp = Secp256k1::new();
        Ok(secp.sign_ecdsa(msg, &wallet.get_secret_key()))
    }

    // TODO: this might be inaccurrate - think about it
    fn calcuate_fee(inputs: &Vec<TxInput>, output: &TxOutput) -> TxResult<u64> {
        let mut fee =
            (bincode::serialize(&inputs)?.len() + bincode::serialize(&output)?.len() * 2) as u64;

        fee += 32; // size of id
        fee += 64 * inputs.len() as u64; // signature
        fee += 16; // block_height & fee itself
        fee *= FEE_RATE;

        Ok(fee)
    }

    /// to = miner address
    pub fn coinbase(to: &str, block_height: u64) -> TxResult<Self> {
        let mut tx = Transaction {
            id: vec![],
            inputs: vec![],
            outputs: vec![TxOutput {
                value: MINING_REWARD,
                address: to.to_string(),
            }],
            block_height,
            fee: 0,
        };

        tx.id = utils::sha256(&bincode::serialize(&tx)?);

        Ok(tx)
    }

    /// search outputs from the chain -> get total value in outputs
    /// check if total value is greater than sending value
    fn verify_sending_value(
        from_outputs: &Vec<(Vec<u8>, u32)>,
        utxos: &HashMap<(Vec<u8>, u32), TxOutput>,
        sending_value: u64,
    ) -> TxResult<u64> {
        let mut utxo_value = 0;
        for output_key in from_outputs {
            if let Some(tx) = utxos.get(output_key) {
                utxo_value += tx.value;
            } else {
                return Err(TxError::InvalidUtxo);
            }
        }

        if utxo_value >= sending_value {
            Ok(utxo_value)
        } else {
            Err(TxError::UTXONotSufficient {
                utxo_value,
                sending_value,
            })
        }
    }

    pub fn get_outputs(&self) -> &Vec<TxOutput> {
        &self.outputs
    }

    pub fn get_inputs(&self) -> &Vec<TxInput> {
        &self.inputs
    }

    // to check if its a coinbase
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 0
    }

    // same as get hash
    pub fn get_id(&self) -> &Vec<u8> {
        &self.id
    }

    pub fn get_fee(&self) -> u64 {
        self.fee
    }

    // only for test use
    pub fn get_test_tx(fee: u64) -> Self {
        Self {
            id: vec![],
            inputs: vec![],
            outputs: vec![],
            block_height: 0,
            fee,
        }
    }
}

#[cfg(test)]
mod tx_tests {
    use super::*;
    use crate::block::{chain::Chain, Block};

    #[test]
    fn coinbase() {
        let tx = Transaction::coinbase("all the miners in the world", 0).unwrap();
        dbg!(tx);
    }

    #[test]
    fn serialize_txoutput() {
        let txout = TxOutput {
            value: 50,
            address: "coinbase".to_string(),
        };

        let serialized = bincode::serialize(&txout).unwrap();
        dbg!(serialized);
    }

    #[test]
    fn new() {
        let wallet = Wallet::new();
        let addr1 = wallet.get_address();

        let mut chain = Chain::new();
        let tx1 = Transaction::coinbase(addr1, chain.get_block_height()).unwrap();
        let mut genesis =
            Block::new("genesis", &vec![tx1.clone()]).expect("failed to create a genesis block");

        genesis.mine().expect("failed to mine the block");

        chain
            .genesis(&genesis)
            .expect("failed to add genesis block");

        // mine one more time maybe
        let wallet2 = Wallet::new();
        let tx2 = Transaction::coinbase(addr1, chain.get_block_height()).unwrap();
        let tx3 = Transaction::new(
            chain.get_utxos(),
            &wallet,
            chain.get_address_txs(wallet.get_address()),
            wallet2.get_address(),
            30,
        )
        .unwrap();

        dbg!(tx3.clone());

        let mut blk1 = Block::new(
            &chain.get_last_block_hash_string().unwrap(),
            &vec![tx2, tx3],
        )
        .unwrap();
        blk1.mine().unwrap();

        chain.add_blk(&blk1).unwrap();

        let _utxos = chain.get_address_utxos(addr1);

        //assert!(chain.get_address_total_value(addr1) == 70);
        //assert!(chain.get_address_total_value(wallet2.get_address()) == 30);
    }

    #[test]
    fn new_failing() {
        let wallet1 = Wallet::new();

        let mut chain = Chain::new();

        // first block
        println!("height: {}", chain.get_block_height());
        let tx1 = Transaction::coinbase(wallet1.get_address(), chain.get_block_height()).unwrap();
        let mut genesis_blk = Block::new("genesis", &vec![tx1.clone()]).unwrap();

        genesis_blk.mine().unwrap();
        chain.genesis(&genesis_blk).unwrap();

        // second block
        println!("height: {}", chain.get_block_height());
        let tx2 = Transaction::coinbase(wallet1.get_address(), chain.get_block_height()).unwrap();
        let mut blk = Block::new(
            &chain.get_last_block_hash_string().unwrap(),
            &vec![tx2.clone()],
        )
        .unwrap();

        println!("tx1: {}", utils::hash_to_string(&tx1.get_id()));
        println!("tx2: {}", utils::hash_to_string(&tx2.get_id()));

        blk.mine().unwrap();
        chain.add_blk(&blk).unwrap();

        dbg!(chain.get_address_total_value(wallet1.get_address()));
    }
}
