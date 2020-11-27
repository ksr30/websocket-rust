
use std::env;

use futures::sink::SinkExt;
use futures::stream::StreamExt;
use websocket_lite::{Message, Opcode, Result};
use serde_json::Value;
use substrate_subxt::{ClientBuilder, PairSigner, NodeTemplateRuntime, Client};
use sp_keyring::AccountKeyring;
use substrate_subxt::generic_asset::{CreateCall, AssetOptions, PermissionsV1, Owner};
use substrate_subxt::polkadex::{RegisterNewOrderbookCall, OrderType, SubmitOrder};
use substrate_subxt::sp_runtime::testing::H256;
use substrate_subxt::sp_runtime::sp_std::str::FromStr;
use substrate_subxt::polkadex::OrderType::AskLimit;

const UNIT: u128 = 1_000_000_000_000;
const UNIT_REP: u128 = 1_000_000_000;

// struct Data {
// e: String,  // Event type
// E: f64,   // Event time
// s: f64,    // Symbol
// a: 12345,       // Aggregate trade ID
// p: "0.001",     // Price
// q: "100",       // Quantity
// f: 100,         // First trade ID
// l: 105,         // Last trade ID
// T: 123456785,   // Trade time
// m: true,        // Is the buyer the market maker?
// M: true         // Ignore
// }

async fn run() -> Result<()> {
    let client = ClientBuilder::<NodeTemplateRuntime>::new()
        .set_url("ws://127.0.0.1:9944")
        .build()
        .await?;

    initial_calls(client.clone()).await?;

    let url = env::args().nth(1).unwrap_or_else(|| "wss://stream.binance.com:9443/ws/btcusdt@aggTrade".to_owned());
    let builder = websocket_lite::ClientBuilder::new(&url)?;
    let mut ws_stream = builder.async_connect().await?;
    // let str : String = String::from(r#"{ "method": "SUBSCRIBE", "params": [ "btcusdt@trade" ], "id": 1 }"#);
    // ws_stream.send(Message::text(str)).await;
    // ws_stream.send(Message::text(String::from("singh"))).await;

    let mut alice_nonce: u32 = 2;



    loop {
        let msg: Option<Result<Message>> = ws_stream.next().await;

        let msg = if let Some(msg) = msg {
            msg
        } else {
            break;
        };

        let msg = if let Ok(msg) = msg {
            msg
        } else {
            //let _ = ws_stream.send(Message::close(None)).await;
            break;
        };

        match msg.opcode() {
            Opcode::Text => {
                let data =  msg.as_text().unwrap();
                let v: Value = serde_json::from_str(data)?;
                println!("{}", v["a"].to_owned().as_f64().unwrap()*1000f64);
                alice_nonce = alice_nonce +1;
                repetitive_calls(client.clone(),v, alice_nonce).await?;

                //ws_stream.send(msg).await?
            }
            Opcode::Binary =>  {},  // ws_stream.send(msg).await?,
            Opcode::Ping => ws_stream.send(Message::pong(msg.into_data())).await?,
            Opcode::Close => {
                let _ = ws_stream.send(Message::close(None)).await;
                break;
            }
            Opcode::Pong => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        run().await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    })
        .await
        .unwrap();
}

async fn repetitive_calls(client: Client<NodeTemplateRuntime> ,v: Value, alice_nonce: u32) -> Result<()>{

    let submit_trade_call = SubmitOrder{
        order_type: if v["m"].as_bool().unwrap() {OrderType::BidLimit} else {OrderType::AskLimit},
        trading_pair: H256::from_str("f28a3c76161b8d5723b6b8b092695f418037c747faa2ad8bc33d8871f720aac9").unwrap(),
        price: (1000f64*v["p"].to_owned().as_f64().unwrap()).round() as u128 * UNIT_REP,
        quantity: (1000f64*v["q"].to_owned().as_f64().unwrap()).round() as u128 * UNIT_REP
    };


    let mut signer = PairSigner::<NodeTemplateRuntime, _>::new(AccountKeyring::Alice.pair());
    signer.set_nonce(alice_nonce);
    let result = client.submit(submit_trade_call.clone(), &signer).await?;
    println!(" Trade Placed #{}",result);
    Ok(())

}

async fn initial_calls(client: Client<NodeTemplateRuntime>) -> Result<()> {
    let client = ClientBuilder::<NodeTemplateRuntime>::new()
        .set_url("ws://127.0.0.1:9944")
        .build()
        .await?;

    let mut signer = PairSigner::<NodeTemplateRuntime, _>::new(AccountKeyring::Alice.pair());
    // let to = AccountKeyring::Bob.to_account_id().into();

    let asset_call = CreateCall {
        options: AssetOptions {
            initial_issuance: 10 * UNIT,
            permissions: PermissionsV1 {
                update: Owner::None,
                mint: Owner::None,
                burn: Owner::None,
            },
        }
    };
    let mut alice_nonce: u32 = 0;

    // Create BTC
    signer.set_nonce(alice_nonce);
    let result = client.submit(asset_call.clone(), &signer).await?;
    println!(" Created Asset #1: {}", result);
    alice_nonce = alice_nonce + 1;

    // Create USD
    signer.set_nonce(alice_nonce);
    let result = client.submit(asset_call.clone(), &signer).await?;
    println!(" Created Asset #1: {}", result);
    alice_nonce = alice_nonce + 1;

    // Register BTC/USD Orderbook
    let register_orderbook_call = RegisterNewOrderbookCall {
        quote_asset_id: 2 as u32,
        base_asset_id: 1 as u32
    };
    signer.set_nonce(alice_nonce);
    let result = client.submit(register_orderbook_call, &signer).await?;
    println!(" Order book Registered: {}", result);
    Ok(())
}
