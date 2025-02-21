use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn hash_to_string(hash: &[u8]) -> String {
    hash.iter()
        .fold(String::new(), |acc, &by| format!("{}{:02x}", acc, by))

    //hash.iter().map(|&by| format!("{:02x}", by)).collect()
}

//pub fn unixtime_now() -> Result<u64, SystemTimeError> {
//    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
//}

pub fn unixtime_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("failed to get system time")
        .as_secs()
}

/// assuming input vectors/array are 32 length
pub fn xor_distance_256(v1: &[u8], v2: &[u8]) -> Vec<u8> {
    assert_eq!(v1.len(), 32, "first input must be 32 byets");
    assert_eq!(v2.len(), 32, "second input must be 32 byets");

    v1.iter().zip(v2.iter()).map(|(a, b)| a ^ b).collect()
}

#[cfg(test)]
mod hash_test {
    use super::*;

    #[test]
    fn hash() {
        let res = sha256("hello world!".as_bytes());
        assert!(res.len() == 32)
    }

    #[test]
    fn hash_to_str() {
        let s = "hello world".as_bytes();
        println!("{}", hash_to_string(s));
    }
}
