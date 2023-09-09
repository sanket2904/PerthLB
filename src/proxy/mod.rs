mod backend;
use std::{sync::Arc, net::SocketAddr, collections::HashMap, str::FromStr};
use backend::Backend;
use tokio::try_join;


use crate::{config::{BaseConfig, Config}, proxy::backend::ServerPool};


#[derive(Debug)]
pub struct Proxy {
    pub name: String,
    pub listen_addr: SocketAddr,
    pub backend: Arc<Backend>
}


#[derive(Debug)]
pub struct Server {
    pub proxies: Vec<Arc<Proxy>>,
    pub rx : std::sync::mpsc::Receiver<BaseConfig>
}

impl Server {
    pub fn new(config: Config) -> Server {
        let sub = config.clone().subscribe();
        let mut server = Server {
            proxies: Vec::new(),
            rx: sub
        };
        for (name, front) in config.base.frontends.iter() {
            let mut backend = HashMap::new();
           
            match config.base.backends.get(&front.backend) {
                Some(back) => {
                    //  create backend
                    for(_,addr) in &back.servers {
                        let listen_addr:SocketAddr = FromStr::from_str(&addr.addr).ok().unwrap();
                        backend.insert(listen_addr, addr.weight);
                    }
                },
                None => {
                    println!("backend {} not found", front.backend);
                    continue;
                }
            };
            if !backend.is_empty() {
                let listen_addr:SocketAddr = FromStr::from_str(&front.listen_addr).ok().unwrap();
                let backend = Backend::new(
                    front.backend.clone(),
                    backend,
                );
                let lb = Proxy{ 
                    name: name.clone(),
                    listen_addr,
                    backend: Arc::new(backend)
                };
                server.proxies.push(Arc::new(lb));
            } else {
                println!("error: backend {} is empty", front.backend);
            }

        }
        server
    }

    async fn config_sync(&mut self) {
        loop {
            match self.rx.recv() {
                Ok(config) => {
                    println!("file changed: {:?}", config);
                    for (name, backend) in config.backends {
                        let mut backend_servers = HashMap::new();

                        for (_, server) in backend.servers {
                            let listen_addr:SocketAddr = FromStr::from_str(&server.addr).ok().unwrap();
                            backend_servers.insert(listen_addr, server.weight);
                        }
                        let server_pool = ServerPool::new_servers(backend_servers);
                        for proxy in self.proxies.iter() {
                            if proxy.backend.name == name   {
                                println!("update backend: {:?}", name);
                                *proxy.backend.servers.write().unwrap() = server_pool.clone();
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("error: {:?}", e);
                    break;
                }
            }
        }
    }

    #[tokio::main]
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>  {
        let proxies = self.proxies.clone();
        for proxy in proxies.iter() {
            // let back = proxy.backend.clone();
            let p = proxy.clone();
            tokio::spawn(async move {
                if let Err(e) = run_server(p.clone()).await {
                    println!("error: {:?}", e);
                    return;
                }
            });

        }
        self.config_sync().await;
        Ok(())
    }
}


async fn run_server(lb: Arc<Proxy>) ->  Result<(), Box<dyn std::error::Error>>  {
    println!("start proxy: {:?}", lb);

    let listner = tokio::net::TcpListener::bind(&lb.listen_addr).await.unwrap();

    while let Ok((inbound,_)) = listner.accept().await {
        let thread_lb = lb.clone();
        // let err_lb = lb.clone();

        tokio::spawn(async move {
            let _ = process(inbound,thread_lb).await;
        });
    }

    Ok(())
}

async fn process(mut inbound:tokio::net::TcpStream, lb: Arc<Proxy>) -> Result<(), Box<dyn std::error::Error>>  {
    let join = crate::proxy::backend::get_next(lb.backend.clone()).await;
    match join  {
        Some(server_addr) => {
            let mut server = tokio::net::TcpStream::connect(server_addr).await.unwrap();
            let (mut ri, mut wi) = inbound.split();
            let (mut ro, mut wo) = server.split();
            let client_to_server = tokio::io::copy(&mut ri, &mut wo);
            let server_to_client = tokio::io::copy(&mut ro, &mut wi);
            let (bytes_tx, bytes_rx) = try_join!(client_to_server,server_to_client).unwrap();
            println!("bytes_tx: {}, bytes_rx: {}", bytes_tx, bytes_rx);
        },
        None => {
            println!("error: no server found");
        }
    };

    Ok(())
}