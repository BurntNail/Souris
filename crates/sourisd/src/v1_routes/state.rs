use axum::http::StatusCode;
use color_eyre::eyre::{bail, Context};
use dirs::data_dir;
use sourisdb::{store::Store, values::Value};
use std::{
    collections::HashMap,
    env::var,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    fs::{create_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, ErrorKind},
    sync::Mutex,
};

fn running_with_superuser() -> bool {
    unsafe { libc::geteuid() == 0 }
}

mod meta {
    ///File name for the database that stores the meta information
    pub const META_DB_FILE_NAME: &str = "meta.sdb";
    ///Name of the key inside the meta information database that stores the array of databases
    pub const DB_FILE_NAMES_KEY: &str = "existing_dbs";
}
use crate::{error::SourisError, v1_routes::value::KeyAndDb};
use meta::{DB_FILE_NAMES_KEY, META_DB_FILE_NAME};

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SourisState {
    ///The base location in which all databases reside
    base_location: PathBuf,
    ///A map of all databases and their names
    dbs: Arc<Mutex<HashMap<String, Store>>>,
}

impl SourisState {
    ///Create a new database.
    ///
    /// Returns [`StatusCode::OK`] if an existing database was overwritten, or [`StatusCode::CREATED`] if a new database was created.
    ///
    /// ## Errors
    /// - [`SourisError::InvalidDatabaseName`] if the name is not ASCII or the name is `meta`.
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn new_db(
        &self,
        name: String,
        overwrite_existing: bool,
    ) -> Result<StatusCode, SourisError> {
        if name == "meta" || !name.is_ascii() {
            return Err(SourisError::InvalidDatabaseName);
        }

        let mut dbs = self.dbs.lock().await;

        if dbs.contains_key(&name) {
            trace!(
                ?name,
                "Tried to add new store, found existing store with name."
            );

            if overwrite_existing {
                let Some(db) = dbs.get_mut(&name) else {
                    unreachable!("just checked that the key exists")
                };
                db.clear();
            }
            return Ok(StatusCode::OK);
        }

        dbs.insert(name.clone(), Store::default());

        Ok(StatusCode::CREATED)
    }

    #[tracing::instrument(level = "trace", skip(self, contents))]
    pub async fn new_db_with_contents(
        &self,
        name: String,
        overwrite_existing: bool,
        contents: Store,
    ) -> StatusCode {
        let mut stores = self.dbs.lock().await;

        let mut contained = false;
        if stores.contains_key(&name) {
            contained = true;
            if !overwrite_existing {
                return StatusCode::OK;
            }
        }

        stores.insert(name, contents);

        if contained {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        }
    }

