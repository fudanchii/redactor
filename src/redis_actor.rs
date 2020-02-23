//! Redis actor module.

use actix::prelude::*;
use futures::future::{TryFutureExt};
use std::error;

/// Represent error for redis actor, this basically wraps
/// any type that implements `std::error::Error`.
#[derive(Debug)]
pub struct Error(String);

impl<E: error::Error> From<E> for Error {
    fn from(err: E) -> Error {
        Error(err.description().to_string())
    }
}

/// Cmd represents command message sent to redis actor.
pub struct Cmd(redis::Cmd);

impl Cmd {

    /// Create Cmd by wrapping `redis::Cmd` struct.
    pub fn wrap(cmd: &mut redis::Cmd) -> Cmd {
        Cmd(cmd.clone())
    }
}

impl Message for Cmd {
    type Result = Result<redis::Value, Error>;
}

/// Pipe represents pipeline message sent to redis actor.
/// This wraps `redis::Pipeline` to support multiple command sent
/// in one roundtrip.
pub struct Pipe(redis::Pipeline);

impl Pipe {

    /// Create Pipe by wrapping `redis::Pipeline` struct.
    pub fn line(rpipe: &mut redis::Pipeline) -> Pipe {
        Pipe(rpipe.clone())
    }
}

impl Message for Pipe {
    type Result = Result<redis::Value, Error>;
}

/// The actor itself.
pub struct RedisActor {
    client: redis::Client,
}

impl RedisActor {

    /// Create actor with builder pattern, call `start()` to start the actor.
    pub fn build() -> RedisActorBuilder {
        RedisActorBuilder(RedisConfig::default())
    }
}

impl Actor for RedisActor {
    type Context = Context<Self>;
}

impl Handler<Cmd> for RedisActor {
    type Result = ResponseFuture<Result<redis::Value, Error>>;

    fn handle(&mut self, msg: Cmd, _ctx: &mut Context<Self>) -> Self::Result {
        let client = self.client.clone();
        Box::pin(async move {
            let mut conn = client.get_async_connection().map_err(Error::from).await?;
            msg.0.query_async(&mut conn).map_err(Error::from).await
        })
    }
}

impl Handler<Pipe> for RedisActor {
    type Result = ResponseFuture<Result<redis::Value, Error>>;

    fn handle(&mut self, msg: Pipe, _ctx: &mut Context<Self>) -> Self::Result {
        let client = self.client.clone();
        Box::pin(async move {
            let mut conn = client.get_async_connection().map_err(Error::from).await?;
            msg.0.query_async(&mut conn).map_err(Error::from).await
        })
    }
}

pub struct RedisConfig {
    redis_uri: String,
}

pub struct RedisActorBuilder(RedisConfig);

impl RedisActorBuilder {
    pub fn set_uri<U: ToString>(mut self, arg: U) -> Self {
        self.0.redis_uri = arg.to_string();
        self
    }

    pub fn start(self) -> Result<Addr<RedisActor>, Error> {
        let actor = RedisActor {
            client: redis::Client::open(self.0.redis_uri.as_ref())?,
        };

        Ok(actor.start())
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        RedisConfig {
            redis_uri: "redis://127.0.0.1:6379".to_string(),
        }
    }
}

/// Translate redis value to Rust types.
pub fn val<T: redis::FromRedisValue>(v: Result<redis::Value, Error>) -> Result<T, Error> {
    v.and_then(|val| redis::from_redis_value(&val).map_err(Error::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis as r;

    #[actix_rt::test]
    async fn test_redis_cmd_ping() -> Result<(), Error> {
        let addr = RedisActor::build().start()?;
        let result = addr.send(Cmd::wrap(&mut r::cmd("PING"))).await?;
        let response: String = val(result)?;
        assert_eq!(response, "PONG");
        Ok(())
    }

    #[actix_rt::test]
    async fn test_redis_pipeline_ping() -> Result<(), Error> {
        let addr = RedisActor::build().start()?;
        let result = addr.send(Pipe::line(r::pipe().cmd("PING"))).await?;
        let response: Vec<String> = val(result)?;
        assert_eq!(response, vec!["PONG"]);
        Ok(())
    }
}
