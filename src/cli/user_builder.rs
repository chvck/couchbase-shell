use serde_derive::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt::{self, Debug};

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum AuthDomain {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "external")]
    External,
}

impl fmt::Display for AuthDomain {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    #[serde(rename = "role")]
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bucket_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collection_name: Option<String>,
}

impl Role {
    pub fn new(
        name: String,
        bucket_name: Option<String>,
        scope_name: Option<String>,
        collection_name: Option<String>,
    ) -> Self {
        Self {
            name,
            bucket_name,
            scope_name,
            collection_name,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bucket(&self) -> Option<String> {
        self.bucket_name.clone()
    }

    pub fn scope(&self) -> Option<String> {
        self.scope_name.clone()
    }

    pub fn collection(&self) -> Option<String> {
        self.collection_name.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct RoleAndDescription {
    #[serde(flatten)]
    role: Role,
    name: String,
    desc: String,
}

impl RoleAndDescription {
    pub fn role(&self) -> &Role {
        self.role.borrow()
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.desc
    }
}

#[derive(Debug, Deserialize)]
pub struct Origin {
    #[serde(rename = "type")]
    origin_type: String,
    name: Option<String>,
}

impl Origin {
    pub fn new(origin_type: String, name: Option<String>) -> Self {
        Self { origin_type, name }
    }

    pub fn origin_type(&self) -> &str {
        self.origin_type.as_str()
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct RoleAndOrigins {
    #[serde(flatten)]
    role: Role,
    origins: Vec<Origin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    #[serde(rename = "id")]
    name: String,
    description: Option<String>,
    roles: Vec<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ldap_group_ref")]
    ldap_group: Option<String>,
}

impl Group {
    pub fn new(name: String, roles: Vec<Role>) -> Self {
        Self {
            name,
            description: None,
            roles,
            ldap_group: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description)
    }

    pub fn roles(&self) -> &Vec<Role> {
        self.roles.as_ref()
    }

    pub fn roles_mut(&mut self) -> &mut Vec<Role> {
        self.roles.as_mut()
    }

    pub fn ldap_group_reference(&self) -> Option<String> {
        self.ldap_group.clone()
    }

    pub fn set_ldap_group_reference(&mut self, reference: String) {
        self.ldap_group = Some(reference)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "id")]
    username: String,
    #[serde(rename = "name")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<String>>,
    roles: Vec<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

impl User {
    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn display_name(&self) -> Option<String> {
        self.display_name.clone()
    }

    pub fn set_display_name(&mut self, display_name: String) {
        self.display_name = Some(display_name);
    }

    pub fn groups(&self) -> Option<&Vec<String>> {
        self.groups.as_ref()
    }

    pub fn groups_mut(&mut self) -> Option<&mut Vec<String>> {
        self.groups.as_mut()
    }

    pub fn roles(&self) -> &Vec<Role> {
        self.roles.as_ref()
    }

    pub fn roles_mut(&mut self) -> &mut Vec<Role> {
        self.roles.as_mut()
    }

    pub fn set_password(&mut self, password: String) {
        self.password = Some(password)
    }
}

pub struct UserBuilder {
    username: String,
    display_name: Option<String>,
    groups: Option<Vec<String>>,
    roles: Vec<Role>,
    password: Option<String>,
}

impl UserBuilder {
    pub fn new(username: String, password: Option<String>, roles: Vec<Role>) -> Self {
        Self {
            username,
            display_name: None,
            groups: None,
            roles,
            password,
        }
    }

    pub fn display_name(mut self, name: String) -> UserBuilder {
        self.display_name = Some(name);
        self
    }

    pub fn groups(mut self, groups: Vec<String>) -> UserBuilder {
        self.groups = Some(groups);
        self
    }

    pub fn build(self) -> User {
        User {
            username: self.username,
            display_name: self.display_name,
            groups: self.groups,
            roles: self.roles,
            password: self.password,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UserAndMetadata {
    #[serde(rename = "id")]
    username: String,
    #[serde(rename = "name")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    groups: Option<Vec<String>>,
    roles: Vec<RoleAndOrigins>,
    domain: AuthDomain,
    password_change_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_groups: Option<Vec<String>>,
}

impl UserAndMetadata {
    pub fn domain(&self) -> AuthDomain {
        self.domain.clone()
    }

    pub fn user(&self) -> User {
        let mut builder = UserBuilder::new(self.username.clone(), None, self.user_roles());
        if let Some(display_name) = &self.display_name {
            builder = builder.display_name(display_name.clone());
        }
        if let Some(groups) = &self.groups {
            builder = builder.groups(groups.clone());
        }

        builder.build()
    }

    pub fn effective_roles(&self) -> &Vec<RoleAndOrigins> {
        self.roles.as_ref()
    }

    pub fn password_changed(&self) -> Option<String> {
        self.password_change_date.clone()
    }

    pub fn external_groups(&self) -> Option<&Vec<String>> {
        self.groups.as_ref()
    }

    fn user_roles(&self) -> Vec<Role> {
        self.roles
            .iter()
            .filter(|role| {
                role.origins
                    .iter()
                    .any(|origin| origin.origin_type.as_str() == "user")
            })
            .map(|role| {
                Role::new(
                    role.role.name.clone(),
                    role.role.bucket_name.clone(),
                    role.role.scope_name.clone(),
                    role.role.collection_name.clone(),
                )
            })
            .collect()
    }
}
