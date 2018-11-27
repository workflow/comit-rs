#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]
extern crate comit_node;
extern crate ethereum_support;
extern crate hex;
extern crate pretty_env_logger;
extern crate secp256k1_support;
#[macro_use]
extern crate log;
extern crate tc_web3_client;
extern crate testcontainers;
#[macro_use]
extern crate lazy_static;
extern crate rlp;
extern crate tiny_keccak;

mod ethereum_wallet;
mod htlc_harness;
mod parity_client;

use ethereum_support::{Bytes, EtherQuantity, U256};
use htlc_harness::*;
use std::time::Duration;
use testcontainers::clients::Cli;

const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";
const HTLC_TIMEOUT: Duration = Duration::from_secs(5);
const HTLC_GAS_COST: u64 = 8879000;

#[test]
fn given_deployed_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let docker = Cli::default();
    let (alice, bob, htlc, _, client, _handle, _container) = harness(
        &docker,
        TestHarnessParams {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_type: HtlcType::Eth {
                htlc_eth_value: EtherQuantity::from_eth(0.4),
            },
            htlc_timeout: HTLC_TIMEOUT,
            htlc_secret: SECRET.clone(),
        },
    );

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );

    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Send correct secret to contract
    client.send_data(htlc, Some(Bytes(SECRET.to_vec())));

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.4).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
    let docker = Cli::default();
    let (alice, bob, htlc, _, client, _handle, _container) = harness(
        &docker,
        TestHarnessParams {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_type: HtlcType::Eth {
                htlc_eth_value: EtherQuantity::from_eth(0.4),
            },
            htlc_timeout: HTLC_TIMEOUT,
            htlc_secret: SECRET.clone(),
        },
    );

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    ::std::thread::sleep(HTLC_TIMEOUT);
    ::std::thread::sleep(HTLC_TIMEOUT);
    client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(1.0).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.0).wei()
    );
}

#[test]
fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let docker = Cli::default();
    let (alice, bob, htlc, _, client, _handle, _container) = harness(
        &docker,
        TestHarnessParams {
            alice_initial_ether: EtherQuantity::from_eth(1.0),
            htlc_type: HtlcType::Eth {
                htlc_eth_value: EtherQuantity::from_eth(0.4),
            },
            htlc_timeout: HTLC_TIMEOUT,
            htlc_secret: SECRET.clone(),
        },
    );

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );

    // Wait for the contract to expire
    client.send_data(htlc, None);

    assert_eq!(
        client.eth_balance_of(bob),
        EtherQuantity::from_eth(0.0).wei()
    );
    assert_eq!(
        client.eth_balance_of(alice),
        EtherQuantity::from_eth(0.6).wei() - U256::from(HTLC_GAS_COST)
    );
    assert_eq!(
        client.eth_balance_of(htlc),
        EtherQuantity::from_eth(0.4).wei()
    );
}