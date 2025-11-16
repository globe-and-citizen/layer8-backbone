use pingora::prelude::Session;

pub(crate) async fn get_request_body(session: &mut Session) -> pingora::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        match session.read_request_body().await {
            Ok(option) => match option {
                Some(chunk) => body.extend_from_slice(&chunk),
                None => break,
            },
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(body)
}
