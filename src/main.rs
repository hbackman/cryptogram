mod block;
use block::Block;

use warp::Filter;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TransactionResponse {
    status: String,
}

#[derive(Deserialize)]
struct TransactionRequest {
    from:   String,
    to:     String,
    amount: u64,
}

#[tokio::main]
async fn main() {
    let transaction = warp::path("transaction")
        .and(warp::post())
        .and(warp::body::json())
        .map(|req: TransactionRequest| {
            let f = req.from;
            let t = req.to;
            let a = req.amount;

            println!("{} is sending {} to {}", f, a, t);

            warp::reply::json(&TransactionResponse{
                status: "success".to_string(),
            })
        });

    println!("Server running at http://127.0.0.1:3030");

    warp::serve(transaction)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

// fn main () {
//     println!("Hello, world!");
//
//     let b0 = Block::genesis();
//
//     let b1 = Block::next(&b0);
//     let b2 = Block::next(&b1);
//     let b3 = Block::next(&b2);
//
//     println!("{:?}", b0);
//
//     println!("b0: {:?}", b0.hash);
//     println!("b1: {:?}", b1.hash);
//     println!("b2: {:?}", b2.hash);
//     println!("b3: {:?}", b3.hash);
// }
