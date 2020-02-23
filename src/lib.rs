//! Redactor is another implementation of Redis actor for Actix.
//! It's demonstrate how [redis-rs](https://github.com/mitsuhiko/redis-rs) async API
//! can be used within Actix.
//!
//! To use, add the following dependency to your actix/actix-web app
//!
//! ```ini
//! [dependencies]
//! actix-rt = "^1.0"
//! redactor = { git = "https://github.com/fudanchii/redactor.git" }
//! ```
//!
//! Usage example:
//!
//! ```rust
//! use redactor::{ RedisActor, Cmd, cmd, val };
//!
//! #[actix_rt::main]
//! async fn main() -> Result<(), redactor::Error> {
//!     let addr = RedisActor::build().start()?;
//!     let _ = addr.send(Cmd::wrap(cmd("SET").arg("key:name").arg("redactor"))).await?;
//!     let bc_future = addr.send(Cmd::wrap(cmd("BITCOUNT").arg("key:name")));
//!     let bitcount: usize = val(bc_future.await?)?;
//!     assert_eq!(bitcount, 32);
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]

mod redis_actor;

pub use redis_actor::{ Cmd, Pipe, RedisActor, val, Error };
pub use redis::{ cmd, pipe };
