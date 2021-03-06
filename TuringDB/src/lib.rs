#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! TuringDB is a key-value database written using async code and backed by `sled.rs`  embedded key/value store.
//! This is just a library version, the server can be found by searching crates-io for `turingdb-server`
//! or checking under the Github repository https://github.com/charleschege/TuringDB/TuringDB-Server/
//!
//!
//! This codebase uses `sled` as the underlying key/value store and builds upon that
//! to provide other functionality like
//!
//! 1. in-memory keys,
//! 2. async-locks for increased acid guarantees
//! 3. Insert operations will fail if a key already exists, use `modify()` method on a key to change its value
//! 4. in-memory locks to ensure that document locks are not dropped until the application is halted
//!
//! Some features that are under development include
//!
//! 1. Replication
//! 2. Multi-cluster queries
//! 3. Changefeeds without polling, inspired by RethinkDB
//! 4. JSON support
//!
//!
//! This module contains all the modules for the database engine that you can use to build a database server
//! or embed in your own app
mod engine;
pub use engine::*;
