//! `async_client` provides an asynchronous client for use with a `sourisd` client.
//!
//! When you create a new client using [`AsyncClient::new`], it polls the database's healthcheck endpoint to confirm that the database is running.
//!
//! ```rust
//! use sourisdb::client::{AsyncClient, ClientError};
//!
//! async fn get_all_database_names_from_localhost () -> Result<Vec<String>, ClientError> {
//!     let client = AsyncClient::new("localhost", 7687).await?;
//!     client.get_all_dbs().await
//! }
//! ```

use crate::{client::ClientError, store::Store, values::Value};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Display;
use http::StatusCode;
use reqwest::{Client, Response};

///A client for interacting with `sourisd` asynchronously.
#[derive(Debug, Clone)]
pub struct AsyncClient {
    path: String,
    port: u32,
    client: Client, //TODO: option to change the protocol
}

impl AsyncClient {
    ///Create a new asynchronous client using the provided path and port.
    ///
    /// ## Errors
    /// - [`reqwest::Error`] if there is a non-status related error with Reqwest
    /// - [`ClientError::ServerNotHealthy`] if we don't get back a [`StatusCode::OK`] from the server.
    pub async fn new(path: impl Display, port: u32) -> Result<Self, ClientError> {
        let path = path.to_string();
        let client = Client::new();

        match client
            .get(&format!("http://{path}:{port}/healthcheck"))
            .send()
            .await
        {
            Ok(rsp) => {
                if rsp.status() != StatusCode::OK {
                    return Err(ClientError::ServerNotHealthy(rsp.status()));
                }
            }
            Err(e) => {
                if let Some(status) = e.status() {
                    if status != StatusCode::OK {
                        return Err(ClientError::ServerNotHealthy(status));
                    }
                } else {
                    return Err(ClientError::Reqwest(e));
                }
            }
        };

        Ok(Self { path, port, client })
    }

    ///Get the names of all the databases present in the instance.
    ///
    /// ## Errors
    /// - [`reqwest::Error`] if there is an error with the HTTP request, or we cannot get the raw bytes out
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn get_all_dbs(&self) -> Result<Vec<String>, ClientError> {
        Ok(self
            .client
            .get(&format!(
                "http://{}:{}/v1/get_all_db_names",
                self.path, self.port
            ))
            .send()
            .await?
            .json()
            .await?)
    }

    ///Creates a new database in the connected instance with the given name.
    ///
    /// ## `overwrite_existing`
    ///
    /// If the database already exists, it will be cleared and `Ok(true)` will always be returned in the happy path.
    ///
    /// ## !`overwrite_existing`
    ///
    /// If the database already exists, it will be left as is and `Ok(false)` will be returned in the happy path.
    ///
    /// If it doesn't, then it will be created and `Ok(true)` will be returned.
    ///
    /// ## Errors
    /// - [`reqwest::Error`] if there is an error with the HTTP request.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn create_new_db(
        &self,
        overwrite_existing: bool,
        name: &str,
    ) -> Result<bool, ClientError> {
        let rsp = self
            .client
            .post(&format!("http://{}:{}/v1/add_db", self.path, self.port))
            .query(&[
                (
                    "overwrite_existing",
                    if overwrite_existing { "true" } else { "false" },
                ),
                ("db_name", name),
            ])
            .send()
            .await?;
        Ok(match rsp.error_for_status_to_client_error()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    /// Gets a given store by name. If the store doesn't exist, [`ClientError::HttpErrorCode`] will be returned with a code of [`StatusCode::NOT_FOUND`].
    ///
    /// ## Errors
    /// - `[ClientError::HttpErrorCode`] if the database isn't found or another error occurs with the HTTP request.
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`crate::store::StoreSerError`] if the store cannot be deserialised from the bytes.
    pub async fn get_store(&self, db_name: &str) -> Result<Store, ClientError> {
        let rsp = self
            .client
            .get(&format!("http://{}:{}/v1/get_db", self.path, self.port))
            .query(&["db_name", db_name])
            .send()
            .await?;
        rsp.error_for_status_to_client_error()?;
        let bytes = rsp.bytes().await?;
        Ok(Store::deser(bytes.as_ref())?)
    }

    ///Adds a new database and immediately inserts the contents of the [`Store`] into it.
    ///
    /// If `overwrite_existing` is true or the store already exists, the server will now have one instance of the provided store with the provided contents.
    ///
    /// If the database already existed, and `overwrite_existing` is false, then the server will append the keys from the provided database into the new one.
    ///
    /// # Errors
    ///
    /// - [`crate::store::StoreSerError`] if we cannot serialise the provided `Store`.
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn add_db_with_contents(
        &self,
        overwrite_existing: bool,
        name: &str,
        store: &Store,
    ) -> Result<bool, ClientError> {
        let store = store.ser()?;

        let rsp = self
            .client
            .put(&format!(
                "http://{}:{}/v1/add_db_with_content",
                self.path, self.port
            ))
            .query(&[
                (
                    "overwrite_existing",
                    if overwrite_existing { "true" } else { "false" },
                ),
                ("db_name", name),
            ])
            .body(store)
            .send()
            .await?;

        Ok(match rsp.error_for_status_to_client_error()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    ///Adds the given entry to the given database. If that database didn't exist before, it will now.
    ///
    /// # Errors
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn add_entry_to_db(
        &self,
        database_name: &str,
        key: &str,
        value: &Value,
    ) -> Result<bool, ClientError> {
        let value = value.ser(None);
        let rsp = self
            .client
            .put(&format!("http://{}:{}/v1/add_kv", self.path, self.port))
            .query(&[("db_name", database_name), ("key", key)])
            .body(value)
            .send()
            .await?;

        Ok(match rsp.error_for_status_to_client_error()? {
            StatusCode::OK => false,
            StatusCode::CREATED => true,
            _ => unreachable!("API cannot return anything but ok or created"),
        })
    }

    ///Removes the entry with the given key from the database.
    ///
    /// # Errors
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn remove_entry_from_db(
        &self,
        database_name: &str,
        key: &str,
    ) -> Result<(), ClientError> {
        self.client
            .post(&format!("http://{}:{}/v1/rm_kv", self.path, self.port))
            .query(&[("db_name", database_name), ("key", key)])
            .send()
            .await?
            .error_for_status_to_client_error()?;
        Ok(())
    }

    ///Removes a given database.
    ///
    /// NB: A 404 code is returned by the daemon if the database cannot be found, which will show up as [`ClientError::HttpErrorCode`].
    ///
    /// # Errors
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn remove_db(&self, database_name: &str) -> Result<(), ClientError> {
        self.client
            .post(&format!("http://{}:{}/v1/rm_db", self.path, self.port))
            .query(&[("db_name", database_name)])
            .send()
            .await?
            .error_for_status_to_client_error()?;
        Ok(())
    }
}

trait ResponseExt {
    fn error_for_status_to_client_error(&self) -> Result<StatusCode, ClientError>;
}

impl ResponseExt for Response {
    fn error_for_status_to_client_error(&self) -> Result<StatusCode, ClientError> {
        let status = self.status();
        if status.is_success() {
            Ok(status)
        } else {
            Err(ClientError::HttpErrorCode(status))
        }
    }
}
