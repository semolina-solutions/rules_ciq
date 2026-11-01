//! MTP library for Garmin devices.
//!
//! Provides a unified Rust interface for listing folders/files, uploading files,
//! and downloading files on physical Garmin MTP devices:
//! - On Unix (macOS and Linux), uses `libmtp`.
//! - On Windows, uses the native Windows Portable Devices (WPD) COM API (`ole32` /
//!   `PortableDeviceApi.dll`), which coexists with `WpdUsb.sys` and Windows Explorer
//!   without requiring custom USB drivers.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Wait for device definition (seconds)
    #[arg(long, default_value_t = 60)]
    wait_for: u64,

    /// Source and destination file pairs
    #[arg(num_args = 2.., value_names = ["SRC", "DST"])]
    files: Vec<String>,
}

pub(crate) fn split_soft_fail(path: &str) -> (&str, bool) {
    if let Some(stripped) = path.strip_prefix('?') {
        (stripped, true)
    } else {
        (path, false)
    }
}

pub(crate) fn parent_folder_parts(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if !parts.is_empty() {
        parts.pop();
    }
    parts
}

#[cfg(unix)]
mod imp {
    use super::{parent_folder_parts, split_soft_fail};
    use libc;
    use std::ffi::{c_void, CStr, CString};
    use std::os::raw::c_char;

    #[repr(C)]
    pub struct LIBMTP_mtpdevice_t {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct LIBMTP_folder_t {
        pub folder_id: u32,
        pub parent_id: u32,
        pub storage_id: u32,
        pub name: *mut c_char,
        pub sibling: *mut LIBMTP_folder_t,
        pub child: *mut LIBMTP_folder_t,
    }

    #[repr(C)]
    pub struct LIBMTP_file_t {
        pub item_id: u32,
        pub parent_id: u32,
        pub storage_id: u32,
        pub filename: *mut c_char,
        pub filesize: u64,
        pub modificationdate: i64,
        pub filetype: i32,
        pub next: *mut LIBMTP_file_t,
    }

    #[link(name = "mtp")]
    unsafe extern "C" {
        pub fn LIBMTP_Init();
        pub fn LIBMTP_Get_First_Device() -> *mut LIBMTP_mtpdevice_t;
        pub fn LIBMTP_Release_Device(device: *mut LIBMTP_mtpdevice_t);
        pub fn LIBMTP_Get_Friendlyname(device: *mut LIBMTP_mtpdevice_t) -> *mut c_char;
        pub fn LIBMTP_Get_Folder_List(device: *mut LIBMTP_mtpdevice_t) -> *mut LIBMTP_folder_t;
        pub fn LIBMTP_destroy_folder_t(folder: *mut LIBMTP_folder_t);
        pub fn LIBMTP_new_file_t() -> *mut LIBMTP_file_t;
        pub fn LIBMTP_destroy_file_t(file: *mut LIBMTP_file_t);
        pub fn LIBMTP_Send_File_From_File(
            device: *mut LIBMTP_mtpdevice_t,
            path: *const c_char,
            filedata: *mut LIBMTP_file_t,
            progress: Option<extern "C" fn(u64, u64, *const c_void) -> i32>,
            data: *const c_void,
        ) -> i32;
        pub fn LIBMTP_Get_File_To_File(
            device: *mut LIBMTP_mtpdevice_t,
            item_id: u32,
            path: *const c_char,
            progress: Option<extern "C" fn(u64, u64, *const c_void) -> i32>,
            data: *const c_void,
        ) -> i32;
        pub fn LIBMTP_Get_Filelisting_With_Callback(
            device: *mut LIBMTP_mtpdevice_t,
            callback: Option<extern "C" fn(*mut LIBMTP_file_t, *mut c_void) -> i32>,
            data: *mut c_void,
        ) -> *mut LIBMTP_file_t;
        pub fn strdup(s: *const c_char) -> *mut c_char;
        pub fn free(p: *mut c_void);
    }

    pub struct MtpDevice {
        pub raw: *mut LIBMTP_mtpdevice_t,
    }

    impl Drop for MtpDevice {
        fn drop(&mut self) {
            unsafe {
                if !self.raw.is_null() {
                    LIBMTP_Release_Device(self.raw);
                }
            }
        }
    }

    pub struct MtpFile {
        pub raw: *mut LIBMTP_file_t,
    }

    impl Drop for MtpFile {
        fn drop(&mut self) {
            unsafe {
                if !self.raw.is_null() {
                    LIBMTP_destroy_file_t(self.raw);
                }
            }
        }
    }

    pub struct MtpFileList {
        pub head: *mut LIBMTP_file_t,
    }

    impl Drop for MtpFileList {
        fn drop(&mut self) {
            unsafe {
                let mut curr = self.head;
                while !curr.is_null() {
                    let next = (*curr).next;
                    LIBMTP_destroy_file_t(curr);
                    curr = next;
                }
            }
        }
    }

    struct StreamSilencer {
        source_stream: i32,
        clone_stream: i32,
    }

    impl StreamSilencer {
        fn new(source_stream: i32) -> Option<Self> {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                let clone_stream = libc::dup(source_stream);
                if clone_stream == -1 {
                    return None;
                }
                let path = CString::new("/dev/null").unwrap();
                let null_fd = libc::open(path.as_ptr(), libc::O_WRONLY);
                if null_fd == -1 {
                    libc::close(clone_stream);
                    return None;
                }
                libc::dup2(null_fd, source_stream);
                libc::close(null_fd);
                Some(StreamSilencer {
                    source_stream,
                    clone_stream,
                })
            }
        }
    }

