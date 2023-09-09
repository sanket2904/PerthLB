mod proxy;
mod config;
fn main() {
    let args : Vec<String> = std::env::args().collect();
    let file = &args[1];
    let config = config::Config::new(file).unwrap();
    let mut server = proxy::Server::new(config);
    if let Err(e) = server.run() {
        println!("error: {:?}", e);
    }
}




