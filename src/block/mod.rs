#![allow(dead_code)]

mod merkle_tree;

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

pub struct BlockHeader {
    previous_hash: String,
}

pub struct Block {}
