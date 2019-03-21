use crate::Network;
use futures::{Async, Stream};
use log::{debug, error};
use std::sync::Arc;
use std::time::Duration;
use std::usize;
use tokio::timer::Interval;

pub struct OutboundPeerService {
    pub stream_interval: Interval,
    pub network: Arc<Network>,
}

impl OutboundPeerService {
    pub fn new(network: Arc<Network>, try_connect_interval: Duration) -> Self {
        debug!(target: "network", "outbound peer service start, interval: {:?}", try_connect_interval);
        OutboundPeerService {
            network,
            stream_interval: Interval::new_interval(try_connect_interval),
        }
    }
}

impl Stream for OutboundPeerService {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        match try_ready!(self.stream_interval.poll().map_err(|_| ())) {
            Some(_tick) => {
                let connection_status = self.network.connection_status();
                let new_outbound = (connection_status.max_outbound
                    - connection_status.unreserved_outbound)
                    as usize;
                if new_outbound > 0 {
                    let attempt_peers = self
                        .network
                        .peer_store()
                        .read()
                        .peers_to_attempt(new_outbound as u32);
                    for (peer_id, addr) in attempt_peers.iter().filter_map(|(peer_id, addr)| {
                        if self.network.local_peer_id() != peer_id {
                            Some((peer_id.clone(), addr.clone()))
                        } else {
                            None
                        }
                    }) {
                        self.network.dial(&peer_id, addr);
                    }
                }
            }
            None => {
                error!(target: "network", "ckb outbound peer service stopped");
                return Ok(Async::Ready(None));
            }
        }
        Ok(Async::Ready(Some(())))
    }
}