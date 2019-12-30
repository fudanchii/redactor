use actix::prelude::*;
use futures::{
    compat::Future01CompatExt,
    future::{ok, TryFutureExt},
};
use std::error;
use std::time;

#[derive(Debug)]
pub struct Error(String);

impl<E: error::Error> From<E> for Error {
    fn from(err: E) -> Error {
        Error(err.description().to_string())
    }
}

pub struct Cmd(redis::Cmd);

impl Message for Cmd {
    type Result = Result<redis::Value, Error>;
}

pub struct Pipe(redis::Pipeline);

impl Pipe {
    pub fn line(rpipe: &mut redis::Pipeline) -> Pipe {
        Pipe(rpipe.clone())
    }
}

impl Message for Pipe {
    type Result = Result<redis::Value, Error>;
}

pub struct RedisActor {
    client: redis::Client,
}

impl RedisActor {
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
        Box::pin(
            self.client
                .get_async_connection()
                .compat()
                .and_then(move |conn| msg.0.query_async(conn).compat())
                .and_then(|result| ok(result.1))
                .map_err(Error::from),
        )
    }
}

impl Handler<Pipe> for RedisActor {
    type Result = ResponseFuture<Result<redis::Value, Error>>;

    fn handle(&mut self, msg: Pipe, _ctx: &mut Context<Self>) -> Self::Result {
        Box::pin(
            self.client
                .get_async_connection()
                .compat()
                .and_then(move |conn| msg.0.query_async(conn).compat())
                .and_then(|result| ok(result.1))
                .map_err(Error::from),
        )
    }
}

pub struct RedisConfig {
    use_breaker: bool,
    exponential_backoff: (time::Duration, time::Duration),
    errors_threshold: usize,
    connection_probe: bool,
    redis_uri: String,
}

pub struct RedisActorBuilder(RedisConfig);

impl RedisActorBuilder {
    pub fn use_breaker(mut self) -> Self {
        self.0.use_breaker = true;
        self
    }

    pub fn exponential_backoff(mut self, min: time::Duration, max: time::Duration) -> Self {
        self.0.exponential_backoff = (min, max);
        self
    }

    pub fn errors_threshold(mut self, count: usize) -> Self {
        self.0.errors_threshold = count;
        self
    }

    pub fn use_connection_probe(mut self) -> Self {
        self.0.connection_probe = true;
        self
    }

    pub fn set_uri<U: ToString>(mut self, arg: U) -> Self {
        self.0.redis_uri = arg.to_string();
        self
    }

    pub fn start(self) -> redis::RedisResult<Addr<RedisActor>> {
        let actor = RedisActor {
            client: redis::Client::open(self.0.redis_uri.as_ref())?,
        };

        Ok(actor.start())
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        let default_duration = time::Duration::from_secs(0);

        RedisConfig {
            use_breaker: false,
            exponential_backoff: (default_duration, default_duration),
            errors_threshold: 0,
            connection_probe: false,
            redis_uri: "redis://127.0.0.1:6379".to_string(),
        }
    }
}

pub fn val<T: redis::FromRedisValue>(v: Result<redis::Value, Error>) -> Result<T, Error> {
    v.and_then(|val| redis::from_redis_value(&val).map_err(Error::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis as r;

    #[actix_rt::test]
    async fn test_redis_cmd_ping() {
        let addr = RedisActor::build().start().unwrap();
        let result = addr.send(Cmd(r::cmd("PING"))).await.unwrap();
        let response: String = val(result).unwrap();
        assert_eq!(response, "PONG");
    }

    #[actix_rt::test]
    async fn test_redis_pipeline_ping() {
        let addr = RedisActor::build().start().unwrap();
        let result = addr.send(Pipe::line(r::pipe().cmd("PING"))).await.unwrap();
        let response: Vec<String> = val(result).unwrap();
        assert_eq!(response, vec!["PONG"]);
    }
}
