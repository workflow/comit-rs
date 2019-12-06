use crate::{
    http_api::{
        ethereum_network, problem, Http, MissingQueryParameters, UnexpectedQueryParameters,
    },
    swap_protocols::{
        actions::{
            bitcoin::{SendToAddress, SpendOutput},
            ethereum,
        },
        ledger, SwapId,
    },
    timestamp::Timestamp,
};
use anyhow::Context;
use blockchain_contracts::bitcoin::witness;
use http_api_problem::HttpApiProblem;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use warp::http::StatusCode;

pub trait ToSirenAction {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action;
}

pub trait ListRequiredFields {
    fn list_required_fields() -> Vec<siren::Field>;
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ActionExecutionParameters {
    BitcoinAddressAndFee {
        address: bitcoin::Address,
        fee_per_wu: String,
    },
    None {},
}

/// `network` field here for backward compatibility, to be removed with #1580
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(tag = "type", content = "payload")]
pub enum ActionResponseBody {
    BitcoinSendAmountToAddress {
        to: bitcoin::Address,
        amount: String,
        network: Http<bitcoin::Network>,
    },
    BitcoinBroadcastSignedTransaction {
        hex: String,
        network: Http<bitcoin::Network>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_median_block_time: Option<Timestamp>,
    },
    EthereumDeployContract {
        data: crate::ethereum::Bytes,
        amount: crate::ethereum::EtherQuantity,
        gas_limit: crate::ethereum::U256,
        network: ethereum_network::Network,
        chain_id: ledger::ethereum::ChainId,
    },
    EthereumCallContract {
        contract_address: crate::ethereum::Address,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<crate::ethereum::Bytes>,
        gas_limit: crate::ethereum::U256,
        chain_id: ledger::ethereum::ChainId,
        network: ethereum_network::Network,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_block_timestamp: Option<Timestamp>,
    },
    None,
}

impl ActionResponseBody {
    fn bitcoin_broadcast_signed_transaction(
        transaction: &bitcoin::Transaction,
        network: bitcoin::Network,
    ) -> Self {
        let min_median_block_time = if transaction.lock_time == 0 {
            None
        } else {
            // The first time a tx with lock_time can be broadcasted is when
            // mediantime == locktime + 1
            let min_median_block_time = transaction.lock_time + 1;
            Some(Timestamp::from(min_median_block_time))
        };

        ActionResponseBody::BitcoinBroadcastSignedTransaction {
            hex: bitcoin::consensus::encode::serialize_hex(transaction),
            network: Http(network),
            min_median_block_time,
        }
    }
}

pub trait IntoResponsePayload {
    fn into_response_payload(
        self,
        parameters: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody>;
}

impl IntoResponsePayload for SendToAddress {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::None {} => Ok(self.into()),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "bitcoin::SendToAddress",
                parameters: &["address", "fee_per_wu"],
            })),
        }
    }
}

impl From<SendToAddress> for ActionResponseBody {
    fn from(action: SendToAddress) -> Self {
        let SendToAddress {
            to,
            amount,
            network,
        } = action;
        ActionResponseBody::BitcoinSendAmountToAddress {
            to,
            amount: amount.as_sat().to_string(),
            network: Http(network),
        }
    }
}

impl ListRequiredFields for SendToAddress {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl IntoResponsePayload for SpendOutput {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        match query_params {
            ActionExecutionParameters::BitcoinAddressAndFee {
                address,
                fee_per_wu,
            } => {
                let fee_per_wu = fee_per_wu.parse::<usize>().with_context(|| {
                    HttpApiProblem::new("Invalid query parameter.")
                        .set_status(StatusCode::BAD_REQUEST)
                        .set_detail("Query parameter fee-per-byte is not a valid unsigned integer.")
                })?;

                let network = self.network;
                let transaction =
                    self.spend_to(address)
                        .sign_with_rate(&*crate::SECP, fee_per_wu)
                        .map_err(|e| {
                            log::error!("Could not sign Bitcoin transaction: {:?}", e);
                            match e {
                                witness::Error::FeeHigherThanInputValue => HttpApiProblem::new(
                                    "Fee is too high.",
                                )
                                .set_status(StatusCode::BAD_REQUEST)
                                .set_detail(
                                    "The Fee per byte/WU provided makes the total fee higher than the spendable input value.",
                                ),
                                witness::Error::OverflowingFee => HttpApiProblem::new(
                                    "Fee is too high.",
                                )
                                    .set_status(StatusCode::BAD_REQUEST)
                                    .set_detail(
                                        "The Fee per byte/WU provided makes the total fee higher than the system supports.",
                                    )
                            }
                        })?;

                Ok(ActionResponseBody::bitcoin_broadcast_signed_transaction(
                    &transaction,
                    network,
                ))
            }
            _ => Err(anyhow::Error::from(MissingQueryParameters {
                action: "bitcoin::SpendOutput",
                parameters: &[
                    problem::MissingQueryParameter {
                        name: "address",
                        data_type: "string",
                        description: "The bitcoin address to where the funds should be sent.",
                    },
                    problem::MissingQueryParameter {
                        name: "fee_per_wu",
                        data_type: "uint",
                        description:
                        "The fee per weight unit you want to pay for the transaction in satoshis.",
                    },
                ]
            }))
        }
    }
}

