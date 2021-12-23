use crate::device::{
    Device,
    Tracking,
};
use alvr_common::prelude::*;
use alvr_sockets::PrivateIdentity;
use once_cell::sync::OnceCell;

static DEVICE: OnceCell<Device> = OnceCell::new();
static IDENTITY: OnceCell<PrivateIdentity> = OnceCell::new();
static DEVICE_DATA_PRODUCER: OnceCell<Box<dyn DeviceDataProducer>> = OnceCell::new();

pub trait DeviceDataProducer: Sync + Send {
    fn get_device(&self) -> StrResult<Device>;
    fn get_tracking(&self) -> StrResult<Tracking>;
}

pub fn set_identity(identity: PrivateIdentity) -> StrResult {
    IDENTITY.set(identity)
        .map_err(|_| "The IDENTITY is already set and will not change.".into())
}

pub fn get_identity() -> StrResult<&'static PrivateIdentity> {
    IDENTITY.get()
        .ok_or("The IDENTITY has not been initialized.".into())
}

pub fn set_device_data_producer(producer: Box<dyn DeviceDataProducer>) -> StrResult {
    DEVICE_DATA_PRODUCER.set(producer)
        .map_err(|_| "The DEVICE_DATA_PRODUCER is already set and will not change.".into())
}

pub fn get_device() -> StrResult<&'static Device> {
    Ok(DEVICE.get_or_init(|| {
        let producer = DEVICE_DATA_PRODUCER.get()
            .expect("The DEVICE_DATA_PRODUCER has not been initialized.");
        producer.get_device()
            .expect("The DEVICE_DATA_PRODUCER can't produce a Device instance.")
    }))
}

pub fn get_tracking() -> StrResult<Tracking> {
    let producer = trace_err!(DEVICE_DATA_PRODUCER.get()
        .ok_or("The DEVICE_DATA_PRODUCER has not been initialized."))?;
    let tracking = trace_err!(producer.get_tracking())?;
    Ok(tracking)
}
