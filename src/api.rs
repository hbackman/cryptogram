// #![allow(unused)]
//
// use crate::block::Block;
// use warp::Filter;
// use serde::{Deserialize, Serialize};
// use std::sync::{Arc, Mutex};
//
// #[derive(Deserialize)]
// struct TransactionRequest {
//     from:   String,
//     to:     String,
//     amount: u64,
// }
//
// #[derive(Serialize)]
// struct TransactionResponse {
//     status: String,
// }
//
// #[derive(Deserialize)]
// struct MiningRequest {}
//
// #[derive(Serialize)]
// struct MiningResponse {
//     block: Block,
// }
//
// pub async fn start_api() {
//     let blockchain: Arc<Mutex<Vec<Block>>> = Arc::new(Mutex::new(Vec::new()));
//
//     blockchain
//         .lock()
//         .unwrap()
//         .push(Block::genesis());
//
//     let transaction = warp::path("transaction")
//         .and(warp::post())
//         .and(warp::body::json())
//         .map(handle_transaction);
//
//     let mine = warp::path("mine")
//         .and(warp::post())
//         .and(warp::body::json())
//         // pass blockchain into mining handler
//         .and(warp::any().map(move || blockchain.clone()))
//         .map(handle_mining);
//
//     let routes = transaction.or(mine);
//
//     println!("Server running at http://127.0.0.1:3030");
//
//     warp::serve(routes)
//         .run(([127, 0, 0, 1], 3030))
//         .await;
// }
//
//
// fn handle_transaction(req: TransactionRequest) -> impl warp::Reply {
//     let f = req.from;
//     let t = req.to;
//     let a = req.amount;
//
//     println!("{} is sending {} to {}", f, a, t);
//
//     warp::reply::json(&TransactionResponse{
//         status: "success".to_string(),
//     })
// }
//
// fn handle_mining(req: MiningRequest, blockchain: Arc<Mutex<Vec<Block>>>) -> impl warp::Reply {
//     let miner_addr = "q3nf394hjg-random-miner-address-34nf3i4nflkn3oi";
//
//     let last_block;
//     let next_block;
//
//     {
//         let mut chain = blockchain
//             .lock()
//             .unwrap();
//
//         last_block = chain
//             .last()
//             .expect("blockchain is empty, that's sus 🤔");
//
//         next_block = Block::next(last_block, miner_addr.to_string());
//
//         chain.push(next_block.clone());
//     }
//
//     println!("🔥 new block mined");
//     dump_chain(blockchain.clone());
//
//     warp::reply::json(&MiningResponse{
//         block: next_block,
//     })
// }
//
// fn dump_chain(blockchain: Arc<Mutex<Vec<Block>>>) {
//     let chain = blockchain.lock().unwrap();
//
//     for block in chain.iter() {
//         println!("{}: {}", block.hash, block.data);
//     }
// }
//
