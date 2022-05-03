use std::str::FromStr;

use anoma::proto::Tx;
use anoma::types::key::{common, RefTo};
use anoma::types::transaction::protocol::ProtocolTxType;

use crate::e2e::helpers::get_actor_rpc;
use crate::e2e::setup;
use crate::e2e::setup::constants::{
    wasm_abs_path, ALBERT, TX_WRITE_STORAGE_KEY_WASM,
};
use crate::e2e::setup::{Bin, Who};
use crate::{run, run_as};

const ETH_BRIDGE_ADDRESS: &str = "atest1v9hx7w36g42ysgzzwf5kgem9ypqkgerjv4ehxgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpq8f99ew";

/// # Examples
///
/// ```
/// let storage_key = storage_key("queue");
/// assert_eq!(storage_key, "#atest1v9hx7w36g42ysgzzwf5kgem9ypqkgerjv4ehxgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpq8f99ew/queue");
/// ```
fn storage_key(path: &str) -> String {
    format!("#{ETH_BRIDGE_ADDRESS}/{}", path)
}

#[test]
fn everything() {
    const LEDGER_STARTUP_TIMEOUT_SECONDS: u64 = 30;
    const CLIENT_COMMAND_TIMEOUT_SECONDS: u64 = 30;
    const SOLE_VALIDATOR: Who = Who::Validator(0);

    let test = setup::single_node_net().unwrap();

    let mut anoman_ledger = run_as!(
        test,
        SOLE_VALIDATOR,
        Bin::Node,
        &["ledger"],
        Some(LEDGER_STARTUP_TIMEOUT_SECONDS)
    )
    .unwrap();
    anoman_ledger
        .exp_string("Anoma ledger node started")
        .unwrap();
    anoman_ledger.exp_string("Tendermint node started").unwrap();
    anoman_ledger.exp_string("Committed block hash").unwrap();

    let tx_data_path = test.base_dir.path().join("queue_storage_key.txt");
    std::fs::write(&tx_data_path, &storage_key("queue")[..]).unwrap();

    let tx_code_path = wasm_abs_path(TX_WRITE_STORAGE_KEY_WASM);
    let tx_code_contents = std::fs::read(&tx_code_path).unwrap();

    let tx_data_path = tx_data_path.to_string_lossy().to_string();
    let tx_code_path = tx_code_path.to_string_lossy().to_string();
    let ledger_addr = get_actor_rpc(&test, &SOLE_VALIDATOR);
    let tx_args = vec![
        "tx",
        "--code-path",
        &tx_code_path,
        "--data-path",
        &tx_data_path,
        "--ledger-address",
        &ledger_addr,
    ];

    println!("Test a transaction signed with a non-validator key is rejected");
    {
        let mut tx_args = tx_args.clone();
        tx_args.append(&mut vec!["--signer", &ALBERT]);

        for &dry_run in &[true, false] {
            let tx_args = if dry_run {
                vec![tx_args.clone(), vec!["--dry-run"]].concat()
            } else {
                tx_args.clone()
            };
            let mut anomac_tx = run!(
                test,
                Bin::Client,
                tx_args,
                Some(CLIENT_COMMAND_TIMEOUT_SECONDS)
            )
            .unwrap();

            if !dry_run {
                if !cfg!(feature = "ABCI") {
                    anomac_tx.exp_string("Transaction accepted").unwrap();
                }
                anomac_tx.exp_string("Transaction applied").unwrap();
            }
            // TODO: we should check here explicitly with the ledger via a
            //  Tendermint RPC call that the path `value/#EthBridge/queue`
            //  is unchanged rather than relying solely  on looking at anomac
            //  stdout.
            anomac_tx.exp_string("Transaction is invalid").unwrap();
            anomac_tx
                .exp_string(&format!("Rejected: {}", ETH_BRIDGE_ADDRESS))
                .unwrap();
            anomac_tx.assert_success();
        }
    }

    println!(
        "Test the same transaction signed with a protocol key is accepted"
    );
    #[cfg(feature = "ferveo-tpke")]
    {
        // TODO: get the sole validator's protocol sk somehow - this one below
        //  was generated for a local devchain
        const ARBITRARY_PROTOCOL_SK_HEX: &str = "00d984d85de44dfc7a1fbca7db43dae6afe38f60244f913cf35a4f0bcdd8d135c8";
        let protocol_sk =
            common::SecretKey::from_str(ARBITRARY_PROTOCOL_SK_HEX).unwrap();
        let protocol_pk = protocol_sk.ref_to();

        // construct a signed Ethereum bridge protocol transaction
        let tx_data_contents = tx_data_contents.to_owned().clone().into_bytes();
        let unsigned = ProtocolTxType::EthereumBridgeUpdate(Tx::new(
            tx_code_contents,
            Some(tx_data_contents),
        ));
        let _signed = unsigned.sign(&protocol_pk, &protocol_sk);

        // TODO: serialize the signed transaction to a file (Borsh?)
        // TODO: submit the signed transaction via an anomac command
        // TODO: verify the transaction was accepted, and the `/queue` key was written to
    }
}
