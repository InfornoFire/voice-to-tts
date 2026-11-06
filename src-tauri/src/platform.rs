//! Process settings of the operating system.

use windows::Win32::System::Threading::{
    BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess, SetPriorityClass,
};

/// Runs the app below normal priority, so it yields to games and calls.
pub fn lower_priority() {
    // For safety, both calls only use the pseudo handle of the current process.
    let _ = unsafe { SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS) };
}
