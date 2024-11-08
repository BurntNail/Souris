//! `async_client` provides an asynchronous client for use with a `sourisd` client.

use crate::{client::ClientError, store::Store, values::Value};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Display;
use http::StatusCode;
use reqwest::{Client, Response};
use crate::client::DEFAULT_SOURISD_PORT;

///A client for interacting with `sourisd` asynchronously.
/// 
/// Construct a new one using [`AsyncClient::new`]
#[derive(Debug, Clone)]
pub struct AsyncClient {
    path: String,
    port: u32,
    client: Client, //TODO: option to change the protocol
}

impl AsyncClient {
    ///Create a new asynchronous client using the provided path and port, and then confirms whether that database is alive.
    /// 
    /// NB: This is an async and fallible method because if the provided path/port doesn't respond correctly to a healthcheck, then the method will not return a client.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # async fn client () -> Result<(), ClientError> {
    /// let client = AsyncClient::new("host.domain.tld", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// # Arguments
    /// - The path should either be an IP address or a DNS part, like `127.0.0.1`, `localhost` or `google.com`.
    /// - The port is optional - if [`None`] is provided, then the default `sourisd` port will be used (7687).
    /// 
    /// # Errors
    /// - [`reqwest::Error`] if there is a non-status related error with [`reqwest`].
    /// - [`ClientError::ServerNotHealthy`] if we don't get back a [`StatusCode::OK`] from the server when doing the healthcheck.
    pub async fn new(path: impl Display, port: Option<u32>) -> Result<Self, ClientError> {
        let path = path.to_string();
        let client = Client::new();
        let port = port.unwrap_or(DEFAULT_SOURISD_PORT);

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

    ///Get the names of all the databases present in the instance that the client is connected to.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # async fn get_all_dbs_example () -> Result<(), ClientError> {
    /// let client = AsyncClient::new("host.domain.tld", None).await?;
    /// println!("Getting databases");
    /// match client.get_all_dbs().await {
    ///    Err(e) => eprintln!("Error getting databases: {e}"),
    ///    Ok(db_names) => {
    ///        println!("Found database names:");
    ///        for name in db_names {
    ///            println!("\t- {name:?}");
    ///        }
    ///    }
    /// }
    /// # Ok(())
    /// # }
    /// ```
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

    ///Creates a new database in the connected instance with the given name. Returns whether a new database had to be created.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # async fn client () -> Result<(), ClientError> {
    /// let client = AsyncClient::new("host.domain.tld", None).await?;
    /// client.create_new_db(false, "example_database").await?; //ensures that a database named `example_database` exists.
    /// client.create_new_db(true, "empty_database").await?; //ensures that an *empty* database called `empty_database` exists.
    /// # Ok(())
    /// # }
    /// ```
    /// 
    /// # Arguments
    /// 
    /// `name` is just the name of the database to create. NB: The name needs to be valid ASCII and not equal to `meta` as that is reserved.
    /// 
    /// |Database already exists|`overwrite_existing`|Behaviour|
    /// |--|--|--|
    /// |`false`|`false` or `true`|A new blank database will be created|
    /// |`true`|`true`|The existing database will be cleared.|
    /// |`true`|`false`|Nothing happens to the existing database.|
    ///
    /// # Errors
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
