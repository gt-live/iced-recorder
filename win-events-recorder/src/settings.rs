use std::fmt;
use std::ops;

//pub type DeviceName = String;
// tuple structs to define new types
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DeviceName(pub String);
// or a new constructor in impl

//impl fmt::Display for DeviceName {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        write!(f, "{}", self.0)
//    }
//}

//impl AsRef<str> for DeviceName {
//    fn as_ref(&self) -> &str {
//        &self.0
//    }
//}

impl ops::Deref for DeviceName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DeviceSelection {
    Default,
    Selected(DeviceName),
    //Inverse(DeviceName),
}
pub type InputDeviceSelection = DeviceSelection;
pub type OutputDeviceSelection = DeviceSelection;

pub const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
