#![allow(dead_code)]

use rand::rngs::OsRng;
use ripemd::{Digest, Ripemd160};
use secp256k1::{PublicKey, Secp256k1};

use crate::utils;

#[derive(Debug, Clone)]
pub struct Wallet {
    public_key: String,
    private_key: String,
    address: String,
}

impl Wallet {
    pub fn new() -> Self {
        let secp = Secp256k1::new();
        let (private_key, public_key) = secp.generate_keypair(&mut OsRng);
        let address = Self::gen_address(&public_key);
        let public_key = public_key.to_string();
        let private_key = private_key.display_secret().to_string();

        Self {
            public_key,
            private_key,
            address,
        }
    }

    ///  pub key -> sha256 -> ripemd -> add version bytes -> checksum -> base58 encode
    ///  ripemd to reduce the length of the address & to add one more security layer (sha 256 bit -> ripemd 160 bit)
    ///  skipping version bytes for this project
    ///  adding checksum to validate input address (just in case)
    ///  bs58 to produce human readable string & reduce more length (ripemd 160 + checksum 32 -> 26~32 length str)
    fn gen_address(pk: &PublicKey) -> String {
        let sha = utils::sha256(&pk.serialize());
        let ripemd = Ripemd160::digest(&sha).to_vec();
        let double_sha = utils::sha256(&utils::sha256(&ripemd));
        let checksum = &double_sha[..4];

        let mut address_bytes = ripemd;
        address_bytes.extend_from_slice(checksum);

        bs58::encode(address_bytes).into_string()
    }

    // TODO: handle error
    pub fn get_pub_key(&self) -> &String {
        &self.public_key
        // PublicKey::from_str(&self.public_key).unwrap()
    }

    // TODO: handle error
    pub fn get_secret_key(&self) -> &String {
        &self.private_key
        // SecretKey::from_str(&self.private_key).unwrap()
    }

    pub fn get_address(&self) -> &String {
        &self.address
    }
}

#[cfg(test)]
mod wallet_tests {
    use super::*;

    #[test]
    fn new() {
        let wallet = Wallet::new();
        dbg!(wallet);
    }
}
