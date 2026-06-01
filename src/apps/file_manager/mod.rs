extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::fat32::DirEntryInfo;
use crate::framebuffer::{BLACK, WHITE};
use crate::wm::window::{Window, TITLE_H};

include!("model.rs");

mod actions;
mod drawing;
mod layout;
mod modals;
