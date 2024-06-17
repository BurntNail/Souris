use core::fmt::Display;
use std::sync::Arc;

use http::StatusCode;
use ureq::{Agent, Response};

use crate::{client::ClientError, store::Store, values::Value};

#[derive(Debug, Clone)]
pub struct SyncClient {
    path: Arc<str>, //path is never changed, so just use arc<str> for cloning benefits
    port: u32,
    agent: Agent, //also internally arc-ed, so easy to clone
}

impl SyncClient {
    pub fn new(path: impl Display, port: u32) -> Result<Self, ClientError> {
        let path = path.to_string().into();
        let agent = Agent::new();

        let rsp = agent
            .get(&format!("http://{path}:{port}/healthcheck"))
            .call()?;
        if rsp.status() != StatusCode::OK {
            return Err(ClientError::ServerNotHealthy(StatusCode::try_from(
                rsp.status(),
            )?));
        }

        Ok(Self { path, port, agent })
    }

    pub fn get_all_dbs(&self) -> Result<Vec<String>, ClientError> {
        let rsp = self
            .agent
            .get(&format!(
                "http://{}:{}/v1/get_all_db_names",
                self.path, self.port
            ))
            .call()?;
        rsp.error_for_status()?;

        let body = rsp.body()?;
        Ok(serde_json::from_slice(&body)?)
    }

    pub fn create_new_db(&self, overwrite_existing: bool, name: &str) -> Result<bool, ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/add_db", self.path, self.port))
            .query(
                "overwrite_existing",
                if overwrite_existing { "true" } else { "false" },
            )
            .query("db_name", name)
            .call()?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn get_store(&self, db_name: &str) -> Result<Store, ClientError> {
        let rsp = self
            .agent
            .get(&format!("http://{}:{}/v1/get_db", self.path, self.port))
            .query("db_name", db_name)
            .call()?;
        rsp.error_for_status()?;
        let body = rsp.body()?;
        println!("Received body from client");
        Ok(Store::deser(&body)?)
    }

    pub fn add_db_with_contents(
        &self,
        overwrite_existing: bool,
        name: &str,
        store: &Store,
    ) -> Result<bool, ClientError> {
        let store = store.ser()?;

        let rsp = self
            .agent
            .put(&format!(
                "http://{}:{}/v1/add_db_with_content",
                self.path, self.port
            ))
            .query(
                "overwrite_existing",
                if overwrite_existing { "true" } else { "false" },
            )
            .query("db_name", name)
            .send_bytes(&store)?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn add_entry_to_db(
        &self,
        database_name: &str,
        key: &str,
        value: &Value,
    ) -> Result<bool, ClientError> {
        let value = value.ser(None)?;
        let rsp = self
            .agent
            .put(&format!("http://{}:{}/v1/add_kv", self.path, self.port))
            .query("db_name", database_name)
            .query("key", key)
            .send_bytes(&value)?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn remove_entry_from_db(&self, database_name: &str, key: &str) -> Result<(), ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/rm_kv", self.path, self.port))
            .query("db_name", database_name)
            .query("key", key)
            .call()?;
        rsp.error_for_status()?;
        Ok(())
    }

    pub fn remove_db(&self, database_name: &str) -> Result<(), ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/rm_db", self.path, self.port))
            .query("db_name", database_name)
            .call()?;
        rsp.error_for_status()?;
        Ok(())
    }
}

trait ResponseExt {
    fn error_for_status(&self) -> Result<StatusCode, ClientError>;
    fn body(self) -> Result<Vec<u8>, std::io::Error>;
}

impl ResponseExt for Response {
    fn error_for_status(&self) -> Result<StatusCode, ClientError> {
        let sc = StatusCode::try_from(self.status())?;
        if sc.is_client_error() || sc.is_server_error() {
            Err(ClientError::HttpErrorCode(sc))
        } else {
            Ok(sc)
        }
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
