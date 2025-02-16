#![allow(dead_code)]

use uuid::Uuid;

/*
 * General Node
 */
#[derive(Debug)]
pub struct Node {
    uuid: String,
}

impl Node {
    fn new() -> Self {
        let uuid = Uuid::new_v4().to_string();

        Self { uuid }
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn new() {
        let node = Node::new();

        dbg!(node);
    }
}
