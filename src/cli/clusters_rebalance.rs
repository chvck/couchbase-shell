use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, OutputStream, TaggedDictBuilder};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, SyntaxShape};
use nu_source::Tag;
use std::sync::Arc;

pub struct ClustersRebalance {
    state: Arc<State>,
}

impl ClustersRebalance {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for ClustersRebalance {
    fn name(&self) -> &str {
        "clusters rebalance"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters rebalance").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Triggers a cluster rebalance"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_rebalance(args, self.state.clone()).await
    }
}

async fn clusters_rebalance(args: CommandArgs, state: Arc<State>) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut results: Vec<ReturnValue> = vec![];
    for identifier in cluster_identifiers {
        let core = match state.clusters().get(&identifier) {
            Some(c) => c.cluster().core(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let (sender, receiver) = oneshot::channel();
        let request = GenericManagementRequest::new(
            sender,
            format!("/controller/rebalance"),
            "post".into(),
            None,
        );
        core.send(Request::GenericManagementRequest(request));

         match receiver.await {
            Ok(_) => {},
            Err(e) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value("error", format!("{}", e));
                results.push(Ok(ReturnSuccess::Value(collected.into_value())));
            }
        };
    }

    Ok(OutputStream::from(results))
}
