use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use log::debug;
use nu_cli::ActionStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct Analytics {
    state: Arc<State>,
}

impl Analytics {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for Analytics {
    fn name(&self) -> &str {
        "analytics"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics")
            .required("statement", SyntaxShape::String, "the analytics statement")
            .named(
                "bucket",
                SyntaxShape::String,
                "the bucket to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Performs an analytics query"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<State>, args: CommandArgs) -> Result<ActionStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let statement = args.nth(0).expect("need statement").as_string()?;

    let active_cluster = state.active_cluster();
    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
        .or_else(|| active_cluster.active_bucket())
    {
        Some(v) => Some(v),
        None => None,
    };
    let scope = match args.call_info.args.get("scope") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };

    let maybe_scope = if bucket.is_some() && scope.is_some() {
        Some((bucket.unwrap().clone(), scope.unwrap().clone()))
    } else {
        None
    };

    let with_meta = args.get_flag::<bool>("with-meta").unwrap().is_some();

    debug!("Running analytics query {}", &statement);

    let response = active_cluster.cluster().analytics_query_request(
        AnalyticsQueryRequest::Execute {
            statement: statement.clone(),
            scope: maybe_scope,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    )?;

    if with_meta {
        let content: serde_json::Value = serde_json::from_str(response.content())?;
        return Ok(ActionStream::one(convert_json_value_to_nu_value(
            &content,
            Tag::default(),
        )?));
    } else {
        let mut content: HashMap<String, serde_json::Value> =
            serde_json::from_str(response.content())?;
        let removed = if content.contains_key("errors") {
            content.remove("errors").unwrap()
        } else {
            content.remove("results").unwrap()
        };

        let values = removed
            .as_array()
            .unwrap()
            .iter()
            .map(|a| convert_json_value_to_nu_value(a, Tag::default()).unwrap())
            .collect::<Vec<_>>();
        return Ok(OutputStream::from(values).into());
    }
}
