// btleplug Source Code File
//
// Copyright 2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
//
// Some portions of this file are taken and/or modified from Rumble
// (https://github.com/mwylde/rumble), using a dual MIT/Apache License under the
// following copyright:
//
// Copyright (c) 2014 The Rust Project Developers

use crate::{Error, Result};
use super::super::bindings;

use bindings::windows::devices::bluetooth::generic_attribute_profile::{
        GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
        GattCommunicationStatus, GattValueChangedEventArgs,
};
use bindings::windows::foundation::{EventRegistrationToken, TypedEventHandler};
use bindings::windows::storage::streams::{DataReader, DataWriter};

pub type NotifiyEventHandler = Box<dyn Fn(Vec<u8>) + Send>;

pub struct BLECharacteristic {
    characteristic: GattCharacteristic,
    notify_token: Option<EventRegistrationToken>,
}

unsafe impl Send for BLECharacteristic {}
unsafe impl Sync for BLECharacteristic {}

impl BLECharacteristic {
    pub fn new(characteristic: GattCharacteristic) -> Self {
        BLECharacteristic {
            characteristic,
            notify_token: None,
        }
    }

    pub fn write_value(&self, data: &[u8]) -> Result<()> {
        let writer = DataWriter::new().unwrap();
        writer.write_bytes(data).unwrap();
        let buffer = writer.detach_buffer().unwrap();
        let result = self
            .characteristic
            .write_value_async(&buffer)
            .unwrap()
            .get()
            .unwrap();
        if result == GattCommunicationStatus::Success {
            Ok(())
        } else {
            Err(Error::NotSupported("get_status".into()))
        }
    }

    pub fn read_value(&self) -> Result<Vec<u8>> {
        let result = self
            .characteristic
            .read_value_async()
            .unwrap()
            .get()
            .unwrap();
        if result.status().unwrap() == GattCommunicationStatus::Success {
            let value = result.value().unwrap();
            let reader = DataReader::from_buffer(&value).unwrap();
            let len = reader.unconsumed_buffer_length().unwrap() as usize;
            let mut input = vec![0u8; len];
            reader.read_bytes(&mut input[0..len]).unwrap();
            Ok(input)
        } else {
            Err(Error::NotSupported("get_status".into()))
        }
    }

    pub fn subscribe(&mut self, on_value_changed: NotifiyEventHandler) -> Result<()> {
        let value_handler = TypedEventHandler::new(
            move |_: &GattCharacteristic, args: &GattValueChangedEventArgs| {
                //let args = unsafe { &*args };
                let args = &*args;
                let value = args.characteristic_value().unwrap();
                let reader = DataReader::from_buffer(&value).unwrap();
                let len = reader.unconsumed_buffer_length().unwrap() as usize;
                let mut input = vec![0u8; len];
                reader.read_bytes(&mut input[0..len]).unwrap();
                info!("changed {:?}", input);
                on_value_changed(input);
                Ok(())
            },
        );
        let token = self
            .characteristic
            .value_changed(&value_handler)
            .unwrap();
        self.notify_token = Some(token);
        let config = GattClientCharacteristicConfigurationDescriptorValue::Notify;
        let status = self
            .characteristic
            .write_client_characteristic_configuration_descriptor_async(config)
            .unwrap()
            .get()
            .unwrap();
        info!("subscribe {:?}", status);
        Ok(())
    }

    pub fn unsubscribe(&mut self) -> Result<()> {
        if let Some(token) = &self.notify_token {
            self.characteristic.remove_value_changed(token).unwrap();
        }
        self.notify_token = None;
        let config = GattClientCharacteristicConfigurationDescriptorValue::None;
        let status = self
            .characteristic
            .write_client_characteristic_configuration_descriptor_async(config)
            .unwrap()
            .get()
            .unwrap();
        info!("unsubscribe {:?}", status);
        Ok(())
    }
}

impl Drop for BLECharacteristic {
    fn drop(&mut self) {
        if let Some(token) = &self.notify_token {
            let result = self.characteristic.remove_value_changed(token);
            if let Err(err) = result {
                info!("Drop:remove_connection_status_changed {:?}", err);
            }
        }
    }
}
