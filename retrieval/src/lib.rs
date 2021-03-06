use err::Error;
use netpod::{Cluster, NodeConfig, NodeConfigCached};
use tokio::task::JoinHandle;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

pub mod cli;
pub mod client;
#[cfg(test)]
pub mod test;

pub fn spawn_test_hosts(cluster: Cluster) -> Vec<JoinHandle<Result<(), Error>>> {
    let mut ret = vec![];
    for node in &cluster.nodes {
        let node_config = NodeConfig {
            cluster: cluster.clone(),
            name: format!("{}:{}", node.host, node.port),
        };
        let node_config: Result<NodeConfigCached, Error> = node_config.into();
        let node_config = node_config.unwrap();
        let h = tokio::spawn(httpret::host(node_config));
        ret.push(h);
    }
    ret
}

pub async fn run_node(node_config: NodeConfigCached) -> Result<(), Error> {
    httpret::host(node_config).await?;
    Ok(())
}
