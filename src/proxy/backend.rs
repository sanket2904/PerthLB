use std::{net::{SocketAddr, TcpStream}, collections::HashMap, sync::{RwLock, Arc}, time};
use rand::{distributions::WeightedIndex, prelude::Distribution, thread_rng};
#[derive(Debug, Clone)]
pub struct Wrr {
    pub healthy: bool,
    pub weight: u16,
    pub weights_index: usize,
}
#[derive(Debug, Clone)]
pub struct ServerPool {
    pub servers_map: HashMap<SocketAddr, Wrr>,
    pub weights: Vec<u16>,
    pub weighted_servers: Vec<SocketAddr>,
    pub dist: Option<WeightedIndex<u16>>,
}
#[derive(Debug)]
pub struct Backend {
    pub name: String,
    pub servers: Arc<RwLock<ServerPool>>,
    
}
impl ServerPool {
    pub fn new_servers(servers: HashMap<SocketAddr,Option<u16>>) -> Self {
        let mut back_servers = HashMap::new();
        let mut weights = Vec::new();
        let mut weight_servers = Vec::new();
        let mut index_count = 0;
        for (server, weight) in &servers {
            let mut server_weight = 1 as u16;
            match weight {
                Some(w) => {
                    server_weight = *w;
                },
                None => {}
            }
            weight_servers.push(*server);
            if let Ok(_) = TcpStream::connect_timeout(&server, time::Duration::from_secs(3)) {
                weights.push(server_weight);
                back_servers.insert(*server, Wrr {
                    healthy: true,
                    weight: server_weight,
                    weights_index: index_count,
                });
            } else {
                weights.push(0);
                back_servers.insert(*server, Wrr {
                    healthy: false,
                    weight: server_weight,
                    weights_index: index_count,
                });
            }
            index_count += 1;
        }
        let mut _server_store = None;
        match WeightedIndex::new(weights.clone()) {
            Ok(dist) => {
                _server_store = Some(dist);
            },
            Err(e) => {
                println!("WeightedIndex error: {:?}", e);
                _server_store = None;
            }
        }
        return ServerPool { servers_map: back_servers, weights, weighted_servers: weight_servers, dist: _server_store };
    }   
}

impl Backend {
    pub fn new(name: String, servers: HashMap<SocketAddr,Option<u16>> ) -> Self  {
        Backend {
            name,
            servers: Arc::new(RwLock::new(ServerPool::new_servers(servers))),
            
        }
    }
    // async fn update_backends_health(&self, updates: &HashMap<SocketAddr, bool>) {
    //     let mut servers = self.servers.write().unwrap();
    //     let mut weights = servers.weights.clone();
    //     let mut reset = false;
    //     for (server, health) in updates {
    //         if let Some(s) = servers.servers_map.get_mut(&server) {
    //             s.healthy = *health;
    //             reset = true;
    //             if *health {
    //                 weights[s.weights_index] = s.weight;
    //             } else {
    //                 weights[s.weights_index] = 0;
    //             }
    //         }
    //     }
    //     servers.weights = weights;
    //     if reset {
    //         match WeightedIndex::new(servers.weights.clone()) {
    //             Ok(dist) => {
    //                 servers.dist = Some(dist);
    //             },
    //             Err(e) => {
    //                 println!("WeightedIndex error: {:?}", e);
    //                 servers.dist = None;
    //             }
    //         }
    //     }
    // }
}


pub async fn get_next(backend: Arc<Backend>) -> Option<SocketAddr> {
    let srvs = backend.servers.read().unwrap();
    if let Some(dist) = &srvs.dist {
        if let Some(addr) = srvs.weighted_servers.get(dist.sample(&mut thread_rng())) {
            Some(*addr)
        } else {
            println!("error: no server found");
            None
        }
    } else {
        println!("Backend {} unhealthy; Unable to schedule", backend.name);
        None
    }
}