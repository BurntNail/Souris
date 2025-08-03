use axum::{body::Bytes, http::StatusCode};
use color_eyre::eyre::{bail, Context};
use dirs::data_dir;
use moka::future::Cache;
use sourisdb::{store::Store, values::Value};
use std::{
    collections::{HashMap, hash_map::Entry as SEntry},
    env::var,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};
use sourisdb::hashbrown::hash_map::Entry as HBEntry;
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
use sourisdb::client::CreationResult;
use crate::v1_routes::value::NewKeyArgs;

#[derive(Clone, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SourisState {
    ///The base location in which all databases reside
    base_location: PathBuf,
    ///A map of all databases and their names
    dbs: Arc<Mutex<HashMap<String, Store>>>,
    db_cache: Cache<String, Bytes>,
}

impl SourisState {
    ///Create a new database.
    ///
    /// Returns [`StatusCode::OK`] if an existing database was present, or [`StatusCode::CREATED`] if a new database was created.
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
        
        let sc = match dbs.entry(name.clone()) {
            SEntry::Occupied(mut occ) => {
                if overwrite_existing {
                    occ.get_mut().clear();
                }
                StatusCode::OK
            }
            SEntry::Vacant(vac) => {
                vac.insert(Store::default());
                StatusCode::CREATED
            }
        };
        
        self.db_cache.invalidate(&name).await;

        Ok(sc)
    }

    #[tracing::instrument(level = "trace", skip(self, contents))]
    pub async fn new_db_with_contents(
        &self,
        name: String,
        overwrite_existing: bool,
        contents: Store,
    ) -> StatusCode {
        self.db_cache.invalidate(&name).await;
        let mut dbs = self.dbs.lock().await;

        let created_new = dbs.contains_key(&name);
        let current = dbs.entry(name).or_default();
        if overwrite_existing {
            *current = contents;
        } else {
            for (k, v) in &*contents {
                current.insert(k.clone(), v.clone());
            }
        }

        if created_new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        }
    }

    ///returns whether it cleared a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn clear_db(&self, name: String) -> Result<(), SourisError> {
        self.db_cache.invalidate(&name).await;

        let mut dbs = self.dbs.lock().await;

        if let SEntry::Occupied(mut e) = dbs.entry(name) {
            e.insert(Store::default());
            Ok(())
        } else {
            trace!("Unable to find store.");
            Err(SourisError::DatabaseNotFound)
        }
    }

    ///returns whether it removed a database
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn remove_db(&self, name: String) -> Result<(), SourisError> {
        self.db_cache.invalidate(&name).await;

        let mut dbs = self.dbs.lock().await;

        if !dbs.contains_key(&name) {
            return Err(SourisError::DatabaseNotFound);
        }

        dbs.remove(&name);
        drop(dbs);

        let file_name = self.base_location.join(format!("{name}.sdb"));

        if let Err(e) = tokio::fs::remove_file(file_name).await {
            if e.kind() != ErrorKind::NotFound {
                return Err(e.into());
            }
        }

        Ok(())
    }

    pub async fn get_db(&self, name: String) -> Result<Bytes, SourisError> {
        if let Some(bytes) = self.db_cache.get(&name).await {
            return Ok(bytes);
        }

        let dbs = self.dbs.lock().await;
        let db = dbs
            .get(&name)
            .cloned()
            .ok_or(SourisError::DatabaseNotFound)?;

        let sered = db.ser();
        let bytes = Bytes::from(sered);

        self.db_cache.insert(name, bytes.clone()).await;
        Ok(bytes)
    }

    pub async fn add_key_value_pair(
        &self,
        NewKeyArgs { db_name, key, create_new_database, overwrite_key }: NewKeyArgs,
        v: Value,
    ) -> CreationResult {
        self.db_cache.invalidate(&db_name).await;

        let mut dbs = self.dbs.lock().await;
        match dbs.entry(db_name) {
            SEntry::Occupied(mut occ) => {
                let db = occ.get_mut();
                match db.entry(key) {
                    HBEntry::Occupied(mut occ) => {
                        if overwrite_key {
                            occ.insert(v);
                            CreationResult::OverwroteKeyInExistingDB
                        } else {
                            CreationResult::FoundExistingKey
                        }
                    }
                    HBEntry::Vacant(vac) => {
                        vac.insert(v);
                        CreationResult::InsertedKeyIntoExistingDB
                    }
                }
            }
            SEntry::Vacant(vac) => {
                if create_new_database {
                    let db = vac.insert(Store::default());
                    db.insert(key, v);
                    CreationResult::InsertedKeyIntoNewDB
                } else {
                    CreationResult::UnableToFindDB
                }
            }
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
        self.db_cache.invalidate(&db_name).await;
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
            db_cache: Cache::new(200),
        };

        Ok(s)
    }

    pub async fn save(&self) -> color_eyre::Result<()> {
        let mut names = vec![];

        for (name, db) in self.dbs.lock().await.iter() {
            let file_name = self.base_location.join(format!("{name}.sdb"));
            let bytes = db.ser();

            if let Err(e) = write_to_file(&bytes, file_name, &self.base_location).await {
                error!(?e, "Error writing out database");
            } else {
                names.push(Value::String(name.to_string()));
            }
        }

        let mut meta = Store::default();
        meta.insert(DB_FILE_NAMES_KEY.into(), Value::Array(names));

        let location = self.base_location.join(META_DB_FILE_NAME);
        let meta = meta.ser();
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
