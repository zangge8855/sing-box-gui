pub fn is_running_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        unsafe {
            // Use OpenProcessToken + GetTokenInformation(TokenElevation).
            #[allow(clippy::upper_case_acronyms)]
            type BOOL = i32;
            #[allow(clippy::upper_case_acronyms)]
            type HANDLE = *mut c_void;
            unsafe extern "system" {
                fn GetCurrentProcess() -> HANDLE;
                fn OpenProcessToken(
                    process: HANDLE,
                    access: u32,
                    token: *mut HANDLE,
                ) -> BOOL;
                fn GetTokenInformation(
                    token: HANDLE,
                    class: u32,
                    buf: *mut c_void,
                    len: u32,
                    ret_len: *mut u32,
                ) -> BOOL;
                fn CloseHandle(h: HANDLE) -> BOOL;
            }
            // TOKEN_QUERY = 0x0008 ; TokenElevation = 20
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) == 0 {
                return false;
            }
            let mut elevated: i32 = 0;
            let mut ret = 0u32;
            let ok = GetTokenInformation(
                token,
                20,
                &mut elevated as *mut _ as *mut c_void,
                std::mem::size_of::<i32>() as u32,
                &mut ret,
            );
            CloseHandle(token);
            ok != 0 && elevated != 0
        }
    }
    #[cfg(unix)]
    {
        // On unix a process is "elevated" enough for TUN if its effective uid
        // is 0 (CAP_NET_ADMIN) or it has the NET_ADMIN capability via file caps.
        // We approximate with euid==0 — covers the typical sudo install case.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        true // assume elevated where we cannot check
    }
}
