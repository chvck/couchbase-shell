use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use std::ops::Add;
use tokio::time::Instant;

use crate::cli::user_builder::RoleAndDescription;
use crate::client::ManagementRequest;
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct UsersRoles {
    state: Arc<State>,
}

impl UsersRoles {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersRoles {
    fn name(&self) -> &str {
        "users roles"
    }

    fn signature(&self) -> Signature {
        Signature::build("users roles")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .named(
                "permission",
                SyntaxShape::String,
                "filter roles based on the permission string",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Shows all roles available on the cluster"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_async(self.state.clone(), args)
    }
}

fn run_async(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let permission = args
        .call_info
        .args
        .get("permission")
        .map(|id| id.as_string().ok())
        .flatten();

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = active_cluster.cluster().management_request(
            ManagementRequest::GetRoles {
                permission: permission.clone(),
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let roles: Vec<RoleAndDescription> = match response.status() {
            200 => match serde_json::from_str(response.content()) {
                Ok(m) => m,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Failed to decode response body {}",
                        e,
                    )));
                }
            },
            _ => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Request failed {}",
                    response.content(),
                )));
            }
        };

        for role_and_desc in roles {
            let mut collected = TaggedDictBuilder::new(Tag::default());

            collected.insert_value("cluster", identifier.clone());

            let role = role_and_desc.role();
            collected.insert_value("name", role_and_desc.display_name());
            collected.insert_value("role", role.name());
            collected.insert_value("bucket", role.bucket().unwrap_or_default());
            collected.insert_value("scope", role.scope().unwrap_or_default());
            collected.insert_value("collection", role.collection().unwrap_or_default());
            collected.insert_value("description", role_and_desc.description());

            entries.push(collected.into_value());
        }
    }

    Ok(entries.into())
}
