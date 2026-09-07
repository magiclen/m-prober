use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use tokio_stream::{Stream, StreamExt, wrappers::WatchStream};

use super::{sampler::Snapshot, state::AppState};

/// Push a comment often enough that a proxy in the middle does not drop an idle connection.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub async fn all(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.sampler.subscribe();

    // `WatchStream::new` yields the value the channel already holds, so a client which connects between two rounds is served at once.
    let stream = WatchStream::new(receiver).filter_map(|snapshot: Option<Arc<Snapshot>>| {
        let snapshot = snapshot?;

        Some(Ok(Event::default()
            .json_data(&*snapshot)
            .unwrap_or_else(|error| Event::default().event("error").data(error.to_string()))))
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE_INTERVAL))
}
