/// Transaction for modifying EthBridge VP storage
pub mod tx {
    use anoma::ledger::eth_bridge::storage::queue_key;

    use crate::imports::tx;

    pub fn update_queue(data: &[u8]) {
        let queue_key = queue_key();
        tx::write(queue_key.to_string(), data)
    }
}