    impl Drop for StreamSilencer {
        fn drop(&mut self) {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.clone_stream, self.source_stream);
                libc::close(self.clone_stream);
            }
        }
    }

    pub fn get_device_friendly_name(device: &MtpDevice) -> String {
        unsafe {
            let name_ptr = LIBMTP_Get_Friendlyname(device.raw);
            if name_ptr.is_null() {
                return "Unknown Device".to_string();
            }
            let c_str = CStr::from_ptr(name_ptr);
            let name = c_str.to_string_lossy().into_owned();
            free(name_ptr as *mut c_void);
            name
        }
    }

    pub fn get_folder_reference(device: &MtpDevice, path: &str) -> Result<u32, String> {
        let folders = unsafe {
            let _silencer = StreamSilencer::new(libc::STDOUT_FILENO);
            LIBMTP_Get_Folder_List(device.raw)
        };
        if folders.is_null() {
            return Err("Could not retrieve folder list".to_string());
        }

        let parts = parent_folder_parts(path);

        let mut current_folder_id = 0;
        let mut current_level_ptr = folders;

        for part in parts {
            let mut found = false;
            let mut sibling = current_level_ptr;

            while !sibling.is_null() {
                unsafe {
                    let folder = &*sibling;
                    let name_cstr = CStr::from_ptr(folder.name);
                    if let Ok(name) = name_cstr.to_str() {
                        if name == part {
                            current_folder_id = folder.folder_id;
                            current_level_ptr = folder.child;
                            found = true;
                            break;
                        }
                    }
                    sibling = folder.sibling;
                }
            }

            if !found {
                unsafe {
                    LIBMTP_destroy_folder_t(folders);
                }
                return Err(format!("Folder '{}' not found", part));
            }
        }

        unsafe {
            LIBMTP_destroy_folder_t(folders);
        }

        Ok(current_folder_id)
    }

    pub fn find_file_in_folder(
        device: &MtpDevice,
        parent_id: u32,
        filename: &str,
    ) -> Result<u32, String> {
        unsafe {
            let files_ptr =
                LIBMTP_Get_Filelisting_With_Callback(device.raw, None, std::ptr::null_mut());
            let _file_list = MtpFileList { head: files_ptr };

            let mut current_file = files_ptr;

            while !current_file.is_null() {
                let file = &*current_file;
                if file.parent_id == parent_id && !file.filename.is_null() {
                    let name_cstr = CStr::from_ptr(file.filename);
                    if let Ok(name) = name_cstr.to_str() {
                        if name == filename {
                            return Ok(file.item_id);
                        }
                    }
                }
                current_file = file.next;
            }

            Err(format!(
                "File '{}' not found in folder ID {}",
                filename, parent_id
            ))
        }
    }

    pub fn upload_file(device: &MtpDevice, src: &str, dst: &str) -> Result<(), String> {
        let (dst, soft_fail) = split_soft_fail(dst);

        let folder_id = get_folder_reference(device, dst)?;

        let filename = std::path::Path::new(dst)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(dst);

        if soft_fail && find_file_in_folder(device, folder_id, filename).is_ok() {
            println!(
                "File \"{}\" already exists on device, skipping upload.",
                dst
            );
            return Ok(());
        }

        let metadata = std::fs::metadata(src)
            .map_err(|e| format!("Error getting metadata for source file '{}': {}", src, e))?;

        let filename_cstr = CString::new(filename).unwrap();
        let src_cstr = CString::new(src).unwrap();

        unsafe {
            let file_t_ptr = LIBMTP_new_file_t();
            if file_t_ptr.is_null() {
                return Err("Error allocating LIBMTP_file_t".to_string());
            }
            let _file_guard = MtpFile { raw: file_t_ptr };

            (*file_t_ptr).item_id = 0;
            (*file_t_ptr).parent_id = folder_id;
            (*file_t_ptr).filename = strdup(filename_cstr.as_ptr());
            (*file_t_ptr).filesize = metadata.len();
            (*file_t_ptr).filetype = 43; // LIBMTP_FILETYPE_UNKNOWN

            let ret = LIBMTP_Send_File_From_File(
                device.raw,
                src_cstr.as_ptr(),
                file_t_ptr,
                None,
                std::ptr::null(),
            );

            if ret != 0 {
                return Err("Error uploading file".to_string());
            }

            println!("Uploaded \"{}\" to \"{}\"", src, dst);
        }
        Ok(())
    }

    pub fn download_file(device: &MtpDevice, src: &str, dst: &str) -> Result<(), String> {
        let (src, soft_fail) = split_soft_fail(src);

        let folder_id = get_folder_reference(device, src)?;

        let src_path = std::path::Path::new(src);
        let filename = src_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let item_id = match find_file_in_folder(device, folder_id, filename) {
            Ok(id) => id,
            Err(e) => {
                if soft_fail {
                    println!("File \"{}\" not found on device, skipping download.", src);
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        };

        let dst_cstr = CString::new(dst).unwrap();

        unsafe {
            let ret = LIBMTP_Get_File_To_File(
                device.raw,
                item_id,
                dst_cstr.as_ptr(),
                None,
                std::ptr::null(),
            );

            if ret != 0 {
                return Err("Error downloading file".to_string());
            }
        }

        println!("Downloaded \"{}\" to \"{}\"", src, dst);
        Ok(())
    }

    pub fn init_and_get_first_device() -> Option<MtpDevice> {
        unsafe {
            let _silencer = StreamSilencer::new(libc::STDERR_FILENO);
            LIBMTP_Init();
            let raw_device = LIBMTP_Get_First_Device();
            if raw_device.is_null() {
                None
            } else {
                Some(MtpDevice { raw: raw_device })
            }
        }
    }
}

#[cfg(any(windows, test))]
#[allow(dead_code, non_snake_case)]
mod wpd {
    use super::{parent_folder_parts, split_soft_fail};
    use std::ffi::c_void;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::ptr;

    type HRESULT = i32;

    const COINIT_MULTITHREADED: u32 = 0x0;
    const CLSCTX_INPROC_SERVER: u32 = 0x1;
    const STGM_READ: u32 = 0x00000000;
    const STGM_WRITE: u32 = 0x00000001;
    const STGM_CREATE: u32 = 0x00001000;
    const VT_LPWSTR: u16 = 31;

    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    struct PROPERTYKEY {
        fmtid: GUID,
        pid: u32,
    }

    #[repr(C)]
    struct PROPVARIANT {
        vt: u16,
        w_reserved1: u16,
        w_reserved2: u16,
        w_reserved3: u16,
        data1: usize,
        data2: usize,
    }

    const CLSID_PORTABLE_DEVICE_MANAGER: GUID = GUID {
        data1: 0x0af10cec,
        data2: 0x2ecd,
        data3: 0x4b92,
        data4: [0x95, 0x81, 0x34, 0xf6, 0xae, 0x06, 0x37, 0xf3],
    };
    const IID_IPORTABLE_DEVICE_MANAGER: GUID = GUID {
        data1: 0xa1567595,
        data2: 0x4c2f,
        data3: 0x4574,
        data4: [0xa6, 0xfa, 0xec, 0xef, 0x91, 0x7b, 0x9a, 0x40],
    };

    const CLSID_PORTABLE_DEVICE_FTM: GUID = GUID {
        data1: 0xf7c0039a,
        data2: 0x4762,
        data3: 0x488a,
        data4: [0xb4, 0xb3, 0x76, 0x0e, 0xf9, 0xa1, 0xba, 0x9b],
    };
    const CLSID_PORTABLE_DEVICE: GUID = GUID {
        data1: 0x728a21c5,
        data2: 0x3d9e,
        data3: 0x48d7,
        data4: [0x98, 0x10, 0x86, 0x48, 0x48, 0xf0, 0xf4, 0x04],
    };
    const IID_IPORTABLE_DEVICE: GUID = GUID {
        data1: 0x625e2df8,
        data2: 0x6392,
        data3: 0x4cf0,
        data4: [0x9a, 0xd1, 0x3c, 0xfa, 0x5f, 0x17, 0x77, 0x5c],
    };

    const CLSID_PORTABLE_DEVICE_VALUES: GUID = GUID {
        data1: 0x0c15d503,
        data2: 0xd017,
        data3: 0x47ce,
        data4: [0x90, 0x16, 0x7b, 0x3f, 0x97, 0x87, 0x21, 0xcc],
    };
    const IID_IPORTABLE_DEVICE_VALUES: GUID = GUID {
        data1: 0x6848f6f2,
        data2: 0x3155,
        data3: 0x4f86,
        data4: [0xb6, 0xf5, 0x26, 0x3e, 0xee, 0xab, 0x31, 0x43],
    };

    const CLSID_PORTABLE_DEVICE_KEY_COLLECTION: GUID = GUID {
        data1: 0xde2d022d,
        data2: 0x2480,
        data3: 0x43be,
        data4: [0x97, 0xf0, 0xd1, 0xfa, 0x2c, 0xf9, 0x8f, 0x4f],
    };
    const IID_IPORTABLE_DEVICE_KEY_COLLECTION: GUID = GUID {
        data1: 0xdada2357,
        data2: 0xe0ad,
        data3: 0x492e,
        data4: [0x98, 0xdb, 0xdd, 0x61, 0xc5, 0x3b, 0xa3, 0x53],
    };

    const CLSID_PORTABLE_DEVICE_PROPVARIANT_COLLECTION: GUID = GUID {
        data1: 0x08a99e2f,
        data2: 0x6d6d,
        data3: 0x4b80,
        data4: [0xaf, 0x5a, 0xba, 0xf2, 0xbc, 0xbe, 0x4c, 0xb9],
    };
    const IID_IPORTABLE_DEVICE_PROPVARIANT_COLLECTION: GUID = GUID {
        data1: 0x89b2e422,
        data2: 0x4f1b,
        data3: 0x4316,
        data4: [0xbc, 0xef, 0xa4, 0x4a, 0xfe, 0xa8, 0x3e, 0xb3],
    };

    const WPD_OBJECT_FMTID: GUID = GUID {
        data1: 0xef6b490d,
        data2: 0x5cd8,
        data3: 0x437a,
        data4: [0xaf, 0xfc, 0xda, 0x8b, 0x60, 0xee, 0x4a, 0x3c],
    };
    const WPD_OBJECT_PARENT_ID: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_OBJECT_FMTID,
        pid: 3,
    };
    const WPD_OBJECT_NAME: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_OBJECT_FMTID,
        pid: 4,
    };
    const WPD_OBJECT_SIZE: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_OBJECT_FMTID,
        pid: 11,
    };
    const WPD_OBJECT_ORIGINAL_FILE_NAME: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_OBJECT_FMTID,
        pid: 12,
    };

    const WPD_CLIENT_FMTID: GUID = GUID {
        data1: 0x204d9f0c,
        data2: 0x2292,
        data3: 0x4080,
        data4: [0x9f, 0x42, 0x40, 0x66, 0x4e, 0x70, 0xf8, 0x59],
    };
    const WPD_CLIENT_NAME: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_CLIENT_FMTID,
        pid: 2,
    };
    const WPD_CLIENT_MAJOR_VERSION: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_CLIENT_FMTID,
        pid: 3,
    };
    const WPD_CLIENT_MINOR_VERSION: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_CLIENT_FMTID,
        pid: 4,
    };
    const WPD_CLIENT_DESIRED_ACCESS: PROPERTYKEY = PROPERTYKEY {
        fmtid: WPD_CLIENT_FMTID,
        pid: 9,
    };

    const WPD_RESOURCE_DEFAULT: PROPERTYKEY = PROPERTYKEY {
        fmtid: GUID {
            data1: 0xe81e79be,
            data2: 0x34f0,
            data3: 0x41bf,
            data4: [0xb5, 0x3f, 0xf1, 0xa0, 0x6a, 0xe8, 0x78, 0x42],
        },
        pid: 0,
    };

    const GENERIC_READ: u32 = 0x80000000;
    const GENERIC_WRITE: u32 = 0x40000000;

    #[cfg(windows)]
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(pvReserved: *mut c_void, dwCoInit: u32) -> HRESULT;
        fn CoCreateInstance(
            rclsid: *const GUID,
            pUnkOuter: *mut c_void,
            dwClsContext: u32,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> HRESULT;
        fn CoTaskMemFree(pv: *mut c_void);
    }

    #[cfg(not(windows))]
    unsafe fn CoInitializeEx(_pv_reserved: *mut c_void, _dw_co_init: u32) -> HRESULT {
        0
    }
    #[cfg(not(windows))]
    unsafe fn CoCreateInstance(
        _rclsid: *const GUID,
        _p_unk_outer: *mut c_void,
        _dw_cls_context: u32,
        _riid: *const GUID,
        _ppv: *mut *mut c_void,
    ) -> HRESULT {
        -1
    }
    #[cfg(not(windows))]
    unsafe fn CoTaskMemFree(_pv: *mut c_void) {}

    #[repr(C)]
    struct ComObject {
        vtbl: *const *const c_void,
    }

    struct ComPtr(*mut ComObject);

    impl ComPtr {
        unsafe fn from_raw(ptr: *mut c_void) -> Option<Self> {
            if ptr.is_null() {
                None
            } else {
                Some(Self(ptr as *mut ComObject))
            }
        }

        fn as_raw(&self) -> *mut c_void {
            self.0 as *mut c_void
        }

        unsafe fn method(&self, index: usize) -> *const c_void {
            *((*self.0).vtbl.add(index))
        }
    }

    impl Drop for ComPtr {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    let release: unsafe extern "system" fn(*mut c_void) -> u32 =
                        std::mem::transmute(self.method(2));
                    release(self.as_raw());
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn from_wide_ptr(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
    }

    unsafe fn create_instance(clsid: &GUID, iid: &GUID) -> Result<ComPtr, String> {
        let mut raw: *mut c_void = ptr::null_mut();
        let hr = CoCreateInstance(clsid, ptr::null_mut(), CLSCTX_INPROC_SERVER, iid, &mut raw);
        if hr < 0 || raw.is_null() {
            return Err(format!("CoCreateInstance failed with HRESULT 0x{:08X}", hr));
        }
        Ok(ComPtr(raw as *mut ComObject))
    }

    unsafe fn create_client_info(read_write: bool) -> Result<ComPtr, String> {
        let vals = create_instance(&CLSID_PORTABLE_DEVICE_VALUES, &IID_IPORTABLE_DEVICE_VALUES)?;
        let set_str: unsafe extern "system" fn(*mut c_void, *const PROPERTYKEY, *const u16) -> HRESULT =
            std::mem::transmute(vals.method(7));
        let set_u32: unsafe extern "system" fn(*mut c_void, *const PROPERTYKEY, u32) -> HRESULT =
            std::mem::transmute(vals.method(9));

        let name = to_wide("rules_ciq");
        let _ = set_str(vals.as_raw(), &WPD_CLIENT_NAME, name.as_ptr());
        let _ = set_u32(vals.as_raw(), &WPD_CLIENT_MAJOR_VERSION, 1);
        let _ = set_u32(vals.as_raw(), &WPD_CLIENT_MINOR_VERSION, 0);
        let access = if read_write {
            GENERIC_READ | GENERIC_WRITE
        } else {
            GENERIC_READ
        };
        let _ = set_u32(vals.as_raw(), &WPD_CLIENT_DESIRED_ACCESS, access);
        Ok(vals)
    }

    pub struct MtpDevice {
        device: ComPtr,
        friendly_name: String,
    }

    pub fn get_device_friendly_name(device: &MtpDevice) -> String {
        device.friendly_name.clone()
    }

    pub fn init_and_get_first_device() -> Option<MtpDevice> {
        unsafe {
            let _ = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
            let mgr = create_instance(
                &CLSID_PORTABLE_DEVICE_MANAGER,
                &IID_IPORTABLE_DEVICE_MANAGER,
            )
            .ok()?;

            // 3: GetDevices, 4: RefreshDeviceList, 5: GetDeviceFriendlyName
            let refresh: unsafe extern "system" fn(*mut c_void) -> HRESULT =
                std::mem::transmute(mgr.method(4));
            let _ = refresh(mgr.as_raw());

            let get_devices: unsafe extern "system" fn(
                *mut c_void,
                *mut *mut u16,
                *mut u32,
            ) -> HRESULT = std::mem::transmute(mgr.method(3));

            let mut count: u32 = 0;
            if get_devices(mgr.as_raw(), ptr::null_mut(), &mut count) < 0 || count == 0 {
                return None;
            }

            let mut ids: Vec<*mut u16> = vec![ptr::null_mut(); count as usize];
            if get_devices(mgr.as_raw(), ids.as_mut_ptr(), &mut count) < 0 || count == 0 {
                return None;
            }

            let pnp_id = ids[0];
            for &id in &ids[1..count as usize] {
                if !id.is_null() {
                    CoTaskMemFree(id as *mut c_void);
                }
            }
            if pnp_id.is_null() {
                return None;
            }

            let get_friendly: unsafe extern "system" fn(
                *mut c_void,
                *const u16,
                *mut u16,
                *mut u32,
            ) -> HRESULT = std::mem::transmute(mgr.method(5));

            let mut name_len: u32 = 0;
            let _ = get_friendly(mgr.as_raw(), pnp_id, ptr::null_mut(), &mut name_len);
            let friendly_name = if name_len > 0 {
                let mut buf: Vec<u16> = vec![0; name_len as usize];
                if get_friendly(mgr.as_raw(), pnp_id, buf.as_mut_ptr(), &mut name_len) >= 0 {
                    from_wide_ptr(buf.as_ptr())
                } else {
                    "Unknown Device".to_string()
                }
            } else {
                "Unknown Device".to_string()
            };

            let dev = create_instance(&CLSID_PORTABLE_DEVICE_FTM, &IID_IPORTABLE_DEVICE)
                .or_else(|_| create_instance(&CLSID_PORTABLE_DEVICE, &IID_IPORTABLE_DEVICE))
                .ok();
            let Some(dev) = dev else {
                CoTaskMemFree(pnp_id as *mut c_void);
                return None;
            };

            // 3: IPortableDevice::Open
            let open_fn: unsafe extern "system" fn(
                *mut c_void,
                *const u16,
                *mut c_void,
            ) -> HRESULT = std::mem::transmute(dev.method(3));

            let client_rw = create_client_info(true).ok()?;
            let mut hr = open_fn(dev.as_raw(), pnp_id, client_rw.as_raw());
            if hr < 0 {
                if let Ok(client_ro) = create_client_info(false) {
                    hr = open_fn(dev.as_raw(), pnp_id, client_ro.as_raw());
                }
            }
            CoTaskMemFree(pnp_id as *mut c_void);

            if hr < 0 {
                return None;
            }

            Some(MtpDevice {
                device: dev,
                friendly_name,
            })
        }
    }

    unsafe fn device_content(device: &MtpDevice) -> Result<ComPtr, String> {
        // 5: IPortableDevice::Content
        let content_fn: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT =
            std::mem::transmute(device.device.method(5));
        let mut raw: *mut c_void = ptr::null_mut();
        let hr = content_fn(device.device.as_raw(), &mut raw);
        if hr < 0 {
            return Err(format!("IPortableDevice::Content failed (0x{:08X})", hr));
        }
        ComPtr::from_raw(raw).ok_or_else(|| "Null IPortableDeviceContent".to_string())
    }

    unsafe fn content_properties(content: &ComPtr) -> Result<ComPtr, String> {
        // 4: IPortableDeviceContent::Properties
        let props_fn: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT =
            std::mem::transmute(content.method(4));
        let mut raw: *mut c_void = ptr::null_mut();
        let hr = props_fn(content.as_raw(), &mut raw);
        if hr < 0 {
            return Err(format!("IPortableDeviceContent::Properties failed (0x{:08X})", hr));
        }
        ComPtr::from_raw(raw).ok_or_else(|| "Null IPortableDeviceProperties".to_string())
    }

    unsafe fn list_child_ids(content: &ComPtr, parent_id: &str) -> Result<Vec<String>, String> {
        // 3: IPortableDeviceContent::EnumObjects
        let enum_fn: unsafe extern "system" fn(
            *mut c_void,
            u32,
            *const u16,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(content.method(3));

        let parent_w = to_wide(parent_id);
        let mut enum_raw: *mut c_void = ptr::null_mut();
        let hr = enum_fn(
            content.as_raw(),
            0,
            parent_w.as_ptr(),
            ptr::null_mut(),
            &mut enum_raw,
        );
        if hr < 0 {
            return Err(format!("EnumObjects failed for '{}' (0x{:08X})", parent_id, hr));
        }
        let Some(enum_ids) = ComPtr::from_raw(enum_raw) else {
            return Ok(Vec::new());
        };

        // 3: IEnumPortableDeviceObjectIDs::Next
        let next_fn: unsafe extern "system" fn(
            *mut c_void,
            u32,
            *mut *mut u16,
            *mut u32,
        ) -> HRESULT = std::mem::transmute(enum_ids.method(3));

        let mut out = Vec::new();
        loop {
            let mut batch: [*mut u16; 16] = [ptr::null_mut(); 16];
            let mut fetched: u32 = 0;
            let hr = next_fn(enum_ids.as_raw(), 16, batch.as_mut_ptr(), &mut fetched);
            if hr < 0 || fetched == 0 {
                break;
            }
            for &ptr in &batch[..fetched as usize] {
                if !ptr.is_null() {
                    out.push(from_wide_ptr(ptr));
                    CoTaskMemFree(ptr as *mut c_void);
                }
            }
        }
        Ok(out)
    }

    unsafe fn object_display_names(props: &ComPtr, object_id: &str) -> (String, String) {
        let Ok(keys) = create_instance(
            &CLSID_PORTABLE_DEVICE_KEY_COLLECTION,
            &IID_IPORTABLE_DEVICE_KEY_COLLECTION,
        ) else {
            return (String::new(), String::new());
        };
        // 5: IPortableDeviceKeyCollection::Add
        let add_key: unsafe extern "system" fn(*mut c_void, *const PROPERTYKEY) -> HRESULT =
            std::mem::transmute(keys.method(5));
        let _ = add_key(keys.as_raw(), &WPD_OBJECT_NAME);
        let _ = add_key(keys.as_raw(), &WPD_OBJECT_ORIGINAL_FILE_NAME);

        // 5: IPortableDeviceProperties::GetValues
        let get_values: unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(props.method(5));

        let obj_w = to_wide(object_id);
        let mut vals_raw: *mut c_void = ptr::null_mut();
        if get_values(props.as_raw(), obj_w.as_ptr(), keys.as_raw(), &mut vals_raw) < 0 {
            return (String::new(), String::new());
        }
        let Some(vals) = ComPtr::from_raw(vals_raw) else {
            return (String::new(), String::new());
        };

        // 8: IPortableDeviceValues::GetStringValue
        let get_str: unsafe extern "system" fn(
            *mut c_void,
            *const PROPERTYKEY,
            *mut *mut u16,
        ) -> HRESULT = std::mem::transmute(vals.method(8));

        let mut name_ptr: *mut u16 = ptr::null_mut();
        let name = if get_str(vals.as_raw(), &WPD_OBJECT_NAME, &mut name_ptr) >= 0 && !name_ptr.is_null() {
            let s = from_wide_ptr(name_ptr);
            CoTaskMemFree(name_ptr as *mut c_void);
            s
        } else {
            String::new()
        };

        let mut orig_ptr: *mut u16 = ptr::null_mut();
        let orig = if get_str(vals.as_raw(), &WPD_OBJECT_ORIGINAL_FILE_NAME, &mut orig_ptr) >= 0
            && !orig_ptr.is_null()
        {
            let s = from_wide_ptr(orig_ptr);
            CoTaskMemFree(orig_ptr as *mut c_void);
            s
        } else {
            String::new()
        };

        (name, orig)
    }

    unsafe fn find_child_by_name(
        content: &ComPtr,
        props: &ComPtr,
        parent_id: &str,
        target_name: &str,
    ) -> Result<Option<String>, String> {
        for child_id in list_child_ids(content, parent_id)? {
            let (name, orig) = object_display_names(props, &child_id);
            if name.eq_ignore_ascii_case(target_name) || orig.eq_ignore_ascii_case(target_name) {
                return Ok(Some(child_id));
            }
        }
        Ok(None)
    }

    unsafe fn resolve_folder_id(
        content: &ComPtr,
        props: &ComPtr,
        path: &str,
    ) -> Result<String, String> {
        let parts = parent_folder_parts(path);
        if parts.is_empty() {
            let root_children = list_child_ids(content, "DEVICE")?;
            return Ok(root_children
                .into_iter()
                .next()
                .unwrap_or_else(|| "DEVICE".to_string()));
        }

        // First segment (e.g. "GARMIN"): check directly under "DEVICE", then under each storage object.
        let first = parts[0];
        let mut current_id = if let Some(id) = find_child_by_name(content, props, "DEVICE", first)? {
            id
        } else {
            let mut found_in_storage = None;
            for storage_id in list_child_ids(content, "DEVICE")? {
                if let Some(id) = find_child_by_name(content, props, &storage_id, first)? {
                    found_in_storage = Some(id);
                    break;
                }
            }
            found_in_storage.ok_or_else(|| format!("Folder '{}' not found", first))?
        };

        for &part in &parts[1..] {
            current_id = find_child_by_name(content, props, &current_id, part)?
                .ok_or_else(|| format!("Folder '{}' not found", part))?;
        }

        Ok(current_id)
    }

    unsafe fn delete_object(content: &ComPtr, object_id: &str) -> Result<(), String> {
        let coll = create_instance(
            &CLSID_PORTABLE_DEVICE_PROPVARIANT_COLLECTION,
            &IID_IPORTABLE_DEVICE_PROPVARIANT_COLLECTION,
        )?;
        // 5: IPortableDevicePropVariantCollection::Add
        let add_fn: unsafe extern "system" fn(*mut c_void, *const PROPVARIANT) -> HRESULT =
            std::mem::transmute(coll.method(5));

        let wide = to_wide(object_id);
        let pv = PROPVARIANT {
            vt: VT_LPWSTR,
            w_reserved1: 0,
            w_reserved2: 0,
            w_reserved3: 0,
            data1: wide.as_ptr() as usize,
            data2: 0,
        };
        let hr = add_fn(coll.as_raw(), &pv);
        if hr < 0 {
            return Err(format!("Failed to add object ID for deletion (0x{:08X})", hr));
        }

        // 8: IPortableDeviceContent::Delete
        let delete_fn: unsafe extern "system" fn(
            *mut c_void,
            u32,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(content.method(8));

        let hr = delete_fn(content.as_raw(), 0, coll.as_raw(), ptr::null_mut());
        if hr < 0 {
            return Err(format!("Failed to delete existing object '{}' (0x{:08X})", object_id, hr));
        }
        Ok(())
    }

    pub fn upload_file(device: &MtpDevice, src: &str, dst: &str) -> Result<(), String> {
        let (dst, soft_fail) = split_soft_fail(dst);
        let filename = std::path::Path::new(dst)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(dst);

        let mut src_file =
            File::open(src).map_err(|e| format!("Error opening source file '{}': {}", src, e))?;
        let file_size = src_file
            .metadata()
            .map_err(|e| format!("Error getting metadata for source file '{}': {}", src, e))?
            .len();

        unsafe {
            let content = device_content(device)?;
            let props = content_properties(&content)?;
            let folder_id = resolve_folder_id(&content, &props, dst)?;

            if let Some(existing_id) = find_child_by_name(&content, &props, &folder_id, filename)? {
                if soft_fail {
                    println!(
                        "File \"{}\" already exists on device, skipping upload.",
                        dst
                    );
                    return Ok(());
                }
                let _ = delete_object(&content, &existing_id);
            }

            let vals = create_instance(&CLSID_PORTABLE_DEVICE_VALUES, &IID_IPORTABLE_DEVICE_VALUES)?;
            let set_str: unsafe extern "system" fn(
                *mut c_void,
                *const PROPERTYKEY,
                *const u16,
            ) -> HRESULT = std::mem::transmute(vals.method(7));
            let set_u64: unsafe extern "system" fn(
                *mut c_void,
                *const PROPERTYKEY,
                u64,
            ) -> HRESULT = std::mem::transmute(vals.method(13));

            let parent_w = to_wide(&folder_id);
            let name_w = to_wide(filename);
            let _ = set_str(vals.as_raw(), &WPD_OBJECT_PARENT_ID, parent_w.as_ptr());
            let _ = set_str(vals.as_raw(), &WPD_OBJECT_NAME, name_w.as_ptr());
            let _ = set_str(vals.as_raw(), &WPD_OBJECT_ORIGINAL_FILE_NAME, name_w.as_ptr());
            let _ = set_u64(vals.as_raw(), &WPD_OBJECT_SIZE, file_size);

            // 7: IPortableDeviceContent::CreateObjectWithPropertiesAndData
            let create_fn: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                *mut *mut c_void,
                *mut u32,
                *mut *mut u16,
            ) -> HRESULT = std::mem::transmute(content.method(7));

            let mut stream_raw: *mut c_void = ptr::null_mut();
            let mut opt_buf_size: u32 = 65536;
            let hr = create_fn(
                content.as_raw(),
                vals.as_raw(),
                &mut stream_raw,
                &mut opt_buf_size,
                ptr::null_mut(),
            );
            if hr < 0 || stream_raw.is_null() {
                return Err(format!(
                    "CreateObjectWithPropertiesAndData failed (0x{:08X})",
                    hr
                ));
            }
            let stream = ComPtr(stream_raw as *mut ComObject);

            // 4: ISequentialStream::Write, 8: IStream::Commit
            let write_fn: unsafe extern "system" fn(
                *mut c_void,
                *const c_void,
                u32,
                *mut u32,
            ) -> HRESULT = std::mem::transmute(stream.method(4));
            let commit_fn: unsafe extern "system" fn(*mut c_void, u32) -> HRESULT =
                std::mem::transmute(stream.method(8));

            let buf_len = (opt_buf_size as usize).max(4096);
            let mut buf = vec![0u8; buf_len];
            loop {
                let n = src_file
                    .read(&mut buf)
                    .map_err(|e| format!("Failed reading '{}': {}", src, e))?;
                if n == 0 {
                    break;
                }
                let mut offset = 0;
                while offset < n {
                    let mut written: u32 = 0;
                    let hr = write_fn(
                        stream.as_raw(),
                        buf[offset..n].as_ptr() as *const c_void,
                        (n - offset) as u32,
                        &mut written,
                    );
                    if hr < 0 {
                        return Err(format!("IStream::Write failed (0x{:08X})", hr));
                    }
                    if written == 0 {
                        break;
                    }
                    offset += written as usize;
                }
            }

            let hr = commit_fn(stream.as_raw(), 0);
            if hr < 0 {
                return Err(format!("IStream::Commit failed (0x{:08X})", hr));
            }

            println!("Uploaded \"{}\" to \"{}\"", src, dst);
            Ok(())
        }
    }

    pub fn download_file(device: &MtpDevice, src: &str, dst: &str) -> Result<(), String> {
        let (src, soft_fail) = split_soft_fail(src);
        let filename = std::path::Path::new(src)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        unsafe {
            let content = device_content(device)?;
            let props = content_properties(&content)?;
            let folder_id = resolve_folder_id(&content, &props, src)?;

            let Some(item_id) = find_child_by_name(&content, &props, &folder_id, filename)? else {
                if soft_fail {
                    println!("File \"{}\" not found on device, skipping download.", src);
                    return Ok(());
                }
                return Err(format!("File '{}' not found in folder '{}'", filename, folder_id));
            };

            // 5: IPortableDeviceContent::Transfer
            let transfer_fn: unsafe extern "system" fn(
                *mut c_void,
                *mut *mut c_void,
            ) -> HRESULT = std::mem::transmute(content.method(5));
            let mut res_raw: *mut c_void = ptr::null_mut();
            let hr = transfer_fn(content.as_raw(), &mut res_raw);
            if hr < 0 || res_raw.is_null() {
                return Err(format!("IPortableDeviceContent::Transfer failed (0x{:08X})", hr));
            }
            let resources = ComPtr(res_raw as *mut ComObject);

            // 5: IPortableDeviceResources::GetStream
            let get_stream_fn: unsafe extern "system" fn(
                *mut c_void,
                *const u16,
                *const PROPERTYKEY,
                u32,
                *mut u32,
                *mut *mut c_void,
            ) -> HRESULT = std::mem::transmute(resources.method(5));

            let item_w = to_wide(&item_id);
            let mut opt_buf_size: u32 = 65536;
            let mut stream_raw: *mut c_void = ptr::null_mut();
            let hr = get_stream_fn(
                resources.as_raw(),
                item_w.as_ptr(),
                &WPD_RESOURCE_DEFAULT,
                STGM_READ,
                &mut opt_buf_size,
                &mut stream_raw,
            );
            if hr < 0 || stream_raw.is_null() {
                return Err(format!("IPortableDeviceResources::GetStream failed (0x{:08X})", hr));
            }
            let stream = ComPtr(stream_raw as *mut ComObject);

            // 3: ISequentialStream::Read
            let read_fn: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                u32,
                *mut u32,
            ) -> HRESULT = std::mem::transmute(stream.method(3));

            let mut dst_file = File::create(dst)
                .map_err(|e| format!("Failed to create destination file '{}': {}", dst, e))?;
            let buf_len = (opt_buf_size as usize).max(4096);
            let mut buf = vec![0u8; buf_len];

            loop {
                let mut read_bytes: u32 = 0;
                let hr = read_fn(
                    stream.as_raw(),
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len() as u32,
                    &mut read_bytes,
                );
                if hr < 0 {
                    return Err(format!("IStream::Read failed (0x{:08X})", hr));
                }
                if read_bytes == 0 {
                    break;
                }
                dst_file
                    .write_all(&buf[..read_bytes as usize])
                    .map_err(|e| format!("Failed writing to '{}': {}", dst, e))?;
            }

            let _ = (STGM_WRITE, STGM_CREATE);
            println!("Downloaded \"{}\" to \"{}\"", src, dst);
            Ok(())
        }
    }
    pub(super) fn test_wide_roundtrip(s: &str) -> String {
        let wide = to_wide(s);
        unsafe { from_wide_ptr(wide.as_ptr()) }
    }
}

