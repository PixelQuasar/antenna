use futures::channel::oneshot;

/// This function is helper that used to convert callbacks to future.
/// It uses channel semantic to generate sender and receiver, that are bound to
/// passed callback, and future gets fulfilled when callback is done
pub fn async_callback<T, F>(cb: F) -> impl Future<Output = Option<T>>
where
    T: 'static,
    F: FnOnce(Box<dyn FnMut(T)>),
{
    let (tx, rx) = oneshot::channel();
    let mut tx = Some(tx);
    cb(Box::new(move |value| {
        if let Some(tx) = tx.take() {
            let _ = tx.send(value);
        }
    }));

    async move { rx.await.ok() }
}
