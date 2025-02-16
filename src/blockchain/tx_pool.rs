use super::{
    errors::{TxpoolError, TxpoolResult},
    Transaction,
};

#[derive(Debug)]
pub struct TxPool {
    pool: Vec<Transaction>,
}

impl TxPool {
    pub fn new() -> Self {
        Self { pool: vec![] }
    }

    pub fn add_many(&mut self, items: &Vec<Transaction>) {
        for it in items {
            self.add_one(it);
        }
    }

    pub fn get_many(&mut self, num: usize) -> TxpoolResult<Vec<Transaction>> {
        if self.pool.len() < num {
            return Err(TxpoolError::IndexOutOfBound);
        }

        let mut items = Vec::with_capacity(num);

        for _ in 0..num {
            items.push(self.get_one()?);
        }

        Ok(items)
    }

    pub fn add_one(&mut self, item: &Transaction) {
        self.pool.push(item.clone());
        self.heap_up();
    }

    pub fn get_one(&mut self) -> TxpoolResult<Transaction> {
        if self.pool.is_empty() {
            return Err(TxpoolError::IndexOutOfBound);
        }

        let length = self.pool.len();
        self.pool.swap(0, length - 1);

        let it = if let Some(it) = self.pool.pop() {
            it
        } else {
            return Err(TxpoolError::IndexOutOfBound);
        };

        self.heap_down();

        Ok(it)
    }

    fn heap_up(&mut self) {
        let mut item_idx = self.pool.len() - 1;

        while item_idx > 0 {
            let parent_idx = if item_idx % 2 == 1 {
                (item_idx - 1) / 2
            } else {
                (item_idx - 2) / 2
            };

            if self.pool[item_idx].get_fee() > self.pool[parent_idx].get_fee() {
                self.pool.swap(item_idx, parent_idx);
                item_idx = parent_idx;
            } else {
                break;
            }
        }
    }

    fn heap_down(&mut self) {
        let mut item_idx = 0;
        let length = self.pool.len();

        while item_idx < length {
            let left_idx = (item_idx * 2) + 1;
            let right_idx = left_idx + 1;
            let mut largest = item_idx;

            if left_idx < length && self.pool[left_idx].get_fee() > self.pool[largest].get_fee() {
                largest = left_idx;
            }

            if right_idx < length && self.pool[right_idx].get_fee() > self.pool[largest].get_fee() {
                largest = right_idx;
            }

            if largest != item_idx {
                self.pool.swap(item_idx, largest);
                item_idx = largest;
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod txpool_tests {
    use super::*;

    fn gen_test_txs() -> Vec<Transaction> {
        let mut txv = vec![];
        for i in 0..100 {
            txv.push(Transaction::get_test_tx(i + 1));
        }
        txv
    }

    #[test]
    fn new() {
        let _ = TxPool::new();
    }

    #[test]
    fn add_get() {
        let txv = gen_test_txs();
        let mut txpool = TxPool::new();

        txpool.add_many(&txv);

        for i in 0..txv.len() {
            let tx = txpool.get_one().unwrap();
            assert!(
                (txv.len() - i) as u64 == tx.get_fee(),
                "failed {}th pop, fee = {}",
                i,
                tx.get_fee()
            );
        }
    }

    #[test]
    fn add_get_many() {
        let txv = gen_test_txs();
        let mut txpool = TxPool::new();

        txpool.add_many(&txv);

        let txv = txpool.get_many(txv.len()).unwrap();

        for i in 0..txv.len() {
            assert!(
                (txv.len() - i) as u64 == txv[i].get_fee(),
                "failed {}th pop, fee = {}",
                i,
                txv[i].get_fee()
            );
        }
    }
}
