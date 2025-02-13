pub struct TxInput {}

pub struct TxOutput {}
pub struct Transaction {
    id: Vec<u8>, // hash

    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
}

impl Transaction {
    pub fn new() {}
}
