//! `async_client` provides an asynchronous client for use with a `sourisd` database.

use crate::{client::ClientError, store::Store, values::Value};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Display;
use http::StatusCode;
use reqwest::{Client, Response};
use crate::client::{bool_to_string, CreationResult, DEFAULT_SOURISD_PORT};

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
    /// # async fn client (){
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
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

    ///Get the names of all the [`Store`]s present in the database that the client is connected to.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # async fn get_all_dbs_example () {
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
    ///
    /// match client.get_all_dbs().await {
    ///    Err(e) => eprintln!("Error getting all database names: {e}"),
    ///    Ok(db_names) => {
    ///        println!("Found database names:");
    ///        for name in db_names {
    ///            println!("\t- {name:?}");
    ///        }
    ///    }
    /// }
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

    ///Creates a new [`Store`] in the connected database with the given name. Returns whether a new database had to be created.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # async fn client () {
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
    /// client.create_new_db(false, "example_database").await.unwrap(); //ensures that a database named `example_database` exists.
    /// client.create_new_db(true, "empty_database").await.unwrap(); //ensures that an *empty* database called `empty_database` exists.
    /// # }
    /// ```
    /// 
    /// # Arguments
    /// 
    /// `name` is just the name of the [`Store`] to create. NB: The name needs to be valid ASCII and not equal to `meta` as that is reserved.
    /// 
    /// |[`Store`] already exists|`overwrite_existing`|Behaviour|
    /// |--|--|--|
    /// |`false`|`false` or `true`|A new blank [`Store`] will be created|
    /// |`true`|`true`|The existing [`Store`] will be cleared.|
    /// |`true`|`false`|Nothing happens to the existing [`Store`].|
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
                    bool_to_string(overwrite_existing)
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

    /// Gets a given [`Store`] by name.
    /// 
    ///```rust
    /// # use http::StatusCode;
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # use sourisdb::store::Store;
    /// # use sourisdb::values::Value;
    /// # async fn client () {
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
    /// 
    /// //existing store:
    /// let example_store = client.get_store("existing store").await.unwrap();
    /// assert!(example_store.is_some());
    /// 
    /// //non-existing store:
    /// let non_existing_store_error = client.get_store("non existing store").await.unwrap();
    /// assert!(non_existing_store_error.is_none());
    /// # }
    /// ```
    /// 
    /// # Arguments
    /// 
    /// - `db_name`: the name of the [`Store`] to get.
    ///
    /// # Errors
    /// - [`ClientError::HttpErrorCode`] if the database isn't found or another error occurs with the HTTP request.
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`crate::store::StoreSerError`] if the store cannot be deserialised from the bytes.
    pub async fn get_store(&self, db_name: &str) -> Result<Option<Store>, ClientError> {
        let rsp = self
            .client
            .get(&format!("http://{}:{}/v1/get_db", self.path, self.port))
            .query(&["db_name", db_name])
            .send()
            .await?;
        
        match rsp.status() {
            gone if gone == StatusCode::GONE => {
                Ok(None)
            },
            other_failure if !other_failure.is_success() => {
                Err(ClientError::HttpErrorCode(other_failure)) 
            }
            _success => {
                let bytes = rsp.bytes().await?;
                Ok(Some(Store::deser(bytes.as_ref())?))
            }
        }
    }

    ///Creates a new [`Store`] and inserts the contents of the provided [`Store`] into it. Returns whether we had to create a new database.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// # use sourisdb::store::Store;
    /// # use sourisdb::values::Value;
    /// 
    /// # async fn stuff () {
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
    /// let example_store = (); //fill in your own store here
    /// # let example_store = Store::new([("key".into(), Value::Character('v')), ("other_key".into(), Value::Boolean(false))]);
    /// 
    /// //replace existing store
    /// client.add_db_with_contents(true, "to be replaced", &example_store).await.unwrap();
    /// //append to existing store
    /// client.add_db_with_contents(false, "to be appended to", &example_store).await.unwrap();
    /// # }
    /// ```
    ///
    /// |[`Store`] already exists|`overwrite_existing`|Behaviour|
    /// |--|--|--|
    /// |`false`|`false` or `true`|The given [`Store`] will be added to the database.|
    /// |`true`|`true`|The existing [`Store`] inside the database will be overwritten by the provided [`Store`].|
    /// |`true`|`false`|The provided [`Store`] will be appended to the existing [`Store`] inside the database.|
    ///
    /// # Errors
    ///
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn add_db_with_contents(
        &self,
        overwrite_existing: bool,
        name: &str,
        store: &Store,
    ) -> Result<bool, ClientError> {
        let store = store.ser();

        let rsp = self
            .client
            .put(&format!(
                "http://{}:{}/v1/add_db_with_content",
                self.path, self.port
            ))
            .query(&[
                (
                    "overwrite_existing",
                    bool_to_string(overwrite_existing)
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

    ///Adds the given entry to the given database. If that database didn't exist before, the database will be created and the key added.
    ///
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError, CreationResult};
    /// # use sourisdb::store::Store;
    /// # use sourisdb::values::Value;
    /// 
    /// # async fn stuff () {
    /// //blank database
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to create client");
    /// let example_value = (); //fill in your own value here
    /// # let example_value = Value::Null(());
    /// 
    /// //returns CreationResult::UnableToFindDB as we specified not to create a new database
    /// client.add_entry_to_db("non_existent_db", false, false, "key", &example_value).await.unwrap();
    /// 
    /// //returns CreationResult::InsertedKeyIntoNewDB as we specified to create a new database, which we then inserted the key into
    /// client.add_entry_to_db("non_existent_db", true, false, "key", &example_value).await.unwrap();
    /// 
    /// 
    /// let existing_store = Store::new([("key", Value::Boolean(true))]);
    /// client.add_db_with_contents(true, "db", &existing_store).await.unwrap();
    /// 
    /// //returns CreationResult::FoundExistingKey as we found the existing key, but did not overwrite it.
    /// client.add_entry_to_db("db", false, false, "key", &example_value).await.unwrap();
    /// 
    /// //returns CreationResult::OverwroteKeyInExistingDB as we found the existing key and overwrote it.
    /// client.add_entry_to_db("db", false, true, "key", &example_value).await.unwrap();
    /// 
    /// //returns CreationResult::InsertedKeyIntoExistingDB as we found the database, but no key yet existed with that name so we added it.
    /// client.add_entry_to_db("db", false, false, "new key", &example_value).await.unwrap();
    /// # }
    /// ```
    /// # Errors
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn add_entry_to_db(
        &self,
        database_name: &str,
        create_new_database_if_needed: bool,
        overwrite_existing_key: bool,
        key: &str,
        value: &Value,
    ) -> Result<CreationResult, ClientError> {
        let value = value.ser(None);
        let rsp = self
            .client
            .put(&format!("http://{}:{}/v1/add_kv", self.path, self.port))
            .query(&[("db_name", database_name), ("key", key), ("create_new_database", bool_to_string(create_new_database_if_needed)), ("overwrite_key", bool_to_string(overwrite_existing_key))])
            .body(value)
            .send()
            .await?;
        
        if !rsp.status().is_success() {
            return Err(ClientError::HttpErrorCode(rsp.status()));
        }
        
        Ok(rsp.json().await?)
    }

    ///Removes the entry with the given key from the database. 
    /// 
    /// NB: This function just ensures that a given key doesn't exist - if the key or database are not found, this function will not error.
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// 
    /// # async fn rm_entry (){
    /// let client = AsyncClient::new("sub.domain.tld", None).await?;
    /// 
    /// client.remove_entry_from_db("existing store", "existing key").await.unwrap();
    /// client.remove_entry_from_db("existing store", "non existent key").await.unwrap();
    /// client.remove_entry_from_db("non existent store", "non existent key").await.unwrap();
    /// 
    /// # }
    /// ```
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

    ///Removes a given database. Returns whether we actually removed anything
    /// 
    ///```rust
    /// # use sourisdb::client::{AsyncClient, ClientError};
    /// 
    /// # async fn rm_db () {
    /// let client = AsyncClient::new("sub.domain.tld", None).await.expect("unable to remove client");
    /// 
    /// let non_existent = client.remove_db("non existent store").await.unwrap();
    /// assert!(!non_existent);
    /// 
    /// let existed = client.remove_db("existing store").await.unwrap();
    /// assert!(existed);
    /// # }
    /// ```
    ///
    /// # Errors
    /// - [`reqwest::Error`] if a reqwest error occurs or the bytes cannot be obtained.
    /// - [`ClientError::HttpErrorCode`] if an HTTP Error status code is encountered.
    pub async fn remove_db(&self, database_name: &str) -> Result<bool, ClientError> {
        let rsp = self.client
            .post(&format!("http://{}:{}/v1/rm_db", self.path, self.port))
            .query(&[("db_name", database_name)])
            .send()
            .await?;
        
        match rsp.status() {
            gone if gone == StatusCode::GONE => {
                Ok(false)
            },
            ok if ok.is_success() => {
                Ok(true)
            },
            other => {
                Err(ClientError::HttpErrorCode(other))
            }
        }
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
