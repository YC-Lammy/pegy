pub fn main() {
    let now = std::time::Instant::now();
    let re = futures::executor::block_on(pegy::parse::<f64, _>("93853.54345"));
    let elapsed = now.elapsed();
    println!("{:#?}", re);
    println!("{}", elapsed.as_nanos());
    let now = std::time::Instant::now();
    let re = "93853.54345".parse::<f64>();
    let elapsed = now.elapsed();
    println!("{:#?}", re);
    println!("{}", elapsed.as_nanos())
}
