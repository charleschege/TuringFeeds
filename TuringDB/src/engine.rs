use anyhow::Result;
use custom_codes::DbOps;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
};
use tai64::TAI64N;
use async_fs::{self, DirBuilder};
use futures_lite::stream::StreamExt;
use async_lock::Mutex;

const REPO_NAME: &str = "TuringDB_Repo";
// TODO use custom_codes errors to give actual errors
// TODO Check whether you can respond with sled::Error

/// This engine handles data all database queries and in-memory keys and sled file locks
/// #### Structure
/// ```
/// #[derive(Debug, Clone)]
/// pub struct TuringEngine {
///     dbs: DashMap<OsString, Tdb>, // Repo<DatabaseName, Databases>
/// }
/// ```
#[derive(Debug, Default)]
pub struct TuringEngine {
    dbs: DashMap<OsString, Tdb>, // Repo<DatabaseName, Databases>
}

impl TuringEngine {
    /// Create a new in-memory repo
    pub fn new() -> TuringEngine {
        Self {
            dbs: DashMap::new(),
        }
    }
    /// Create a repo
    pub async fn repo_create(&self) -> Result<DbOps> {
        let path = "TuringDB_Repo";
        DirBuilder::new().recursive(false).create(path).await?;

        Ok(DbOps::RepoCreated)
    }
    /// Check if the repository is empty
    pub async fn is_empty(&self) -> bool {
        self.dbs.is_empty()
    }
    //TODO
    // 1. READ THE REPO AND CHECK AGANIST A HMAC FOR TIME AND HASHES
    // 5. APPLY TIMESTAMP AND DATABASE OPS TO ops.log file
    //---------
    /// Read a repo
    pub async fn repo_init(&self) -> Result<&TuringEngine> {
        let mut repo = match async_fs::read_dir("TuringDB_Repo").await {
            Ok(value) => value,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(self);
                } else {
                    return Err(anyhow::Error::new(e));
                }
            }
        };

        while let Some(database_entry) = repo.try_next().await? {
            let database_name = database_entry.file_name();

            if database_entry.file_type().await?.is_dir() {
                let mut repo = async_fs::read_dir(&database_entry.path()).await?;
                let mut current_db = Tdb::new();

                while let Some(document_entry) = repo.try_next().await? {
                    let mut field_keys = Vec::new();

                    if document_entry.file_type().await?.is_dir() {
                        let document_name = document_entry.file_name();
                        let db = sled::open(document_entry.path())?;

                        for field_key in db.into_iter().keys() {
                            field_keys.push(field_key?);
                        }

                        let data = field_keys.iter().map(|inner| inner.to_vec()).collect();

                        current_db.list.insert(
                            document_name,
                            Document {
                                fd: Mutex::new(db),
                                keys: data,
                            },
                        );
                    }
                }
                self.dbs.insert(database_name, current_db);
            }
        }

        Ok(self)
    }
    /// Drop a repository
    pub async fn repo_drop(&self) -> Result<DbOps> {
        async_fs::remove_dir_all(REPO_NAME).await?;
        Ok(DbOps::RepoDropped)
    }

    /************** DATABASES *******************/
    /// Create a database
    pub async fn db_create(&self, db_name: &Path) -> Result<DbOps> {
        let mut path: PathBuf = REPO_NAME.into();
        path.push(db_name);

        DirBuilder::new().recursive(false).create(path).await?;

        self.dbs.insert(db_name.into(), Tdb::new());

        Ok(DbOps::DbCreated)
    }
    /// Drop the database
    pub async fn db_drop(&self, db_name: &Path) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        let mut path: PathBuf = REPO_NAME.into();
        path.push(db_name);
        async_fs::remove_dir_all(path).await?;

        self.dbs.remove(&OsString::from(db_name));

        Ok(DbOps::DbDropped)
    }
    /// List all the databases in the repo
    pub async fn db_list(&self) -> DbOps {
        if self.dbs.is_empty() {
            return DbOps::RepoEmpty;
        }

        let list = self
            .dbs
            .iter()
            .map(|db| db.key().clone().to_string_lossy().to_string())
            .collect::<Vec<String>>();

        if list.is_empty() {
            DbOps::RepoEmpty
        } else {
            DbOps::DbList(list)
        }
    }

    /************** DOCUMENTS ************/
    /// Create a document
    pub async fn doc_create(&self, db_name: &Path, doc_name: &Path) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if let Some(path) = doc_name.to_str() {
            if path.is_empty() {
                return Ok(DbOps::EncounteredErrors(
                    "[TuringDB::<DocumentCreate>::(ERROR)-DOCUMENT_NAME_EMPTY]".to_owned(),
                ));
            }
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            let mut path: PathBuf = REPO_NAME.into();
            path.push(db_name);
            path.push(doc_name);

            if database.list.get_mut(&OsString::from(doc_name)).is_some() {
                Ok(DbOps::DocumentAlreadyExists)
            } else {
                database.value_mut().list.insert(
                    OsString::from(doc_name),
                    Document {
                        fd: Mutex::new(sled::Config::default().create_new(true).path(path).open()?),
                        keys: Vec::new(),
                    },
                );

                Ok(DbOps::DocumentCreated)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /// Drop a document
    pub async fn doc_drop(&self, db_name: &Path, doc_name: &Path) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            match database.value_mut().list.remove(&OsString::from(doc_name)) {
                Some(_) => {
                    let mut path: PathBuf = REPO_NAME.into();
                    path.push(db_name);
                    path.push(doc_name);
                    async_fs::remove_dir_all(path).await?;

                    Ok(DbOps::DocumentDropped)
                }
                None => Ok(DbOps::DocumentNotFound),
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /// List all fields in a document
    pub async fn doc_list(&self, db_name: &Path) -> DbOps {
        if self.dbs.is_empty() {
            return DbOps::RepoEmpty;
        }

        if let Some(database) = self.dbs.get(&OsString::from(db_name)) {
            let list = database
                .list
                .keys()
                .map(|document| document.to_string_lossy().to_string())
                .collect::<Vec<String>>();

            if list.is_empty() {
                DbOps::DbEmpty
            } else {
                DbOps::DocumentList(list)
            }
        } else {
            DbOps::DbNotFound
        }
    }
    /// Flush all dirty I/O buffers from pagecache to disk.
    /// `RECOMMENDED:` Always use this function whenever you are building a networked server
    pub async fn flush(&self, db_name: &Path, doc_name: &Path) -> Result<DbOps> {
        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                document.fd.lock().await.flush()?;
                Ok(DbOps::Commited)
            } else {
                Ok(DbOps::DocumentNotFound)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /************* FIELDS ************/
    /// List all fields in a document
    pub async fn field_list(&self, db_name: &Path, doc_name: &Path) -> DbOps {
        if self.dbs.is_empty() {
            return DbOps::RepoEmpty;
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                if document.keys.is_empty() {
                    DbOps::DocumentEmpty
                } else {
                    let data = document.keys.iter().map(|key| key.to_vec()).collect();

                    DbOps::FieldList(data)
                }
            } else {
                DbOps::DocumentNotFound
            }
        } else {
            DbOps::DbNotFound
        }
    }
    /// Create a field with data
    pub async fn field_insert(
        &self,
        db_name: &Path,
        doc_name: &Path,
        field_name: &[u8],
        data: &[u8],
    ) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if field_name.is_empty() {
            return Ok(DbOps::EncounteredErrors(
                "[TuringDB::<FieldList>::(ERROR)-FIELD_NAME_EMPTY]".to_owned(),
            ));
        }

        if data.is_empty() {
            return Ok(DbOps::EncounteredErrors(
                "[TuringDB::<FieldList>::(ERROR)-DATA_FIELD_EMPTY]".to_owned(),
            ));
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                if document.keys.binary_search(&field_name.to_vec()).is_ok() {
                    Ok(DbOps::FieldAlreadyExists)
                } else {
                    document.fd.lock().await.insert(field_name, data)?;

                    Ok(DbOps::FieldInserted)
                }
            } else {
                Ok(DbOps::DocumentNotFound)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /// Get a field
    pub async fn field_get(
        &self,
        db_name: &Path,
        doc_name: &Path,
        field_name: &[u8],
    ) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                if document.keys.binary_search(&field_name.to_vec()).is_ok() {
                    match document.fd.lock().await.get(field_name)? {
                        Some(data) => Ok(DbOps::FieldContents(data.to_vec())),
                        None => Ok(DbOps::FieldNotFound),
                    }
                } else {
                    Ok(DbOps::FieldNotFound)
                }
            } else {
                Ok(DbOps::DocumentNotFound)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /// Drop a field
    pub async fn field_remove(
        &self,
        db_name: &Path,
        doc_name: &Path,
        field_name: &[u8],
    ) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                if let Ok(field_index) = document.keys.binary_search(&field_name.to_vec()) {
                    let sled_op = document.fd.lock().await.remove(field_name)?;

                    match sled_op {
                        Some(_) => {
                            document.keys.remove(field_index);
                            Ok(DbOps::FieldDropped)
                        }
                        None => Ok(DbOps::FieldNotFound),
                    }
                } else {
                    Ok(DbOps::FieldNotFound)
                }
            } else {
                Ok(DbOps::DocumentNotFound)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
    /// Update a field
    pub async fn field_modify(
        &self,
        db_name: &Path,
        doc_name: &Path,
        field_name: &[u8],
        field_value: &[u8],
    ) -> Result<DbOps> {
        if self.dbs.is_empty() {
            return Ok(DbOps::RepoEmpty);
        }

        if let Some(mut database) = self.dbs.get_mut(&OsString::from(db_name)) {
            if let Some(document) = database.value_mut().list.get_mut(&OsString::from(doc_name)) {
                if document.keys.binary_search(&field_name.to_vec()).is_ok() {
                    let field_key: Vec<u8> = field_name.to_owned();
                    let stored_data;

                    let key_exists = document.fd.lock().await.get(&field_key)?;

                    match key_exists {
                        Some(data) => {
                            stored_data = data.to_vec();
                            let mut current_field_data =
                                bincode::deserialize::<FieldData>(&stored_data)?;
                            current_field_data.update(field_value);
                            let modified_field_data = bincode::serialize(&current_field_data)?;
                            match document
                                .fd
                                .lock()
                                .await
                                .insert(field_key, modified_field_data)?
                            {
                                Some(_) => Ok(DbOps::FieldModified),
                                // FIXME Decide what to do in case the field didnt exist
                                // Maybe push these to the database logs and alert DB Admin
                                None => Ok(DbOps::FieldInserted),
                            }
                        }
                        None => Ok(DbOps::FieldNotFound),
                    }
                } else {
                    Ok(DbOps::FieldNotFound)
                }
            } else {
                Ok(DbOps::DocumentNotFound)
            }
        } else {
            Ok(DbOps::DbNotFound)
        }
    }
}

/// #### Contains the list of documents and databases in-memory
/// ```
/// #[derive(Debug, Clone)]
/// struct Tdb {
///     list: HashMap<OsString, Document>,
/// }
///```
#[derive(Debug,)]
struct Tdb {
    list: HashMap<OsString, Document>,
    //Database<Document, Fileds>
    //rights: Option<HashMap<UserIdentifier, (Role, AccessRights)>>,
    //database_hash: Blake2hash,
    //secrecy: TuringSecrecy,
    //config: TuringConfig,
    //authstate: Assymetric Crypto
    //superuser: Only one
    // admins: vec![], -> (User, PriveledgeAccess)
    //users: vec![] -> """"
}

impl Tdb {
    /// Create a new in-memory database
    fn new() -> Tdb {
        Self {
            list: HashMap::new(),
        }
    }
}

/// #### Contains an in-memory representation of a document, with an async lock on sled file descriptor and document keys
/// ```
/// #[derive(Debug, Clone)]
/// struct Document {
///     fd: Mutex<sled::Db>,
///     keys: Vec<String>
/// }
/// ```
#[derive(Debug,)]
struct Document {
    fd: Mutex<sled::Db>,
    keys: Vec<Vec<u8>>,
}

/// Contains the structure of a value represented by a key
///
/// `Warning:` This is serialized using bincode so deserialization should be done using same version of bincode
/// ```
/// #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// pub struct FieldData {
///     data: Vec<u8>,
///     created: TAI64N,
///     modified: TAI64N,
/// }
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct FieldData {
    data: Vec<u8>,
    created: TAI64N,
    modified: TAI64N,
}

impl FieldData {
    /// Initializes a new `FieldData` struct
    pub fn new(value: &[u8]) -> FieldData {
        let current_time = TAI64N::now();

        Self {
            data: value.into(),
            created: current_time,
            modified: current_time,
        }
    }
    /// Updates a `FieldData` by modifying its time with a new `TAI64N` timestamp
    pub fn update(&mut self, value: &[u8]) -> &FieldData {
        self.data = value.into();
        self.modified = TAI64N::now();

        self
    }
}

// Get structure from file instead of making it a `pub` type
#[allow(unused_variables)]
#[derive(Debug, Serialize, Deserialize)]
enum Structure {
    Schemaless,
    Schema,
    Vector,
}
