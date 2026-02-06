use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{Stream, StreamExt};
use serde::Serialize;
use std::convert::Infallible;
use std::time::Duration;

pub fn create_sse_stream<S, T>(stream: S) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
    S: Stream<Item = T> + Send + 'static,
    T: Serialize,
{
    let event_stream = stream
        .map(|data| Ok(Event::default().data(serde_json::to_string(&data).unwrap_or_default())));

    Sse::new(event_stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    )
}

#[allow(dead_code)]
pub fn create_done_event() -> Event {
    Event::default().data("[DONE]")
}
