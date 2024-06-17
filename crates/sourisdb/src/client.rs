use crate::{
    store::{Store, StoreSerError},
    values::{Value, ValueSerError, ValueTy},
};
use core::fmt::{Display, Formatter};
use http::{status::InvalidStatusCode, StatusCode};
use std::{io::Read, sync::Arc};
use ureq::{Agent, Response};

#[derive(Debug, Clone)]
pub struct SourisClient {
    path: Arc<str>, //path is never changed, so just use arc<str> for cloning benefits
    port: u32,
    agent: Agent, //also internally arc-ed, so easy to clone
}

#[derive(Debug)]
pub enum ClientError {
    Ureq(Box<ureq::Error>),
    Store(StoreSerError),
    Value(ValueSerError),
    HttpErrorCode(StatusCode),
    IO(std::io::Error),
    InvalidStatusCode(InvalidStatusCode),
    ExpectedKey(&'static str),
    IncorrectType { ex: ValueTy, found: ValueTy },
    ServerNotHealthy(StatusCode),
    SerdeJson(serde_json::Error),
}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ureq(u) => write!(f, "Error with ureq: {u}"),
            Self::Store(s) => write!(f, "Error with store: {s}"),
            Self::HttpErrorCode(sc) => write!(f, "Error with response: {sc:?}"),
            Self::IO(e) => write!(f, "IO Error: {e}"),
            Self::InvalidStatusCode(e) => write!(f, "Invalid status code provided: {e}"),
            Self::ExpectedKey(k) => write!(f, "Expected to find key: {k:?} in `Store` of body"),
            Self::IncorrectType { ex, found } => {
                write!(f, "Expected to find value of type {ex:?}, found {found:?}")
            }
            Self::ServerNotHealthy(sc) => write!(
                f,
                "Tried to get server health check, got status code: {sc:?}"
            ),
            Self::SerdeJson(e) => write!(f, "Tried to parse JSON and failed: {e}"),
            Self::Value(e) => write!(f, "Error with value: {e}"),
        }
    }
}

impl From<ureq::Error> for ClientError {
    fn from(value: ureq::Error) -> Self {
        Self::Ureq(Box::new(value))
    }
}
impl From<StoreSerError> for ClientError {
    fn from(value: StoreSerError) -> Self {
        Self::Store(value)
    }
}
impl From<std::io::Error> for ClientError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}
impl From<InvalidStatusCode> for ClientError {
    fn from(value: InvalidStatusCode) -> Self {
        Self::InvalidStatusCode(value)
    }
}
impl From<serde_json::Error> for ClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<ValueSerError> for ClientError {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Ureq(u) => Some(u),
            Self::Store(s) => Some(s),
            Self::IO(e) => Some(e),
            Self::InvalidStatusCode(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Value(e) => Some(e),
            _ => None,
        }
    }
}

impl SourisClient {
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
                "http://{}:{}/v1/get_all_dbs",
                self.path, self.port
            ))
            .call()?;
        rsp.error_for_status()?;

        let body = rsp.body()?;
        Ok(serde_json::from_slice(&body)?)
    }

    pub fn create_new_db(
        &self,
        overwrite_existing: bool,
        name: String,
    ) -> Result<bool, ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/add_db", self.path, self.port))
            .query(
                "overwrite_existing",
                if overwrite_existing { "true" } else { "false" },
            )
            .query("name", name.as_str())
            .call()?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn get_store(&self, db_name: String) -> Result<Store, ClientError> {
        let rsp = self
            .agent
            .get(&format!("http://{}:{}/v1/get_db", self.path, self.port))
            .query("name", db_name.as_str())
            .call()?;
        rsp.error_for_status()?;
        let body = rsp.body()?;
        Ok(Store::deser(&body)?)
    }

    pub fn add_db_with_contents(
        &self,
        overwrite_existing: bool,
        name: String,
        store: Store,
    ) -> Result<bool, ClientError> {
        let store = store.ser()?;
        let rsp = self
            .agent
            .get(&format!(
                "http://{}:{}/v1/add_db_with_content",
                self.path, self.port
            ))
            .query(
                "overwrite_existing",
                if overwrite_existing { "true" } else { "false" },
            )
            .query("name", name.as_str())
            .send_bytes(&store)?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn add_entry_to_db(
        &self,
        database_name: String,
        key: String,
        value: Value,
    ) -> Result<bool, ClientError> {
        let value = value.ser()?;
        let rsp = self
            .agent
            .put(&format!("http://{}:{}/v1/add_kv", self.path, self.port))
            .query("db", database_name.as_str())
            .query("key", key.as_str())
            .send_bytes(&value)?;
        rsp.error_for_status()?;
        Ok(match rsp.error_for_status()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    pub fn remove_entry_from_db(
        &self,
        database_name: String,
        key: String,
    ) -> Result<(), ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/rm_key", self.path, self.port))
            .query("db", database_name.as_str())
            .query("key", key.as_str())
            .call()?;
        rsp.error_for_status()?;
        Ok(())
    }

    pub fn remove_db(&self, database_name: String) -> Result<(), ClientError> {
        let rsp = self
            .agent
            .post(&format!("http://{}:{}/v1/rm_db", self.path, self.port))
            .query("name", database_name.as_str())
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
