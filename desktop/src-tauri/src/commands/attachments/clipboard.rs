//! # Clipboard reading for the editor's image paste
//!
//! WebKitGTK does not expose non-text clipboard contents to the page: when an
//! image (a screenshot) or files copied in a file manager are pasted, the
//! paste event's `clipboardData` is empty and WebKit inserts the image itself
//! as a temporary `blob:` image. On Linux the editor therefore reads the
//! clipboard here, through the app's own GTK clipboard, and stores the image
//! like any other upload. The other platforms' webviews expose clipboard files
//! to the page directly, so these commands return nothing there.

/// `file://` URIs of the files on the clipboard (files copied in a file
/// manager); empty when the clipboard holds no files.
#[tauri::command]
pub async fn clipboard_file_uris(app: tauri::AppHandle) -> Result<Vec<String>, llava_core::Error> {
    #[cfg(target_os = "linux")]
    {
        on_main_thread(&app, || {
            gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD)
                .wait_for_uris()
                .into_iter()
                .map(|uri| uri.to_string())
                .collect()
        })
        .await
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Ok(Vec::new())
    }
}

/// The image on the clipboard encoded as PNG; an empty body when the clipboard
/// holds no image.
#[tauri::command]
pub async fn clipboard_image_png(
    app: tauri::AppHandle,
) -> Result<tauri::ipc::Response, llava_core::Error> {
    #[cfg(target_os = "linux")]
    {
        let png = on_main_thread(&app, || {
            gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD)
                .wait_for_image()
                .map(|pixbuf| pixbuf.save_to_bufferv("png", &[]))
        })
        .await?;

        match png {
            Some(Ok(bytes)) => Ok(tauri::ipc::Response::new(bytes)),
            Some(Err(err)) => Err(llava_core::Error::InternalError(format!(
                "failed to encode clipboard image: {err}"
            ))),
            None => Ok(tauri::ipc::Response::new(Vec::new())),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Ok(tauri::ipc::Response::new(Vec::new()))
    }
}

/// GTK may only be used from the main thread. The command itself is async (so
/// it does not run on, and block, the main thread) and waits for the result.
#[cfg(target_os = "linux")]
async fn on_main_thread<T: Send + 'static>(
    app: &tauri::AppHandle,
    read: impl FnOnce() -> T + Send + 'static,
) -> Result<T, llava_core::Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(read());
    })
    .map_err(|err| llava_core::Error::InternalError(format!("cannot read clipboard: {err}")))?;

    receiver
        .await
        .map_err(|_| llava_core::Error::InternalError("clipboard read was cancelled".to_string()))
}
