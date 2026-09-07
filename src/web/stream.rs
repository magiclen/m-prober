use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio_stream::{Stream, StreamExt, wrappers::WatchStream};

use super::{sampler::Sample, state::AppState};

/// Push a comment often enough that a proxy in the middle does not drop an idle connection.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub async fn all(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.sampler.subscribe();
    let detect_interval = state.detect_interval;

    // `WatchStream::new` yields the value the channel already holds, so a client which connects between two rounds is served at once, unless that value was taken before sampling paused.
    let stream = WatchStream::new(receiver).filter_map(move |sample: Option<Sample>| {
        let sample = sample.filter(|sample| sample.is_fresh(detect_interval))?;

        Some(Ok(Event::default()
            .json_data(&*sample.snapshot)
            .unwrap_or_else(|error| Event::default().event("error").data(error.to_string()))))
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL))
}
