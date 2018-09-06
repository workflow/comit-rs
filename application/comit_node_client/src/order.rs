use api_client::{ApiClient, BuyOrderRequestBody, TradingServiceError};
use offer::Symbol;
use uuid::Uuid;

pub fn run<C: ApiClient>(
    client: &C,
    symbol: Symbol,
    uid: Uuid,
    success_address: String,
    refund_address: String,
) -> Result<String, TradingServiceError> {
    let order_request_body = BuyOrderRequestBody::new(success_address, refund_address);

    let request_to_fund = client.request_order(&symbol, uid, &order_request_body)?;

    Ok(format!(
        "#### Trade id: {} ####\n\
         You have accepted the order!\n\
         Please send {} to the following address to get your {}:\n\
         {}\n\
         Once you transaction has 6 confirmations, the {} contract will be deployed.\n\
         You can then get your redeem details with:\n\
         comit_node_client redeem --symbol={} --uid={}",
        uid,
        request_to_fund.btc_amount,
        request_to_fund.eth_amount,
        //TODO: make a payment address
        request_to_fund.address_to_fund,
        symbol.get_traded_currency(),
        symbol,
        uid,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_client::FakeApiClient;

    #[test]
    fn accept_order_happy_path() {
        let symbol = "ETH-BTC".parse().unwrap();
        let uid = "27b36adf-eda3-4684-a21c-a08a84f36fb1".parse().unwrap();

        let response = run(
            &FakeApiClient::default(),
            symbol,
            uid,
            "0x00a329c0648769a73afac7f9381e08fb43dbea72".to_string(),
            "tb1qj3z3ymhfawvdp4rphamc7777xargzufztd44fv".to_string(),
        ).unwrap();

        assert_eq!(
            response,
            "#### Trade id: 27b36adf-eda3-4684-a21c-a08a84f36fb1 ####\n\
             You have accepted the order!\n\
             Please send 1001 BTC to the following address to get your 140 ETH:\n\
             bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap\n\
             Once you transaction has 6 confirmations, the ETH contract will be deployed.\n\
             You can then get your redeem details with:\n\
             comit_node_client redeem --symbol=ETH-BTC --uid=27b36adf-eda3-4684-a21c-a08a84f36fb1"
        );
    }
}
