use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::{self, ErrorKind};
use std::thread::sleep;
use std::time::Duration;

use crate::scanner_rust::{Scanner, ScannerError};
use std::string::ToString;

const NET_DEV_PATH: &str = "/proc/net/dev";

#[derive(Debug, Clone, Eq)]
pub struct Network {
    pub interface: String,
    pub receive_bytes: u64,
    pub transmit_bytes: u64,
}

impl Hash for Network {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.interface.hash(state)
    }
}

impl PartialEq for Network {
    #[inline]
    fn eq(&self, other: &Network) -> bool {
        self.interface.eq(&other.interface)
    }
}

impl Network {
    pub fn get_networks() -> Result<Vec<Network>, ScannerError> {
        let mut sc = Scanner::scan_path(NET_DEV_PATH)?;

        for _ in 0..2 {
            if sc.next_line()?.is_none() {
                return Err(ScannerError::IOError(io::Error::new(
                    ErrorKind::UnexpectedEof,
                    "Cannot find any network interface.".to_string(),
                )));
            }
        }

        let mut networks = Vec::with_capacity(1);

        loop {
            let interface = sc.next()?;

            match interface {
                Some(mut interface) => {
                    interface.remove(interface.len() - 1);

                    let receive_bytes = match sc.next_u64()? {
                        Some(v) => v,
                        None => {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                format!("The format of interface `{}` is not correct.", interface),
                            )))
                        }
                    };

                    for _ in 0..7 {
                        if sc.next_u64()?.is_none() {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                format!("The format of interface `{}` is not correct.", interface),
                            )));
                        }
                    }

                    let transmit_bytes = match sc.next_u64()? {
                        Some(v) => v,
                        None => {
                            return Err(ScannerError::IOError(io::Error::new(
                                ErrorKind::UnexpectedEof,
                                format!("The format of interface `{}` is not correct.", interface),
                            )))
                        }
                    };

                    let network = Network {
                        interface,
                        receive_bytes,
                        transmit_bytes,
                    };

                    networks.push(network);

                    if sc.next_line()?.is_none() {
                        return Err(ScannerError::IOError(io::Error::new(
                            ErrorKind::UnexpectedEof,
                            "The format of networks is not correct.".to_string(),
                        )));
                    }
                }
                None => {
                    break;
                }
            }
        }

        Ok(networks)
    }
}

#[derive(Debug, Clone)]
pub struct Speed {
    pub receive: f64,
    pub transmit: f64,
}

#[derive(Debug, Clone)]
pub struct NetworkWithSpeed {
    pub network: Network,
    pub speed: Speed,
}

impl NetworkWithSpeed {
    pub fn get_networks_with_speed(
        interval: Duration,
    ) -> Result<Vec<NetworkWithSpeed>, ScannerError> {
        let mut pre_networks = Network::get_networks()?;

        let pre_networks_len = pre_networks.len();

        let mut pre_networks_hashset = HashSet::with_capacity(pre_networks_len);

        while let Some(network) = pre_networks.pop() {
            pre_networks_hashset.insert(network);
        }

        let seconds = interval.as_secs_f64();

        sleep(interval);

        let networks = Network::get_networks()?;

        let mut result = Vec::with_capacity(networks.len().min(pre_networks_len));

        for network in networks {
            if let Some(pre_network) = pre_networks_hashset.get(&network) {
                let d_receive = network.receive_bytes - pre_network.receive_bytes;
                let d_transmit = network.transmit_bytes - pre_network.transmit_bytes;

                let receive = d_receive as f64 / seconds;
                let transmit = d_transmit as f64 / seconds;

                let speed = Speed {
                    receive,
                    transmit,
                };

                let network_with_speed = NetworkWithSpeed {
                    network,
                    speed,
                };

                result.push(network_with_speed);
            }
        }

        Ok(result)
    }
}
