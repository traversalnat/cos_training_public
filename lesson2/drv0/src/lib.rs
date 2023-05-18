#![no_std]

use drv_common::{module, CallEntry, Driver};

module!(
    type: Driver, name: "rtc", compatible: "google,goldfish-rtc");
