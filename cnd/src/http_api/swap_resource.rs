#![allow(clippy::type_repetition_in_bounds)]
use crate::{
    db::{Swap, SwapTypes},
    http_api::{
        action::ToSirenAction,
        route_factory::swap_path,
        routes::rfc003::{LedgerState, SwapCommunication, SwapState},
        Http,
    },
    swap_protocols::{
        actions::Actions,
        asset::Asset,
        rfc003::{self, state_store::StateStore, Ledger},
        HashFunction, SwapId, SwapProtocol,
    },
};
use http_api_problem::HttpApiProblem;
use libp2p::PeerId;
use serde::Serialize;
use warp::http::StatusCode;

#[derive(Debug, Serialize)]
#[serde(
    bound = "Http<AL>: Serialize, Http<BL>: Serialize, Http<AA>: Serialize, Http<BA>: Serialize,\
             Http<AL::Identity>: Serialize, Http<BL::Identity>: Serialize,\
             Http<AL::HtlcLocation>: Serialize, Http<BL::HtlcLocation>: Serialize,\
             Http<AL::Transaction>: Serialize, Http<BL::Transaction>: Serialize,"
)]
pub struct SwapResource<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset, S: Serialize> {
    pub id: Http<SwapId>,
    pub role: String,
    pub counterparty: Http<PeerId>,
    pub protocol: Http<SwapProtocol>,
    pub status: SwapStatus,
    pub parameters: SwapParameters<AL, BL, AA, BA>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<S>,
}

#[derive(Debug, Serialize)]
#[serde(
    bound = "Http<AL>: Serialize, Http<BL>: Serialize, Http<AA>: Serialize, Http<BA>: Serialize"
)]
pub struct SwapParameters<AL, BL, AA, BA> {
    alpha_ledger: Http<AL>,
    beta_ledger: Http<BL>,
    alpha_asset: Http<AA>,
    beta_asset: Http<BA>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SwapStatus {
    InProgress,
    Swapped,
    NotSwapped,
    InternalFailure,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<rfc003::Request<AL, BL, AA, BA>>
    for SwapParameters<AL, BL, AA, BA>
{
    fn from(request: rfc003::Request<AL, BL, AA, BA>) -> Self {
        Self {
            alpha_ledger: Http(request.alpha_ledger),
            alpha_asset: Http(request.alpha_asset),
            beta_ledger: Http(request.beta_ledger),
            beta_asset: Http(request.beta_asset),
        }
    }
}

pub enum IncludeState {
    Yes,
    No,
}

pub fn build_rfc003_siren_entity<S: StateStore>(
    state_store: &S,
    swap: Swap,
    types: SwapTypes,
    include_state: IncludeState,
) -> anyhow::Result<siren::Entity> {
    let id = swap.swap_id;

    with_swap_types!(types, {
        let state = state_store
            .get::<ROLE>(&id)?
            .ok_or_else(|| anyhow::anyhow!("state store did not contain an entry for {}", id))?;

        let communication = SwapCommunication::from(state.swap_communication.clone());
        let alpha_ledger = LedgerState::from(state.alpha_ledger_state.clone());
        let beta_ledger = LedgerState::from(state.beta_ledger_state.clone());
        let parameters = SwapParameters::from(state.clone().request());
        let actions = state.clone().actions();

        let error = state.error;
        let status = SwapStatus::new(
            communication.status,
            alpha_ledger.status,
            beta_ledger.status,
            &error,
        );

        let swap = SwapResource {
            id: Http(id),
            status,
            protocol: Http(SwapProtocol::Rfc003(HashFunction::Sha256)),
            parameters,
            role: swap.role.to_string(),
            counterparty: Http(swap.counterparty),
            state: match include_state {
                IncludeState::Yes => Some(SwapState::<AL, BL> {
                    communication,
                    alpha_ledger,
                    beta_ledger,
                }),
                IncludeState::No => None,
            },
        };

        let entity = siren::Entity::default()
            .with_class_member("swap")
            .with_properties(swap)
            .map_err(|e| {
                log::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?
            .with_link(siren::NavigationalLink::new(&["self"], swap_path(id)))
            .with_link(siren::NavigationalLink::new(
                &["human-protocol-spec"],
                "https://github.com/comit-network/RFCs/blob/master/RFC-003-SWAP-Basic.md",
            ));

        let entity = actions.into_iter().fold(entity, |acc, action| {
            let action = action.to_siren_action(&id);
            acc.with_action(action)
        });

        Ok(entity)
    })
}
