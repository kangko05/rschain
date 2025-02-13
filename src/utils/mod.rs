use sha2::{Digest, Sha256};

pub fn sha256(it: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.update(it);
    hasher.finalize().to_vec()
}

pub fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    if n & (n - 1) == 0 {
        return n;
    }

    1 << (usize::BITS - n.leading_zeros())
}

pub fn hash_to_string(hash: &Vec<u8>) -> String {
    hash.iter().map(|&by| format!("{:02x}", by)).collect()
}

#[cfg(test)]
mod hash_test {
    use super::*;

    #[test]
    fn hash() {
        let res = sha256("hello world!".as_bytes());
        assert!(res.len() == 32)
    }
}