#[cfg(windows)]
use wpd as imp;

pub use imp::{
    download_file, get_device_friendly_name, init_and_get_first_device, upload_file, MtpDevice,
};

pub fn run_mtp_operation<F>(operation: F)
where
    F: Fn(&MtpDevice, &str, &str) -> Result<(), String>,
{
    let cli = Cli::parse();

    if cli.files.len() % 2 != 0 {
        eprintln!("Error: Files must be provided in pairs of <src> <dst>");
        std::process::exit(1);
    }

    let start_time = std::time::Instant::now();
    let wait_duration = std::time::Duration::from_secs(cli.wait_for);
    let mut first_attempt = true;

    let device = loop {
        match init_and_get_first_device() {
            Some(d) => break d,
            None => {
                if start_time.elapsed() >= wait_duration {
                    eprintln!("No MTP device found, or device already claimed.");
                    std::process::exit(1);
                }

                if first_attempt {
                    eprintln!("No MTP device found, retrying...");
                    first_attempt = false;
                }

                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    };

    println!("Connected to {}", get_device_friendly_name(&device));

    let mut exit_code = 0;

    for chunk in cli.files.chunks(2) {
        let src = &chunk[0];
        let dst = &chunk[1];

        if let Err(e) = operation(&device, src, dst) {
            eprintln!("{}", e);
            exit_code = 1;
        }
    }

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_soft_fail() {
        assert_eq!(
            split_soft_fail("?/GARMIN/Apps/LOGS/app.TXT"),
            ("/GARMIN/Apps/LOGS/app.TXT", true)
        );
        assert_eq!(
            split_soft_fail("/GARMIN/Apps/app.prg"),
            ("/GARMIN/Apps/app.prg", false)
        );
    }

    #[test]
    fn test_parent_folder_parts() {
        assert_eq!(
            parent_folder_parts("/GARMIN/Apps/LOGS/CIQ_LOG.YML"),
            vec!["GARMIN", "Apps", "LOGS"]
        );
        assert_eq!(
            parent_folder_parts("/GARMIN/Apps/my_app.prg"),
            vec!["GARMIN", "Apps"]
        );
        assert!(parent_folder_parts("/file_at_root.txt").is_empty());
    }

    #[test]
    fn test_wpd_wide_string_roundtrip() {
        assert_eq!(wpd::test_wide_roundtrip("GARMIN/Apps/my_app.prg"), "GARMIN/Apps/my_app.prg");
        assert_eq!(wpd::test_wide_roundtrip(""), "");
    }
}
