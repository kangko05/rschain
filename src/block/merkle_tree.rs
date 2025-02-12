#![allow(dead_code)]

/*
 * (TODO) need better performance
 *
 * - each leaf node contains 'hash'
 * - parent node contains concatenation of its children hashs -> hash
 * - merkle root can verify if a certain hash belongs to one of its children (leaf)
 *
 * Merkel Proof - reconstructing Merkle root with given hash to prove that given hash belongs to
 * the tree
 */

use std::fmt::Display;

use crate::utils;

#[derive(Debug)]
pub enum MerkleError {
    Message(String),
}

impl MerkleError {
    pub fn err(msg: &str) -> MerkleError {
        Self::Message(msg.to_string())
    }
}

impl std::error::Error for MerkleError {}

impl Display for MerkleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MerkleError::Message(s) => write!(f, "{}", s),
        }
    }
}

type MerkleResult<T> = Result<T, MerkleError>;

//
#[derive(Clone)]
struct MerkleLevelChain {
    next_level: Option<Box<MerkleLevelChain>>,
    nodes: Vec<Vec<u8>>,
}

impl MerkleLevelChain {
    fn new(nodes: &Vec<Vec<u8>>) -> Self {
        Self {
            next_level: None,
            nodes: nodes.clone(),
        }
    }

    fn build_chain(hashes: &Vec<Vec<u8>>) -> MerkleResult<(MerkleLevelChain, Vec<u8>)> {
        if hashes.len() < 1 {
            return Err(MerkleError::err("need more than 1 hashes")); // error
        }

        let mut leaf_hashes = hashes.clone();
        let leaf_count = leaf_hashes.len();

        if leaf_count % 2 == 1 {
            leaf_hashes.push(leaf_hashes.last().unwrap().clone());
        }

        let mut mtl = MerkleLevelChain::new(&leaf_hashes);
        let mut curr_hashes = leaf_hashes.clone();
        let mut curr_len = curr_hashes.len();

        while curr_len > 1 {
            let mut parent_hashes = vec![];
            for i in (0..curr_hashes.len()).step_by(2) {
                let left = &curr_hashes[i];
                let mut right = &curr_hashes[i + 1];

                if right.is_empty() {
                    right = &curr_hashes[i];
                }

                let parent = Self::get_parent_hash(left, right);

                parent_hashes.push(parent);
            }

            let p_len = parent_hashes.len();
            let p_len_should_be = utils::next_power_of_two(p_len);

            for _ in p_len..p_len_should_be {
                parent_hashes.push(vec![]);
            }

            mtl.add_next_level(MerkleLevelChain::new(&parent_hashes));

            curr_hashes = parent_hashes;
            curr_len = p_len;
        }

        Ok((mtl, curr_hashes[0].clone()))
    }

    fn verify(&self, hash: &Vec<u8>) -> bool {
        match self.find_hash(hash) {
            None => false,
            Some(idx) => self.verify_inner(hash, idx),
        }
    }

    #[allow(unused_assignments)]
    fn verify_inner(&self, hash: &Vec<u8>, idx: usize) -> bool {
        let length = self.nodes.len();

        match &self.next_level {
            None => &self.get_root() == hash,
            Some(next) => {
                let mut left = &Vec::new();
                let mut right = &Vec::new();

                if idx % 2 == 0 {
                    left = &self.nodes[idx];

                    if idx + 1 >= length {
                        return false;
                    }

                    right = &self.nodes[idx + 1];

                    if right.is_empty() {
                        right = left;
                    }
                } else {
                    left = &self.nodes[idx - 1];
                    right = &self.nodes[idx];
                }

                let parent_hash = Self::get_parent_hash(left, right);

                next.verify_inner(&parent_hash, idx / 2)
            }
        }
    }

    fn add_next_level(&mut self, new_level: MerkleLevelChain) {
        match &mut self.next_level {
            None => self.next_level = Some(Box::new(new_level)),
            Some(next) => next.add_next_level(new_level),
        }
    }

    // returns idx
    fn find_hash(&self, hash: &Vec<u8>) -> Option<usize> {
        self.nodes.iter().position(|h| h == hash)
    }

    // for debugging
    fn print(&self) {
        for node in &self.nodes {
            println!("{}", hash_to_string(node));
        }

        println!();

        match &self.next_level {
            None => return,
            Some(next) => next.print(),
        }
    }

    // for debugging
    fn get_idx(&self, idx: usize) -> Vec<u8> {
        self.nodes[idx].clone()
    }

    fn get_root(&self) -> Vec<u8> {
        match &self.next_level {
            None => self.nodes[0].clone(),
            Some(next) => next.get_root(),
        }
    }

    fn get_parent_hash(left: &Vec<u8>, right: &Vec<u8>) -> Vec<u8> {
        let mut combined = Vec::with_capacity(left.len() + right.len());
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);
        utils::sha256(&combined)
    }
}

pub struct MerkleTree {
    root: Vec<u8>,
    level_chain: MerkleLevelChain,
    leaf_count: usize,
}

impl MerkleTree {
    pub fn build(hashes: &Vec<Vec<u8>>) -> MerkleResult<Self> {
        let leaf_count = hashes.len();
        let (level_chain, root) = MerkleLevelChain::build_chain(hashes)?;

        Ok(Self {
            level_chain,
            root,
            leaf_count,
        })
    }

    pub fn verify(&self, hash: &Vec<u8>) -> bool {
        self.level_chain.verify(hash)
    }
}

// for debugging
fn hash_to_string(hash: &Vec<u8>) -> String {
    hash.iter().map(|&by| format!("{:02x}", by)).collect()
}

#[cfg(test)]
mod merkle_tests {
    use rand::Rng;

    use super::*;

    fn gen_test_cases(v: Vec<i32>) -> Vec<Vec<u8>> {
        v.iter()
            .map(|it| utils::sha256(it.to_string().as_bytes()))
            .collect()
    }

    fn gen_random_vector(length: usize) -> Vec<i32> {
        let mut v = vec![];
        let mut rng = rand::rng();

        for _ in 0..length {
            v.push(rng.random::<i32>());
        }

        v
    }

    #[test]
    fn merkle_chain() {
        let rv = gen_random_vector(10000);
        let tcs = gen_test_cases(rv);

        let (mtl, root) = MerkleLevelChain::build_chain(&tcs).unwrap();

        for i in 0..tcs.len() {
            assert!(mtl.verify(&tcs[i]), "failed at {i}");
        }

        assert!(mtl.get_root() == root);
    }

    #[test]
    fn merkle_tree() {
        let rv = gen_random_vector(10000);
        let tcs = gen_test_cases(rv);

        let mt = MerkleTree::build(&tcs).unwrap();

        for i in 0..tcs.len() {
            assert!(mt.verify(&tcs[i]), "failed at {i}");
        }
    }

    #[test]
    fn performance_test() {
        for size in [100, 1000, 10000].iter() {
            let v: Vec<i32> = (0..*size).collect();
            let tcs = gen_test_cases(v);
            let start = std::time::Instant::now();
            let mt = MerkleTree::build(&tcs).unwrap();
            for hash in tcs.iter() {
                assert!(mt.verify(hash));
            }
            println!("Size {}: {:?}", size, start.elapsed());
        }
    }
}
