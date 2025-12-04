use std::{pin::Pin, sync::LazyLock, time::Duration};

use hid_rs::SafeCallback2;
use sayo_api_rs::device::SayoDevice;
use tokio::time::sleep;

static HID_DEVICE_EVENT_HANDLER: LazyLock<SafeCallback2<u128, Vec<u8>, ()>> = LazyLock::new(|| {
    SafeCallback2::<u128, Vec<u8>, ()>::new(move |uuid, data| {
        Box::pin(async move {
            println!("Received report from device {:?}, Data: {:?}", uuid, data);
        }) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    })
});

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    hid_rs::Hid::init_hid().await?;
    println!("HID initialized.");
    sayo_api_rs::device::init_sayo_device().await;
    println!("Sayo device initialized.");

    let runtime_handle = tokio::runtime::Handle::current();

    sayo_api_rs::device::sub_connection_changed(SafeCallback2::<u128, bool, ()>::new(
        move |uuid, event_type| {
            let handle = runtime_handle.clone();
            Box::pin(async move {
                handle.spawn(async move {
                    println!(
                        "Device connection changed: {:?}, Type: {:?}",
                        uuid, event_type
                    );
                    let device = SayoDevice::from(uuid);
                    match device.get_system_info().await {
                        Some(sys_info) => println!("System Info: {:?}", sys_info),
                        None => println!("Failed to fetch system info for {:?}", uuid),
                    }
                });
            }) as Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        },
    ))
    .await;

    sleep(Duration::from_secs(100)).await;
    Ok(())
}
