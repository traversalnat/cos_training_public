#![no_std]
use drv_common::{module, CallEntry, Driver};

module!(
    type: Driver, name: "uart", compatible: "ns16550a");


