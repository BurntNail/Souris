use core::fmt::Display;
use http::StatusCode;
use ureq::{Agent, Response};
use crate::{client::ClientError, store::Store, values::Value};
use crate::client::{bool_to_string, CreationResult};

#[derive(Debug, Clone)]
pub struct SyncClient {
    //TODO: option to change protocol
    path: String, //path is never changed, so just maybe use arc<str> for cloning benefits
    port: u32,
    agent: Agent, //also internally arc-ed, so easy to clone
}

impl SyncClient {
    #[allow(clippy::result_large_err)]
    pub fn new(path: impl Display, port: u32) -> Result<Self, ClientError> {
        let path = path.to_string();
        let agent = Agent::new();

        let rsp = agent
            .get(&format!("http://{path}:{port}/healthcheck"))
            .call()?;
        let status = rsp.status_code()?;
        if status != StatusCode::OK {
            return Err(ClientError::ServerNotHealthy(status));
        }

        Ok(Self { path, port, agent })
    }

    #[allow(clippy::result_large_err)]
    pub fn get_all_dbs(&self) -> Result<Vec<String>, ClientError> {
        let rsp = self
            .agent
            .get(&format!(
                "http://{}:{}/v1/get_all_db_names",
                self.path, self.port
            ))
            .call()?;

        let body = rsp.body()?;
        Ok(serde_json::from_slice(&body)?)
    }

    #[allow(clippy::result_large_err)]
    pub fn create_new_db(&self, overwrite_existing: bool, name: &str) -> Result<bool, ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/add_db", self.path, self.port))
            .query(
                "overwrite_existing",
                bool_to_string(overwrite_existing),
            )
            .query("db_name", name)
            .call()?;

        Ok(match rsp.status_code()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    #[allow(clippy::result_large_err)]
    pub fn get_store(&self, db_name: &str) -> Result<Store, ClientError> {
        let rsp = self
            .agent
            .get(&format!("http://{}:{}/v1/get_db", self.path, self.port))
            .query("db_name", db_name)
            .call()?;
        let body = rsp.body()?;
        println!("Received body from client");
        Ok(Store::deser(&body)?)
    }

    #[allow(clippy::result_large_err)]
    pub fn add_db_with_contents(
        &self,
        overwrite_existing: bool,
        name: &str,
        store: &Store,
    ) -> Result<bool, ClientError> {
        let store = store.ser();

        let rsp = self
            .agent
            .put(&format!(
                "http://{}:{}/v1/add_db_with_content",
                self.path, self.port
            ))
            .query(
                "overwrite_existing",
                bool_to_string(overwrite_existing),
            )
            .query("db_name", name)
            .send_bytes(&store)?;
        Ok(match rsp.status_code()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    #[allow(clippy::result_large_err)]
    pub fn add_entry_to_db(
        &self,
        database_name: &str,
        create_new_database_if_needed: bool,
        overwrite_existing_key: bool,
        key: &str,
        value: &Value,
    ) -> Result<CreationResult, ClientError> {
        let value = value.ser(None);
        let rsp = self
            .agent
            .put(&format!("http://{}:{}/v1/add_kv", self.path, self.port))
            .query("db_name", database_name)
            .query("key", key)
            .query("create_new_database", bool_to_string(create_new_database_if_needed))
            .query("overwrite_key", bool_to_string(overwrite_existing_key))
            .send_bytes(&value)?;
        
        rsp.status_code()?;
        
        Ok(rsp.into_json()?)
    }

    #[allow(clippy::result_large_err)]
    pub fn remove_entry_from_db(&self, database_name: &str, key: &str) -> Result<(), ClientError> {
        self.agent
            .post(&format!("http://{}:{}/v1/rm_kv", self.path, self.port))
            .query("db_name", database_name)
            .query("key", key)
            .call()?;
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    pub fn remove_db(&self, database_name: &str) -> Result<(), ClientError> {
        self.agent
            .post(&format!("http://{}:{}/v1/rm_db", self.path, self.port))
            .query("db_name", database_name)
            .call()?;
        Ok(())
    }
}

trait ResponseExt {
    #[allow(clippy::result_large_err)]
    fn status_code(&self) -> Result<StatusCode, ClientError>;
    fn body(self) -> Result<Vec<u8>, std::io::Error>;
}

impl ResponseExt for Response {
    fn status_code(&self) -> Result<StatusCode, ClientError> {
        Ok(StatusCode::try_from(self.status())?)
    }
    fn body(self) -> Result<Vec<u8>, std::io::Error> {
        let mut reader = self.into_reader();
        let mut output = vec![];
        loop {
            let mut tmp = [0_u8; 64];
            match reader.read(&mut tmp)? {
                0 => break,
                n => output.extend(&tmp[0..n]),
            }
        }

        Ok(output)
    }
}
