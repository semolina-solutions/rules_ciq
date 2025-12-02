//! MTP library for Garmin devices.
//!
//! This library provides a Rust interface to `libmtp` for interacting with MTP devices.
//! It supports listing files, uploading files, and downloading files.

use libc;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
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

    let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if !parts.is_empty() {
        parts.pop();
    }

    let mut current_folder_id = 0; // Root folder ID.
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
            if file.parent_id == parent_id {
                if !file.filename.is_null() {
                    let name_cstr = CStr::from_ptr(file.filename);
                    if let Ok(name) = name_cstr.to_str() {
                        if name == filename {
                            return Ok(file.item_id);
                        }
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
    let (dst, soft_fail) = if dst.starts_with('?') {
        (&dst[1..], true)
    } else {
        (dst, false)
    };

    let folder_id = get_folder_reference(device, dst)?;

    let filename = std::path::Path::new(dst)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(dst);

    if soft_fail {
        if let Ok(_) = find_file_in_folder(device, folder_id, filename) {
            println!(
                "File \"{}\" already exists on device, skipping upload.",
                dst
            );
            return Ok(());
        }
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
    let (src, soft_fail) = if src.starts_with('?') {
        (&src[1..], true)
    } else {
        (src, false)
    };

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

pub fn run_mtp_operation<F>(operation: F)
where
    F: Fn(&MtpDevice, &str, &str) -> Result<(), String>,
{
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 || (args.len() - 1) % 2 != 0 {
        eprintln!("Usage: {} <src> <dst> [<src> <dst> ...]", args[0]);
        std::process::exit(1);
    }

    let device = match init_and_get_first_device() {
        Some(d) => d,
        None => {
            eprintln!("No MTP device found, or device already claimed.");
            std::process::exit(1);
        }
    };

    println!("Connected to {}", get_device_friendly_name(&device));

    let user_args = &args[1..];
    let mut exit_code = 0;

    for chunk in user_args.chunks(2) {
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
