use axum::{
    response::{sse::{Event, Sse}, IntoResponse, Response},
    http::header,
};
use futures::stream::{Stream, StreamExt};
use std::convert::Infallible;

#[allow(dead_code)]
pub fn string_stream_to_sse<S>(stream: S) -> Sse<S>
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    Sse::new(stream)
}

pub fn result_string_stream_to_sse(stream: impl Stream<Item = Result<String, std::convert::Infallible>> + Send + 'static) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream.map(|result: Result<String, Infallible>| {
        let data = match result {
            Ok(s) => s,
            Err(e) => match e {},
        };
        Ok(Event::default().data(data))
    });
    Sse::new(stream)
}

pub fn error_string_stream_to_sse<E: std::fmt::Debug>(stream: impl Stream<Item = Result<String, E>> + Send + 'static) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream.map(|result| {
        let data = match result {
            Ok(s) => s,
            Err(e) => format!("Error: {:?}", e),
        };
        Ok(Event::default().data(data))
    });
    Sse::new(stream)
}

#[allow(dead_code)]
pub async fn sse_response() -> Response {
    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        "Starting SSE stream..."
    ).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream as tstream;

    #[tokio::test]
    async fn test_result_string_stream_to_sse_ok() {
        let stream = tstream::iter(vec![Ok("hello".to_string()), Ok("world".to_string())]);
        let _sse = result_string_stream_to_sse(stream);
    }

    #[tokio::test]
    async fn test_error_string_stream_to_sse_err() {
        let stream = tstream::iter(vec![Ok("ok".to_string()), Err("error occurred")]);
        let _sse = error_string_stream_to_sse(stream);
    }

    #[tokio::test]
    async fn test_error_string_stream_to_sse_mixed() {
        let stream = tstream::iter(vec![
            Ok("hello".to_string()),
            Err("error1"),
            Ok("world".to_string()),
            Err("error2"),
        ]);
        let _sse = error_string_stream_to_sse(stream);
    }

    #[tokio::test]
    async fn test_sse_response() {
        let response = sse_response().await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