    ///returns whether it cleared a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn clear_db(&self, name: String) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;
        if let Some(store) = dbs.get_mut(&name) {
            store.clear();
            Ok(())
        } else {
            trace!("Unable to find store.");
            Err(SourisError::DatabaseNotFound)
        }
    }

    ///returns whether it removed a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn remove_db(&self, name: String) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;
        if !dbs.contains_key(&name) {
            return Err(SourisError::DatabaseNotFound);
        }

        dbs.remove(&name);

        let file_name = self.base_location.join(format!("{name}.sdb"));

        if let Err(e) = tokio::fs::remove_file(file_name).await {
            if e.kind() != ErrorKind::NotFound {
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn get_db(&self, name: String) -> Result<Store, SourisError> {
        let dbs = self.dbs.lock().await;
        dbs.get(&name).cloned().ok_or(SourisError::DatabaseNotFound)
    }

    pub async fn add_key_value_pair(
        &self,
        KeyAndDb { key, db_name }: KeyAndDb,
        v: Value,
    ) -> StatusCode {
        let mut dbs = self.dbs.lock().await;

        let db = if let Some(d) = dbs.get_mut(&db_name) {
            d
        } else {
            dbs.insert(db_name.clone(), Store::default());
            dbs.get_mut(&db_name)
                .expect("just added this database key lol")
        };

        match db.insert(key, v) {
            Some(_) => StatusCode::OK,
            None => StatusCode::CREATED,
        }
    }

    pub async fn get_value(
        &self,
        KeyAndDb { key, db_name }: KeyAndDb,
    ) -> Result<Value, SourisError> {
        let dbs = self.dbs.lock().await;

        let Some(db) = dbs.get(&db_name) else {
            return Err(SourisError::DatabaseNotFound);
        };
        let Some(key) = db.get(&key).cloned() else {
            return Err(SourisError::KeyNotFound);
        };

        Ok(key)
    }

    pub async fn remove_key(&self, KeyAndDb { key, db_name }: KeyAndDb) -> Result<(), SourisError> {
        let mut dbs = self.dbs.lock().await;

        let Some(db) = dbs.get_mut(&db_name) else {
            return Err(SourisError::DatabaseNotFound);
        };

        match db.remove(&key) {
            Some(_) => Ok(()),
            None => Err(SourisError::KeyNotFound),
        }
    }

    pub async fn get_all_db_names(&self) -> Vec<String> {
        self.dbs.lock().await.keys().cloned().collect()
    }
}

impl SourisState {
    pub async fn new() -> color_eyre::Result<Self> {
        #[tracing::instrument(level = "trace")]
        async fn get_store(location: PathBuf) -> color_eyre::Result<Store> {
            let mut file = match File::open(&location).await {
                Ok(f) => f,
                Err(e) => {
                    return if e.kind() == ErrorKind::NotFound {
                        trace!(?location, "File not found, getting empty store.");
                        return Ok(Store::default());
                    } else {
                        Err(e.into())
                    };
                }
            };

            let mut contents = vec![];
            let mut tmp = [0_u8; 128];

            loop {
                match file.read(&mut tmp).await? {
                    0 => break,
                    n => {
                        contents.extend(&tmp[0..n]);
                    }
                }
            }

            Ok(Store::deser(&contents)?)
        }

        #[tracing::instrument(level = "trace", skip(meta))]
        async fn get_internal_stores(
            meta: &Store,
            base: PathBuf,
        ) -> Option<HashMap<String, Store>> {
            let Some(Value::Array(values)) = meta.get(DB_FILE_NAMES_KEY) else {
                trace!("Unable to find existing databases.");
                return None;
            };

            let mut dbs = HashMap::new();

            for val in values {
                let Some(file_name) = val.as_str() else {
                    trace!(?val, "Found non-string inside existing databases list");
                    continue;
                };

                match get_store(base.join(format!("{file_name}.sdb"))).await {
                    Ok(s) => {
                        dbs.insert(file_name.to_string(), s);
                    }
                    Err(e) => {
                        trace!(?e, ?file_name, "Error getting database");
                        continue;
                    }
                }
            }

            Some(dbs)
        }

        let base_location = if let Ok(loc) = var("BASE_LOCATION") {
            let path = PathBuf::from(loc);
            std::fs::create_dir_all(&path).context("trying to create custom base location")?;
            path
        } else if running_with_superuser() {
            PathBuf::from("/etc/souris/")
        } else {
            let Some(base_location) = data_dir() else {
                bail!("Unable to find non-superuser data directory");
            };
            base_location.join("souris/")
        };

        let mut meta = get_store(base_location.join(META_DB_FILE_NAME)).await?;

        let dbs = if let Some(dbs) = get_internal_stores(&meta, base_location.clone()).await {
            dbs
        } else {
            meta.insert(DB_FILE_NAMES_KEY.into(), Value::Array(vec![]));
            HashMap::default()
        };

        let s = Self {
            base_location,
            dbs: Arc::new(Mutex::new(dbs)),
        };

        Ok(s)
    }

    pub async fn save(&self) -> color_eyre::Result<()> {
        let mut names = vec![];

        for (name, db) in self.dbs.lock().await.iter() {
            let file_name = self.base_location.join(format!("{name}.sdb"));
            let bytes = db.ser()?;

            if let Err(e) = write_to_file(&bytes, file_name, &self.base_location).await {
                error!(?e, "Error writing out database");
            } else {
                names.push(Value::String(name.to_string()));
            }
        }

        let mut meta = Store::default();
        meta.insert(DB_FILE_NAMES_KEY.into(), Value::Array(names));

        let location = self.base_location.join(META_DB_FILE_NAME);
        let meta = meta.ser()?;
        write_to_file(&meta, location, &self.base_location).await
    }
}

async fn write_to_file(
    bytes: &[u8],
    path: impl AsRef<Path> + Debug,
    base_location: impl AsRef<Path> + Debug,
) -> color_eyre::Result<()> {
    let mut file = match File::create(&path).await {
        Ok(f) => f,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                //folder must not exist, hopefully?
                trace!(?path, ?base_location, "Unable to find folder, creating");

                create_dir_all(&base_location).await?;
                File::create(path).await?
            } else {
                return Err(e.into());
            }
        }
    };

    file.write_all(bytes).await?;

    Ok(())
}