impl ListRequiredFields for SpendOutput {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![
            siren::Field {
                name: "address".to_owned(),
                class: vec!["bitcoin".to_owned(), "address".to_owned()],
                _type: Some("text".to_owned()),
                value: None,
                title: None,
            },
            siren::Field {
                name: "fee_per_wu".to_owned(),
                class: vec![
                    "bitcoin".to_owned(),
                    // feePerByte is deprecated because it is actually fee per WU
                    // Have to keep it around until clients are upgraded
                    "feePerByte".to_owned(),
                    "feePerWU".to_owned(),
                ],
                _type: Some("number".to_owned()),
                value: None,
                title: None,
            },
        ]
    }
}

impl IntoResponsePayload for ethereum::DeployContract {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        let ethereum::DeployContract {
            data,
            amount,
            gas_limit,
            chain_id,
        } = self;
        match query_params {
            ActionExecutionParameters::None {} => Ok(ActionResponseBody::EthereumDeployContract {
                data,
                amount,
                gas_limit,
                chain_id,
                network: chain_id.into(),
            }),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "ethereum::ContractDeploy",
                parameters: &["address", "fee_per_wu"],
            })),
        }
    }
}

impl ListRequiredFields for ethereum::DeployContract {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl IntoResponsePayload for ethereum::CallContract {
    fn into_response_payload(
        self,
        query_params: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        let ethereum::CallContract {
            to,
            data,
            gas_limit,
            chain_id,
            min_block_timestamp,
        } = self;
        match query_params {
            ActionExecutionParameters::None {} => Ok(ActionResponseBody::EthereumCallContract {
                contract_address: to,
                data,
                gas_limit,
                chain_id,
                network: chain_id.into(),
                min_block_timestamp,
            }),
            _ => Err(anyhow::Error::from(UnexpectedQueryParameters {
                action: "ethereum::SendTransaction",
                parameters: &["address", "fee_per_wu"],
            })),
        }
    }
}

impl ListRequiredFields for ethereum::CallContract {
    fn list_required_fields() -> Vec<siren::Field> {
        vec![]
    }
}

impl ListRequiredFields for Infallible {
    fn list_required_fields() -> Vec<siren::Field> {
        unreachable!("how did you manage to construct Infallible?")
    }
}

impl IntoResponsePayload for Infallible {
    fn into_response_payload(
        self,
        _: ActionExecutionParameters,
    ) -> anyhow::Result<ActionResponseBody> {
        unreachable!("how did you manage to construct Infallible?")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        ethereum::{Address as EthereumAddress, U256},
        swap_protocols::ledger::ethereum::ChainId,
    };
    use bitcoin::Address as BitcoinAddress;
    use std::str::FromStr;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(res, Ok(ActionExecutionParameters::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_wu=10.59";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(
            res,
            Ok(ActionExecutionParameters::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_wu: "10.59".to_string(),
            })
        );
    }

    #[test]
    fn call_contract_serializes_correctly_to_json_with_none() {
        let addr = EthereumAddress::from_str("0A81e8be41b21f651a71aaB1A85c6813b8bBcCf8").unwrap();
        let chain_id = ChainId::new(3);
        let contract = ActionResponseBody::EthereumCallContract {
            contract_address: addr,
            data: None,
            gas_limit: U256::from(1),
            chain_id,
            network: chain_id.into(),
            min_block_timestamp: None,
        };
        let serialized = serde_json::to_string(&contract).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"ethereum-call-contract","payload":{"contract_address":"0x0a81e8be41b21f651a71aab1a85c6813b8bbccf8","gas_limit":"0x1","chain_id":3,"network":"ropsten"}}"#
        );
    }

    #[test]
    fn bitcoin_send_amount_to_address_serializes_correctly_to_json() {
        let to = BitcoinAddress::from_str("2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9").unwrap();
        let amount = bitcoin::Amount::from_btc(1.0).unwrap();

        let input = &[
            ActionResponseBody::from(SendToAddress {
                to: to.clone(),
                amount,
                network: bitcoin::Network::Bitcoin,
            }),
            ActionResponseBody::from(SendToAddress {
                to: to.clone(),
                amount,
                network: bitcoin::Network::Testnet,
            }),
            ActionResponseBody::from(SendToAddress {
                to: to.clone(),
                amount,
                network: bitcoin::Network::Regtest,
            }),
        ];

        let expected = &[
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"mainnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"testnet"}}"#,
            r#"{"type":"bitcoin-send-amount-to-address","payload":{"to":"2N3pk6v15FrDiRNKYVuxnnugn1Yg7wfQRL9","amount":"100000000","network":"regtest"}}"#
        ];

        let actual = input
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<String>, serde_json::Error>>()
            .unwrap();

        assert_eq!(actual, expected);
    }
}
