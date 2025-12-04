#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn future_delay(milliseconds: u32) -> impl Future<Output = ()> {
    use futures_timer::Delay;
    use std::time::Duration;
    let future = Delay::new(Duration::from_millis(milliseconds.into()));
    return future;
}
#[cfg(target_arch = "wasm32")]
pub fn future_delay(milliseconds: u32) -> impl Future<Output = ()> {
    use gloo_timers::future::TimeoutFuture;
    return TimeoutFuture::new(milliseconds);
}
#[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
pub fn future_delay(milliseconds: u32) -> impl Future<Output = ()> {
    use futures_timer::Delay;
    use std::time::Duration;
    Delay::new(Duration::from_millis(milliseconds.into()))
}
