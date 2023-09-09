mod proxy;
mod config;



fn main() {
    let file = "config.toml";
    let config = config::Config::new(file).unwrap();
    let mut server = proxy::Server::new(config);
    if let Err(e) = server.run() {
        println!("error: {:?}", e);
    }
}




