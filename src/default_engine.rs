use async_std::{
    task,
    fs::{File, OpenOptions, DirBuilder},
    net::{TcpListener, TcpStream},
	io::{prelude::*},
	path::PathBuf,
};
use std::collections::HashMap;
use custom_codes::{FileOps, DbOps};
use tai64::TAI64N;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{UserIdentifier, Role, AccessRights, TuringFeedsError, AutoGeneratedIdentifier, UserDefinedName, SeaHashCipher, NoOfEntries, CreateTaiTime, ModifiedTaiTime};

/// No need for rights as the user who decrypts the DB has total access

#[derive(Debug)]
pub struct TuringFeeds {
	created: TAI64N,
	db_docs: Option<TuringFeedsDB>,
	//hash: Blake2hash,
	//secrecy: TuringSecrecy,
	//config: TuringConfig,
	//authstate: Assymetric Crypto
	//superuser: Only one
	// admins: vec![], -> (User, PriveledgeAccess)
	//users: vec![] -> """"
}

impl TuringFeeds {
	/// Recursively walk through the Directory
	/// Load all the Directories into memory
	/// Hash and Compare with Persisted Hash to check for corruption
	/// Throw errors if any otherwise 
	pub async fn init() {
		
/*
		match DirBuilder::new()
			.recursive(false)
			.create(&turing_path)
			{
				Ok(val) => {
					dbg!(val); 
					Ok(DbOps::Created)
				},
				Err(error) => {
					if error.kind() == std::io::ErrorKind::AlreadyExists {
						println!("[CURSOR]: {:?}", len);
						println!("{:?}", contents.trim());
					}
					
					Err(TuringFeedsError::IoError(error))
				}
			}*/
	}
}

#[derive(Debug)]
pub struct TFDocument {
	// Gives the document path
	identifier: AutoGeneratedIdentifier,
	primary_key: UserDefinedName,
	indexes: Vec<String>,
	hash: SeaHashCipher,
	size: NoOfEntries,
	create_time: CreateTaiTime,
	modified_time: ModifiedTaiTime
}

impl TFDocument {
	pub fn new() -> Self {
		Self {
			identifier: Uuid::new_v4().to_hyphenated().to_string(),
			primary_key: Default::default(),
			indexes: Vec::default(),
			hash: Default::default(),
			size: Default::default(),
			create_time: TAI64N::now(),
			modified_time: TAI64N::now(),
		}
	}
}

#[derive(Debug)]
enum DocumentRights {
	/// Create Access
	C,
	/// Read Access
	R,
	/// Write Access
	W,
	/// Delete Access
	D,
	/// Forward
	F,
	/// Create Read Write Delete Access
	CRWD,
	/// Read Write Access
	RW,
}

#[derive(Debug)]
pub struct TuringFeedsDB {	
	identifier: AutoGeneratedIdentifier,
	db_name: UserDefinedName,
	time: TAI64N,
	document_list: Option<Vec<TFDocument>>,
	rights: Option<HashMap<UserIdentifier, (Role, AccessRights)>>,
	//database_hash: Blake2hash,
	//secrecy: TuringSecrecy,
	//config: TuringConfig,
	//authstate: Assymetric Crypto
	//superuser: Only one
	// admins: vec![], -> (User, PriveledgeAccess)
	//users: vec![] -> """"
}

impl TuringFeedsDB {
	pub fn new() -> Self {
		Self {
			identifier: Uuid::new_v4().to_hyphenated().to_string(),
			db_name: Default::default(),
			time: TAI64N::now(),
			document_list: Default::default(),
			rights: Option::default(),
		}
	}
}

struct TFTable {
	identifier: AutoGeneratedIdentifier,
	indexes: Vec<String>,
	primary_key: Option<String>,
	secrecy: TuringSecrecy,
}

enum TuringConfig {
	DefaultCOnfig,
	WriteACKs,

}
	// Shows the level of security from the database level to a document level
enum TuringSecrecy {
	DatabaseMode,
	TableMode,
	DocumentMode,
	DefaultMode,
	InactiveMode,
}