use encoding_rs::{CoderResult, UTF_8};
use futures_util::StreamExt;
use llava_core::AppState;
use llava_core::online_auth::AccessToken;
use tauri::ipc::Channel;

#[tauri::command]
pub async fn ai_request(
    prompt_context: llava_core::ai::AiPromptContext,
    channel: Channel<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    crate::commands::utils::check_connection_before_request(state.clone())?;
    let client = state.server_client.clone();
    let access_token = {
        let guard = state
            .access_token
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard
            .as_ref()
            .ok_or(llava_core::Error::NotLoggedIn)?
            .clone()
    };
    let res = llava_core::ai::send_ai_request(client.clone(), prompt_context.clone(), access_token).await;
    match res {
        Err(llava_core::Error::OnlineSessionExpired) => {
            let online_id: String = {
                let guard = state
                    .online_user_id
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;
                guard
                    .as_ref()
                    .ok_or(llava_core::Error::LockError)?
                    .clone()
            };
            let new_token: AccessToken =
                crate::commands::sync::sync::refresh_access_token(&state, &client, &online_id).await?;
            let res = llava_core::ai::send_ai_request(client.clone(), prompt_context, new_token).await?;
            handle_response(res, channel).await?;
            Ok(())
        }
        Err(err) => Err(err),
        Ok(res) => {
            handle_response(res, channel).await?;
            Ok(())
        }
    }
}

async fn handle_response(
    response: reqwest::Response,
    channel: Channel<String>,
) -> Result<(), llava_core::Error> {
    let mut stream = response.bytes_stream();
    let mut decoder = UTF_8.new_decoder();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| llava_core::Error::AiError)?;
        send_decoded(&mut decoder, &chunk, false, &channel)?;
    }

    // Flush whatever partial sequence is left buffered at end of stream.
    send_decoded(&mut decoder, &[], true, &channel)?;

    Ok(())
}

/// Feeds `src` through the incremental decoder and sends whatever
/// complete text comes out. A multi-byte char split across two chunks
/// stays buffered in `decoder` until the rest of it arrives, instead of
/// being replaced with U+FFFD.
fn send_decoded(
    decoder: &mut encoding_rs::Decoder,
    src: &[u8],
    last: bool,
    channel: &Channel<String>,
) -> Result<(), llava_core::Error> {
    let mut read = 0;

    loop {
        let mut buf = String::with_capacity(
            decoder
                .max_utf8_buffer_length(src.len() - read)
                .unwrap_or(4096),
        );

        let (result, consumed, _) = decoder.decode_to_string(&src[read..], &mut buf, last);
        read += consumed;

        if !buf.is_empty() {
            channel.send(buf).map_err(|_| llava_core::Error::AiError)?;
        }

        match result {
            CoderResult::InputEmpty => break,
            CoderResult::OutputFull => continue,
        }
    }

    Ok(())
}