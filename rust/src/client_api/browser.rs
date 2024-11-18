use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{ErrorEvent, MessageEvent};

use super::{client_events::ClientRequest, Error, HostResult};

type Connection = web_sys::WebSocket;

/// Represents a WebSocket API interface with handlers for message processing,
/// error handling, and connection lifecycle events.
pub struct WebApi {
    conn: Connection,
    error_handler: Box<dyn FnMut(Error) + 'static>,
}

impl WebApi {
    /// Initializes and starts a WebSocket connection with provided handlers for
    /// messages, errors, and the connection opening event.
    ///
    /// # Parameters
    /// - `conn`: The WebSocket connection instance.
    /// - `result_handler`: A function invoked to handle deserialized messages.
    /// - `error_handler`: A function invoked to handle errors.
    /// - `onopen_handler`: A function invoked when the connection is successfully opened.
    ///
    /// # Returns
    /// An instance of `WebApi` configured with the provided handlers.
    pub fn start<ErrFn>(
        conn: Connection,
        result_handler: impl FnMut(HostResult) + 'static,
        error_handler: ErrFn,
        onopen_handler: impl FnOnce() + 'static,
    ) -> Self
    where
        ErrFn: FnMut(Error) + Clone + 'static,
    {
        // Wrap the error handler in a reference-counted, mutable container for shared access.
        let eh = Rc::new(RefCell::new(error_handler.clone()));
        // Wrap the result handler similarly for shared access.
        let result_handler = Rc::new(RefCell::new(result_handler));

        // Create a closure to handle incoming WebSocket messages.
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Extract the Blob data from the WebSocket message.
            let value: JsValue = e.data();
            let blob: web_sys::Blob = value.into();

            // Create a FileReader instance to read the Blob's contents asynchronously.
            let file_reader = web_sys::FileReader::new().unwrap();

            // Clone references to the FileReader and handlers for use in the onloadend closure.
            let fr_clone = file_reader.clone();
            let eh_clone = eh.clone();
            let result_handler_clone = result_handler.clone();

            // Closure to handle when the FileReader has completed reading.
            let onloadend_callback = Closure::<dyn FnMut()>::new(move || {
                // Attempt to retrieve and deserialize the data as a byte array.
                let array_buffer = fr_clone
                    .result()
                    .unwrap()
                    .dyn_into::<js_sys::ArrayBuffer>()
                    .unwrap();
                let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
                // Deserialize the bytes into a `HostResult`.
                let response: HostResult = match bincode::deserialize(&bytes) {
                    Ok(val) => val,
                    // Handle deserialization errors by invoking the error handler.
                    Err(err) => {
                        eh_clone.borrow_mut()(Error::ConnectionError(serde_json::json!({
                            "error": format!("{err}"),
                            "source": "host response deserialization"
                        })));
                        return;
                    }
                };
                // Invoke the result handler with the deserialized response.
                result_handler_clone.borrow_mut()(response);
            });

            // Set the FileReader's onloadend event to the closure and start reading the Blob.
            file_reader.set_onloadend(Some(onloadend_callback.as_ref().unchecked_ref()));
            file_reader.read_as_array_buffer(&blob).unwrap();
            onloadend_callback.forget(); // Prevent the closure from being dropped prematurely.
        });
        conn.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget(); // Prevent the closure from being dropped prematurely.

        // Create a closure to handle WebSocket errors.
        let mut eh = error_handler.clone();
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            let error = format!(
                "error: {file}:{lineno}: {msg}",
                file = e.filename(),
                lineno = e.lineno(),
                msg = e.message()
            );
            // Invoke the error handler with formatted error details.
            eh(Error::ConnectionError(serde_json::json!({
                "error": error, "source": "exec error"
            })));
        });
        conn.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget(); // Prevent the closure from being dropped prematurely.

        // Create a closure to handle the WebSocket's open event.
        let onopen_callback = Closure::<dyn FnOnce()>::once(move || {
            onopen_handler(); // Invoke the provided onopen handler.
        });
        conn.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget(); // Prevent the closure from being dropped prematurely.

        // Set the WebSocket's binary type to `Blob` for incoming binary messages.
        conn.set_binary_type(web_sys::BinaryType::Blob);

        // Return a new instance of `WebApi` with the configured WebSocket and error handler.
        WebApi {
            conn,
            error_handler: Box::new(error_handler),
        }
    }

    /// Sends a serialized `ClientRequest` over the WebSocket connection.
    ///
    /// # Parameters
    /// - `request`: The `ClientRequest` to be serialized and sent.
    ///
    /// # Returns
    /// - `Ok(())` if the request was successfully sent.
    /// - `Err(Error)`: An error if serialization or sending fails.
    pub async fn send(&mut self, request: ClientRequest<'static>) -> Result<(), Error> {
        // Serialize the request using bincode.
        let send = bincode::serialize(&request)?;
        // Send the serialized request as a byte array over the WebSocket.
        self.conn.send_with_u8_array(&send).map_err(|err| {
            // Attempt to parse the error using `serde_wasm_bindgen`.
            let err: serde_json::Value = match serde_wasm_bindgen::from_value(err) {
                Ok(e) => e,
                Err(e) => {
                    // Handle serialization errors by invoking the error handler.
                    let e = serde_json::json!({
                        "error": format!("{e}"),
                        "origin": "request serialization",
                        "request": format!("{request:?}"),
                    });
                    (self.error_handler)(Error::ConnectionError(e.clone()));
                    return Error::ConnectionError(e);
                }
            };
            // Handle sending errors by invoking the error handler.
            (self.error_handler)(Error::ConnectionError(serde_json::json!({
                "error": err,
                "origin": "request sending",
                "request": format!("{request:?}"),
            })));
            Error::ConnectionError(err)
        })?;
        Ok(())
    }

    /// Closes the WebSocket connection with a specified close code and reason.
    ///
    /// # Parameters
    /// - `cause`: A string describing the reason for disconnecting.
    pub fn disconnect(self, cause: impl AsRef<str>) {
        // Close the WebSocket connection with a code (1000 indicates normal closure).
        let _ = self.conn.close_with_code_and_reason(1000, cause.as_ref());
    }
}
