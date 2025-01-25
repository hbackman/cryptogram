mod block;
use block::Block;

fn main() {
    println!("Hello, world!");

    let b0 = Block::genesis();

    let b1 = Block::next(&b0);
    let b2 = Block::next(&b1);
    let b3 = Block::next(&b2);

    println!("{:?}", b0);

    println!("b0: {:?}", b0.hash);
    println!("b1: {:?}", b1.hash);
    println!("b2: {:?}", b2.hash);
    println!("b3: {:?}", b3.hash);
}
