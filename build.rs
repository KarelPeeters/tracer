fn main() {
    windows::build!(
        windows::win32::windows_programming::PROCESS_CREATION_FLAGS,
        windows::win32::system_services::{GetCurrentProcess, SetPriorityClass},
    )
}