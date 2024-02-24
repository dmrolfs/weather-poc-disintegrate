use async_trait::async_trait;
use disintegrate::{query, Event, EventListener, PersistedEvent, StreamQuery};
use std::fmt::Debug;
use tracing::log::Level;

pub struct TracingProcessor<E>
where
    E: Debug + Clone,
{
    id: &'static str,
    level: Level,
    query: StreamQuery<E>,
}

impl<E> TracingProcessor<E>
where
    E: disintegrate::Event + Debug + Clone,
{
    pub fn new(id: &'static str, level: Level) -> Self {
        Self { id, level, query: query!(E) }
    }
}

#[async_trait]
impl<E> EventListener<E> for TracingProcessor<E>
where
    E: Debug + Clone + Event + Sync + Send,
{
    type Error = crate::errors::WeatherError;

    fn id(&self) -> &'static str {
        self.id
    }

    fn query(&self) -> &StreamQuery<E> {
        &self.query
    }

    async fn handle(&self, event: PersistedEvent<E>) -> Result<(), Self::Error> {
        match self.level {
            Level::Trace => trace!(event=?event.into_inner(), "event"),
            Level::Debug => debug!(event=?event.into_inner(), "event"),
            Level::Info => info!(event=?event.into_inner(), "event"),
            Level::Warn => warn!(event=?event.into_inner(), "event"),
            Level::Error => error!(event=?event.into_inner(), "event"),
        }

        Ok(())
    }
}
