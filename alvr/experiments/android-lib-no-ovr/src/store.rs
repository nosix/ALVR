use crate::device::Device;
use alvr_common::prelude::*;
use alvr_sockets::PrivateIdentity;
use once_cell::sync::OnceCell;

static DEVICE: OnceCell<Device> = OnceCell::new();
static IDENTITY: OnceCell<PrivateIdentity> = OnceCell::new();
static DEVICE_DATA_PRODUCER: OnceCell<Box<dyn DeviceDataProducer>> = OnceCell::new();

pub trait DeviceDataProducer: Sync + Send {
    fn request(&self, data_kind: i8) -> StrResult;
}

pub fn set_device(device: Device) -> StrResult {
    DEVICE.set(device)
        .map_err(|_| "The DEVICE is already set and will not change.".into())
}

pub fn get_device() -> StrResult<&'static Device> {
    DEVICE.get()
        .ok_or("The DEVICE has not been initialized.".into())
}

pub fn set_identity(identity: PrivateIdentity) -> StrResult {
    IDENTITY.set(identity)
        .map_err(|_| "The IDENTITY is already set and will not change.".into())
}

pub fn get_identity() -> StrResult<&'static PrivateIdentity> {
    IDENTITY.get()
        .ok_or("The IDENTITY has not been initialized.".into())
}

pub fn set_data_producer(producer: Box<dyn DeviceDataProducer>) -> StrResult {
    DEVICE_DATA_PRODUCER.set(producer)
        .map_err(|_| "The DEVICE_DATA_PRODUCER is already set and will not change.".into())
}

pub fn request_data(data_kind: i8) -> StrResult {
    DEVICE_DATA_PRODUCER.get()
        .map(|f| f.request(data_kind))
        .unwrap_or(Err("The DEVICE_DATA_PRODUCER has not been initialized.".into()))
}