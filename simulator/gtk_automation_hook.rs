//! LD_PRELOAD GTK3 automation hook for the Linux Garmin Connect IQ Simulator.
//!
//! Responsibilities:
//! 1. Intercepts `GtkFileChooserDialog` response emission blocking and filename
//!    retrieval so `/_File/gtk-save` ("Save Screen Capture") saves headlessly
//!    to a target path without hanging on a modal file dialog.
//! 2. Polls for `/work/trigger` and executes the line-oriented automation script
//!    in `/work/hook_cmds.txt` on the GTK main thread:
//!      - `MENU <menu_path>`
//!      - `DIALOG_BUTTON <button_label>`
//!      - `LOAD_FIT <fit_path>`
//!      - `CLICK <x> <y> <hold_ms>`
//!      - `SWIPE <x1> <y1> <x2> <y2> <duration_ms>`
//!      - `WAIT <ms>`
//!      - `SAVE <output_path>`

use std::collections::VecDeque;
use std::ffi::{c_char, c_double, c_int, c_short, c_uint, c_void, CStr, CString};
use std::fs;
use std::path::Path;
use std::ptr;
use std::sync::{Mutex, OnceLock};

const RTLD_NEXT: *mut c_void = -1isize as *mut c_void;

const G_SOURCE_REMOVE: c_int = 0;
const G_SOURCE_CONTINUE: c_int = 1;

const GTK_RESPONSE_ACCEPT: c_int = -3;

const GDK_MOTION_NOTIFY: c_int = 3;
const GDK_BUTTON_PRESS: c_int = 4;
const GDK_BUTTON_RELEASE: c_int = 7;
const GDK_BUTTON1_MASK: c_uint = 1 << 8;

#[repr(C)]
struct GList {
    data: *mut c_void,
    next: *mut GList,
    prev: *mut GList,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct GtkAllocation {
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
}

#[repr(C)]
struct GdkEventButton {
    type_: c_int,
    window: *mut c_void,
    send_event: i8,
    time: u32,
    x: c_double,
    y: c_double,
    axes: *mut c_double,
    state: c_uint,
    button: c_uint,
    device: *mut c_void,
    x_root: c_double,
    y_root: c_double,
}

#[repr(C)]
struct GdkEventMotion {
    type_: c_int,
    window: *mut c_void,
    send_event: i8,
    time: u32,
    x: c_double,
    y: c_double,
    axes: *mut c_double,
    state: c_uint,
    is_hint: c_short,
    device: *mut c_void,
    x_root: c_double,
    y_root: c_double,
}

const _: [(); 80] = [(); std::mem::size_of::<GdkEventButton>()];
const _: [(); 80] = [(); std::mem::size_of::<GdkEventMotion>()];

type GSourceFunc = unsafe extern "C" fn(*mut c_void) -> c_int;

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "dl"))]
extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "c"))]
extern "C" {
    fn _exit(status: c_int) -> !;
}

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "glib-2.0"))]
extern "C" {
    fn g_timeout_add(interval: c_uint, function: GSourceFunc, data: *mut c_void) -> c_uint;
    fn g_strdup(str: *const c_char) -> *mut c_char;
    fn g_get_monotonic_time() -> i64;
    fn g_list_free(list: *mut GList);
}

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "gobject-2.0"))]
extern "C" {
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_type_check_instance_is_a(instance: *mut c_void, iface_type: usize) -> c_int;
    fn g_type_name_from_instance(instance: *mut c_void) -> *const c_char;
}

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "gdk-3"))]
extern "C" {
    fn gdk_window_get_children(window: *mut c_void) -> *mut GList;
    fn gdk_window_get_display(window: *mut c_void) -> *mut c_void;
    fn gdk_window_get_screen(window: *mut c_void) -> *mut c_void;
    fn gdk_window_get_origin(window: *mut c_void, x: *mut c_int, y: *mut c_int) -> c_int;
    fn gdk_display_get_default_seat(display: *mut c_void) -> *mut c_void;
    fn gdk_seat_get_pointer(seat: *mut c_void) -> *mut c_void;
    fn gdk_device_warp(device: *mut c_void, screen: *mut c_void, x: c_int, y: c_int);
    fn gdk_event_new(type_: c_int) -> *mut c_void;
    fn gdk_event_set_device(event: *mut c_void, device: *mut c_void);
    fn gdk_event_free(event: *mut c_void);
}

#[cfg(not(test))]
#[cfg_attr(target_os = "linux", link(name = "gtk-3"))]
extern "C" {
    fn gtk_file_chooser_get_type() -> usize;
    fn gtk_dialog_get_type() -> usize;
    fn gtk_menu_item_get_type() -> usize;
    fn gtk_container_get_type() -> usize;
    fn gtk_button_get_type() -> usize;
    fn gtk_statusbar_get_type() -> usize;
    fn gtk_menu_bar_get_type() -> usize;
    fn gtk_combo_box_get_type() -> usize;

    fn gtk_dialog_response(dialog: *mut c_void, response_id: c_int);
    fn gtk_window_list_toplevels() -> *mut GList;
    fn gtk_widget_get_visible(widget: *mut c_void) -> c_int;
    fn gtk_widget_get_allocation(widget: *mut c_void, allocation: *mut GtkAllocation);
    fn gtk_widget_get_window(widget: *mut c_void) -> *mut c_void;
    fn gtk_menu_item_get_label(menu_item: *mut c_void) -> *const c_char;
    fn gtk_menu_item_get_submenu(menu_item: *mut c_void) -> *mut c_void;
    fn gtk_menu_item_activate(menu_item: *mut c_void);
    fn gtk_container_get_children(container: *mut c_void) -> *mut GList;
    fn gtk_button_get_label(button: *mut c_void) -> *const c_char;
    fn gtk_button_clicked(button: *mut c_void);
    fn gtk_combo_box_set_active(combo_box: *mut c_void, index_: c_int);
    fn gtk_main_do_event(event: *mut c_void);
}

struct HookState {
    cmds: VecDeque<String>,
    triggered: bool,
    accepted: bool,
    save_target_path: String,
    exit_after_file_chooser: bool,
}

fn state() -> &'static Mutex<HookState> {
    static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(HookState {
            cmds: VecDeque::new(),
            triggered: false,
            accepted: false,
            save_target_path: "/work/output.png".to_string(),
            exit_after_file_chooser: true,
        })
    })
}

unsafe fn is_a(instance: *mut c_void, gtype: usize) -> bool {
    !instance.is_null() && g_type_check_instance_is_a(instance, gtype) != 0
}

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

unsafe fn glist_to_vec(list: *mut GList) -> Vec<*mut c_void> {
    let mut out = Vec::new();
    let mut curr = list;
    while !curr.is_null() {
        out.push((*curr).data);
        curr = (*curr).next;
    }
    if !list.is_null() {
        g_list_free(list);
    }
    out
}

/// GTK3's internal `response_cb` in `gtkfilechooserdialog.c` calls
/// `g_signal_stop_emission_by_name(dialog, "response")` when a programmatic
/// `gtk_dialog_response(dialog, GTK_RESPONSE_ACCEPT)` is emitted before the
/// internal file chooser widget finishes asynchronous directory loading, which
/// prevents `wxFileDialog`'s response handler from firing. Suppressing
/// `stop_emission_by_name` for `"response"` on `GtkFileChooser` lets `wxFileDialog`
/// receive `GTK_RESPONSE_ACCEPT` and query `gtk_file_chooser_get_filename`.
#[no_mangle]
pub unsafe extern "C" fn g_signal_stop_emission_by_name(
    instance: *mut c_void,
    detailed_signal: *const c_char,
) {
    static REAL_FN: OnceLock<unsafe extern "C" fn(*mut c_void, *const c_char)> = OnceLock::new();
    let real_fn = *REAL_FN.get_or_init(|| {
        let sym = CString::new("g_signal_stop_emission_by_name").unwrap();
        let ptr = dlsym(RTLD_NEXT, sym.as_ptr());
        std::mem::transmute(ptr)
    });

    if is_a(instance, gtk_file_chooser_get_type()) && !detailed_signal.is_null() {
        let sig = cstr_to_string(detailed_signal);
        if sig.starts_with("response") {
            return;
        }
    }
    real_fn(instance, detailed_signal);
}

unsafe extern "C" fn finish_and_exit_cb(_data: *mut c_void) -> c_int {
    let _ = fs::write("/work/done", "ok\n");
    _exit(0);
}

unsafe extern "C" fn auto_accept_file_chooser_cb(data: *mut c_void) -> c_int {
    let exit_after = {
        let mut st = state().lock().unwrap();
        if st.accepted {
            return G_SOURCE_REMOVE;
        }
        st.accepted = true;
        st.exit_after_file_chooser
    };
    gtk_dialog_response(data, GTK_RESPONSE_ACCEPT);
    if exit_after {
        g_timeout_add(400, finish_and_exit_cb, ptr::null_mut());
    } else {
        g_timeout_add(500, execute_next_cmd, ptr::null_mut());
    }
    G_SOURCE_REMOVE
}

#[no_mangle]
pub unsafe extern "C" fn gtk_widget_show(widget: *mut c_void) {
    static REAL_FN: OnceLock<unsafe extern "C" fn(*mut c_void)> = OnceLock::new();
    let real_fn = *REAL_FN.get_or_init(|| {
        let sym = CString::new("gtk_widget_show").unwrap();
        let ptr = dlsym(RTLD_NEXT, sym.as_ptr());
        std::mem::transmute(ptr)
    });

    real_fn(widget);
    if is_a(widget, gtk_file_chooser_get_type()) && is_a(widget, gtk_dialog_get_type()) {
        g_timeout_add(250, auto_accept_file_chooser_cb, widget);
    }
}

#[no_mangle]
pub unsafe extern "C" fn gtk_file_chooser_get_filename(_chooser: *mut c_void) -> *mut c_char {
    let path = state().lock().unwrap().save_target_path.clone();
    let cstr = CString::new(path).unwrap_or_else(|_| CString::new("/work/output.png").unwrap());
    g_strdup(cstr.as_ptr())
}

fn labels_match(actual: &str, expected: &str) -> bool {
    if actual.eq_ignore_ascii_case(expected) {
        return true;
    }
    let clean_a: String = actual.chars().filter(|&c| c != '_').collect();
    let clean_b: String = expected.chars().filter(|&c| c != '_').collect();
    clean_a.eq_ignore_ascii_case(&clean_b)
}

unsafe fn activate_menu_segments(
    container: *mut c_void,
    segments: &[&str],
    depth: usize,
) -> bool {
    if container.is_null() || depth >= segments.len() {
        return false;
    }
    if is_a(container, gtk_menu_item_get_type()) {
        let label = cstr_to_string(gtk_menu_item_get_label(container));
        if labels_match(&label, segments[depth]) {
            if depth == segments.len() - 1 {
                eprintln!("[HOOK] Activating menu item: {}", label);
                gtk_menu_item_activate(container);
                return true;
            }
            let sub = gtk_menu_item_get_submenu(container);
            if !sub.is_null() {
                // Activate parent submenu item first so wxGTK fires wxEVT_MENU_OPEN
                // and updates child menu item sensitivity before descending.
                gtk_menu_item_activate(container);
                return activate_menu_segments(sub, segments, depth + 1);
            }
        }
        return false;
    }
    if is_a(container, gtk_container_get_type()) {
        for child in glist_to_vec(gtk_container_get_children(container)) {
            if activate_menu_segments(child, segments, depth) {
                return true;
            }
        }
    }
    false
}

unsafe fn activate_menu_path(path_str: &str) {
    let segments: Vec<&str> = path_str
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        return;
    }
    for win in glist_to_vec(gtk_window_list_toplevels()) {
        if activate_menu_segments(win, &segments, 0) {
            break;
        }
    }
}

unsafe fn click_button_rec(w: *mut c_void, target_label: &str) -> bool {
    if w.is_null() {
        return false;
    }
    if is_a(w, gtk_button_get_type()) && !is_a(w, gtk_menu_item_get_type()) {
        let label = cstr_to_string(gtk_button_get_label(w));
        if target_label == "__PLAY__" {
            if label.is_empty() {
                eprintln!("[HOOK] Clicking play/pause icon button");
                gtk_button_clicked(w);
                return true;
            }
        } else if labels_match(&label, target_label) {
            eprintln!("[HOOK] Clicking dialog button: {}", label);
            gtk_button_clicked(w);
            return true;
        }
    }
    if is_a(w, gtk_container_get_type()) {
        for child in glist_to_vec(gtk_container_get_children(w)) {
            if click_button_rec(child, target_label) {
                return true;
            }
        }
    }
    false
}

unsafe fn click_dialog_button(target_label: &str) {
    for win in glist_to_vec(gtk_window_list_toplevels()) {
        if gtk_widget_get_visible(win) != 0 && click_button_rec(win, target_label) {
            break;
        }
    }
}

unsafe fn select_combo_box_rec(w: *mut c_void, index: c_int) -> bool {
    if w.is_null() {
        return false;
    }
    if is_a(w, gtk_combo_box_get_type()) {
        eprintln!("[HOOK] Setting GtkComboBox active index to {}", index);
        gtk_combo_box_set_active(w, index);
        return true;
    }
    if is_a(w, gtk_container_get_type()) {
        for child in glist_to_vec(gtk_container_get_children(w)) {
            if select_combo_box_rec(child, index) {
                return true;
            }
        }
    }
    false
}

unsafe fn select_first_combo_box(index: c_int) {
    for win in glist_to_vec(gtk_window_list_toplevels()) {
        if gtk_widget_get_visible(win) != 0 && select_combo_box_rec(win, index) {
            break;
        }
    }
}

unsafe fn collect_canvas_pizzas(w: *mut c_void, out: &mut Vec<*mut c_void>) {
    if w.is_null() {
        return;
    }
    if is_a(w, gtk_statusbar_get_type()) || is_a(w, gtk_menu_bar_get_type()) {
        return;
    }
    let type_name = cstr_to_string(g_type_name_from_instance(w));
    let mut alloc = GtkAllocation::default();
    gtk_widget_get_allocation(w, &mut alloc);
    if type_name == "wxPizza" && alloc.height > 100 {
        let win = gtk_widget_get_window(w);
        if !win.is_null() {
            let subwins = gdk_window_get_children(win);
            if subwins.is_null() {
                out.push(w);
            } else {
                g_list_free(subwins);
            }
        }
    }
    if is_a(w, gtk_container_get_type()) {
        for child in glist_to_vec(gtk_container_get_children(w)) {
            collect_canvas_pizzas(child, out);
        }
    }
}

unsafe fn visible_canvas_pizzas() -> Vec<*mut c_void> {
    let mut pizzas = Vec::new();
    for win in glist_to_vec(gtk_window_list_toplevels()) {
        if gtk_widget_get_visible(win) != 0 {
            collect_canvas_pizzas(win, &mut pizzas);
        }
    }
    if let Some(last) = pizzas.pop() {
        vec![last]
    } else {
        Vec::new()
    }
}

unsafe fn emit_button_event(
    w: *mut c_void,
    type_: c_int,
    x: f64,
    y: f64,
    time_ms: u32,
    state_mask: c_uint,
) {
    let win = gtk_widget_get_window(w);
    if win.is_null() {
        return;
    }
    let display = gdk_window_get_display(win);
    let seat = gdk_display_get_default_seat(display);
    let device = gdk_seat_get_pointer(seat);

    let mut ox: c_int = 0;
    let mut oy: c_int = 0;
    gdk_window_get_origin(win, &mut ox, &mut oy);
    let screen = gdk_window_get_screen(win);
    if !device.is_null() && !screen.is_null() {
        gdk_device_warp(device, screen, ox + (x as c_int), oy + (y as c_int));
    }

    let ev = gdk_event_new(type_);
    let btn = &mut *(ev as *mut GdkEventButton);
    btn.window = g_object_ref(win);
    btn.send_event = 1;
    btn.time = time_ms;
    btn.x = x;
    btn.y = y;
    btn.x_root = (ox as f64) + x;
    btn.y_root = (oy as f64) + y;
    btn.button = 1;
    btn.state = state_mask;
    gdk_event_set_device(ev, device);
    gtk_main_do_event(ev);
    gdk_event_free(ev);
}

unsafe fn emit_motion_event(
    w: *mut c_void,
    x: f64,
    y: f64,
    time_ms: u32,
    state_mask: c_uint,
) {
    let win = gtk_widget_get_window(w);
    if win.is_null() {
        return;
    }
    let display = gdk_window_get_display(win);
    let seat = gdk_display_get_default_seat(display);
    let device = gdk_seat_get_pointer(seat);

    let mut ox: c_int = 0;
    let mut oy: c_int = 0;
    gdk_window_get_origin(win, &mut ox, &mut oy);
    let screen = gdk_window_get_screen(win);
    if !device.is_null() && !screen.is_null() {
        gdk_device_warp(device, screen, ox + (x as c_int), oy + (y as c_int));
    }

    let ev = gdk_event_new(GDK_MOTION_NOTIFY);
    let motion = &mut *(ev as *mut GdkEventMotion);
    motion.window = g_object_ref(win);
    motion.send_event = 1;
    motion.time = time_ms;
    motion.x = x;
    motion.y = y;
    motion.x_root = (ox as f64) + x;
    motion.y_root = (oy as f64) + y;
    motion.state = state_mask;
    motion.is_hint = 0;
    gdk_event_set_device(ev, device);
    gtk_main_do_event(ev);
    gdk_event_free(ev);
}

struct ClickReleaseData {
    x: f64,
    y: f64,
    release_time: u32,
}

unsafe extern "C" fn release_click_cb(user_data: *mut c_void) -> c_int {
    let d = Box::from_raw(user_data as *mut ClickReleaseData);
    for pw in visible_canvas_pizzas() {
        emit_button_event(
            pw,
            GDK_BUTTON_RELEASE,
            d.x,
            d.y,
            d.release_time,
            GDK_BUTTON1_MASK,
        );
    }
    g_timeout_add(150, execute_next_cmd, ptr::null_mut());
    G_SOURCE_REMOVE
}

unsafe fn perform_click(x: f64, y: f64, hold_ms: u32) {
    eprintln!("[HOOK] CLICK ({:.1}, {:.1}) hold={}ms", x, y, hold_ms);
    let now_ms = (g_get_monotonic_time() / 1000) as u32;
    for pw in visible_canvas_pizzas() {
        emit_button_event(pw, GDK_BUTTON_PRESS, x, y, now_ms, 0);
    }
    let hold_ms = hold_ms.max(50);
    let data = Box::new(ClickReleaseData {
        x,
        y,
        release_time: now_ms.wrapping_add(hold_ms),
    });
    g_timeout_add(hold_ms, release_click_cb, Box::into_raw(data) as *mut c_void);
}

struct SwipeState {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    step: u32,
    total_steps: u32,
}

unsafe extern "C" fn swipe_step_cb(user_data: *mut c_void) -> c_int {
    let st = &mut *(user_data as *mut SwipeState);
    st.step += 1;
    let now_ms = (g_get_monotonic_time() / 1000) as u32;
    if st.step <= st.total_steps {
        let frac = (st.step as f64) / (st.total_steps as f64);
        let cx = st.x1 + (st.x2 - st.x1) * frac;
        let cy = st.y1 + (st.y2 - st.y1) * frac;
        for pw in visible_canvas_pizzas() {
            emit_motion_event(pw, cx, cy, now_ms, GDK_BUTTON1_MASK);
        }
        return G_SOURCE_CONTINUE;
    }
    for pw in visible_canvas_pizzas() {
        emit_button_event(
            pw,
            GDK_BUTTON_RELEASE,
            st.x2,
            st.y2,
            now_ms,
            GDK_BUTTON1_MASK,
        );
    }
    drop(Box::from_raw(user_data as *mut SwipeState));
    g_timeout_add(200, execute_next_cmd, ptr::null_mut());
    G_SOURCE_REMOVE
}

unsafe fn perform_swipe(x1: f64, y1: f64, x2: f64, y2: f64, duration_ms: u32) {
    eprintln!(
        "[HOOK] SWIPE ({:.1}, {:.1}) -> ({:.1}, {:.1}) duration={}ms",
        x1, y1, x2, y2, duration_ms
    );
    // Sim::Device::onMouseMotion queries wxGetLocalTimeMillis() and requires >50ms
    // wall-clock delta between motion events, as well as warping the X11 pointer.
    let total_steps: u32 = 6;
    let step_ms = (duration_ms / total_steps).max(65);
    let now_ms = (g_get_monotonic_time() / 1000) as u32;

    for pw in visible_canvas_pizzas() {
        emit_button_event(pw, GDK_BUTTON_PRESS, x1, y1, now_ms, 0);
    }
    let state = Box::new(SwipeState {
        x1,
        y1,
        x2,
        y2,
        step: 0,
        total_steps,
    });
    g_timeout_add(step_ms, swipe_step_cb, Box::into_raw(state) as *mut c_void);
}

unsafe extern "C" fn execute_next_cmd(_data: *mut c_void) -> c_int {
    let next_line = state().lock().unwrap().cmds.pop_front();
    let Some(line) = next_line else {
        return G_SOURCE_REMOVE;
    };

    if let Some(menu_path) = line.strip_prefix("MENU ") {
        activate_menu_path(menu_path.trim());
        g_timeout_add(250, execute_next_cmd, ptr::null_mut());
    } else if let Some(btn_label) = line.strip_prefix("DIALOG_BUTTON ") {
        click_dialog_button(btn_label.trim());
        g_timeout_add(250, execute_next_cmd, ptr::null_mut());
    } else if let Some(rest) = line.strip_prefix("CLICK ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 2 {
            let x: f64 = parts[0].parse().unwrap_or(0.0);
            let y: f64 = parts[1].parse().unwrap_or(0.0);
            let hold_ms: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
            perform_click(x, y, hold_ms);
        } else {
            g_timeout_add(50, execute_next_cmd, ptr::null_mut());
        }
    } else if let Some(rest) = line.strip_prefix("SWIPE ") {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 4 {
            let x1: f64 = parts[0].parse().unwrap_or(0.0);
            let y1: f64 = parts[1].parse().unwrap_or(0.0);
            let x2: f64 = parts[2].parse().unwrap_or(0.0);
            let y2: f64 = parts[3].parse().unwrap_or(0.0);
            let duration_ms: u32 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(360);
            perform_swipe(x1, y1, x2, y2, duration_ms);
        } else {
            g_timeout_add(50, execute_next_cmd, ptr::null_mut());
        }
    } else if let Some(rest) = line.strip_prefix("WAIT ") {
        let ms: u32 = rest.trim().parse().unwrap_or(10).max(10);
        g_timeout_add(ms, execute_next_cmd, ptr::null_mut());
    } else if let Some(rest) = line.strip_prefix("LOAD_FIT ") {
        {
            let mut st = state().lock().unwrap();
            st.accepted = false;
            st.save_target_path = rest.trim().to_string();
            st.exit_after_file_chooser = false;
        }
        // In the "Activity Data" dialog, the "Data Source" wxChoice (GtkComboBox)
        // defaults to index 2 ("Simulate Data"), which disables "Load File".
        // Selecting index 0 ("FIT/GPX Playable File") enables "Load File" and tells
        // OnPlayButton to play the loaded FIT file rather than random simulation.
        select_first_combo_box(0);
        click_dialog_button("Load File");
    } else if let Some(rest) = line.strip_prefix("SAVE ") {
        {
            let mut st = state().lock().unwrap();
            st.accepted = false;
            st.save_target_path = rest.trim().to_string();
            st.exit_after_file_chooser = true;
        }
        activate_menu_path("/_File/gtk-save");
    } else {
        g_timeout_add(10, execute_next_cmd, ptr::null_mut());
    }
    G_SOURCE_REMOVE
}

fn parse_hook_cmds(content: &str) -> VecDeque<String> {
    let mut cmds = VecDeque::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            cmds.push_back(line.to_string());
        }
    }
    if cmds.is_empty() {
        cmds.push_back("SAVE /work/output.png".to_string());
    }
    cmds
}

unsafe extern "C" fn poll_trigger_cb(_data: *mut c_void) -> c_int {
    {
        let st = state().lock().unwrap();
        if st.triggered || !Path::new("/work/trigger").exists() {
            return G_SOURCE_CONTINUE;
        }
    }

    let _ = fs::remove_file("/work/trigger");
    let content = fs::read_to_string("/work/hook_cmds.txt").unwrap_or_default();
    let cmds = parse_hook_cmds(&content);

    {
        let mut st = state().lock().unwrap();
        st.triggered = true;
        st.cmds = cmds;
    }

    g_timeout_add(50, execute_next_cmd, ptr::null_mut());
    G_SOURCE_REMOVE
}

extern "C" fn init_ciq_gtk_hook() {
    unsafe {
        g_timeout_add(100, poll_trigger_cb, ptr::null_mut());
    }
}

#[used]
#[cfg_attr(all(target_os = "linux", not(test)), link_section = ".init_array")]
static INIT: extern "C" fn() = init_ciq_gtk_hook;

#[cfg(test)]
use tests::ffi_stubs::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    pub(super) mod ffi_stubs {
        use super::*;
        pub unsafe fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn _exit(status: c_int) -> ! {
            std::process::exit(status)
        }
        pub unsafe fn g_timeout_add(
            _interval: c_uint,
            _function: GSourceFunc,
            _data: *mut c_void,
        ) -> c_uint {
            1
        }
        pub unsafe fn g_strdup(s: *const c_char) -> *mut c_char {
            let cstr = CStr::from_ptr(s);
            CString::new(cstr.to_bytes()).unwrap().into_raw()
        }
        pub unsafe fn g_get_monotonic_time() -> i64 {
            1_000_000
        }
        pub unsafe fn g_list_free(_list: *mut GList) {}
        pub unsafe fn g_object_ref(object: *mut c_void) -> *mut c_void {
            object
        }
        pub unsafe fn g_type_check_instance_is_a(
            _instance: *mut c_void,
            _iface_type: usize,
        ) -> c_int {
            0
        }
        pub unsafe fn g_type_name_from_instance(_instance: *mut c_void) -> *const c_char {
            b"GtkWidget\0".as_ptr() as *const c_char
        }
        pub unsafe fn gdk_window_get_children(_window: *mut c_void) -> *mut GList {
            ptr::null_mut()
        }
        pub unsafe fn gdk_window_get_display(_window: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gdk_window_get_screen(_window: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gdk_window_get_origin(
            _window: *mut c_void,
            x: *mut c_int,
            y: *mut c_int,
        ) -> c_int {
            if !x.is_null() {
                *x = 0;
            }
            if !y.is_null() {
                *y = 0;
            }
            1
        }
        pub unsafe fn gdk_display_get_default_seat(_display: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gdk_seat_get_pointer(_seat: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gdk_device_warp(
            _device: *mut c_void,
            _screen: *mut c_void,
            _x: c_int,
            _y: c_int,
        ) {
        }
        pub unsafe fn gdk_event_new(_type: c_int) -> *mut c_void {
            Box::into_raw(Box::new([0u8; 80])) as *mut c_void
        }
        pub unsafe fn gdk_event_set_device(_event: *mut c_void, _device: *mut c_void) {}
        pub unsafe fn gdk_event_free(event: *mut c_void) {
            if !event.is_null() {
                drop(Box::from_raw(event as *mut [u8; 80]));
            }
        }
        pub unsafe fn gtk_file_chooser_get_type() -> usize {
            1
        }
        pub unsafe fn gtk_dialog_get_type() -> usize {
            2
        }
        pub unsafe fn gtk_menu_item_get_type() -> usize {
            3
        }
        pub unsafe fn gtk_container_get_type() -> usize {
            4
        }
        pub unsafe fn gtk_button_get_type() -> usize {
            5
        }
        pub unsafe fn gtk_statusbar_get_type() -> usize {
            6
        }
        pub unsafe fn gtk_menu_bar_get_type() -> usize {
            7
        }
        pub unsafe fn gtk_combo_box_get_type() -> usize {
            8
        }
        pub unsafe fn gtk_dialog_response(_dialog: *mut c_void, _response_id: c_int) {}
        pub unsafe fn gtk_window_list_toplevels() -> *mut GList {
            ptr::null_mut()
        }
        pub unsafe fn gtk_widget_get_visible(_widget: *mut c_void) -> c_int {
            1
        }
        pub unsafe fn gtk_widget_get_allocation(
            _widget: *mut c_void,
            allocation: *mut GtkAllocation,
        ) {
            if !allocation.is_null() {
                *allocation = GtkAllocation::default();
            }
        }
        pub unsafe fn gtk_widget_get_window(_widget: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gtk_menu_item_get_label(_menu_item: *mut c_void) -> *const c_char {
            b"\0".as_ptr() as *const c_char
        }
        pub unsafe fn gtk_menu_item_get_submenu(_menu_item: *mut c_void) -> *mut c_void {
            ptr::null_mut()
        }
        pub unsafe fn gtk_menu_item_activate(_menu_item: *mut c_void) {}
        pub unsafe fn gtk_container_get_children(_container: *mut c_void) -> *mut GList {
            ptr::null_mut()
        }
        pub unsafe fn gtk_button_get_label(_button: *mut c_void) -> *const c_char {
            b"\0".as_ptr() as *const c_char
        }
        pub unsafe fn gtk_button_clicked(_button: *mut c_void) {}
        pub unsafe fn gtk_combo_box_set_active(_combo_box: *mut c_void, _index: c_int) {}
        pub unsafe fn gtk_main_do_event(_event: *mut c_void) {}
    }

    #[test]
    fn test_gdk_event_layouts() {
        assert_eq!(size_of::<GdkEventButton>(), 80);
        assert_eq!(offset_of!(GdkEventButton, type_), 0);
        assert_eq!(offset_of!(GdkEventButton, window), 8);
        assert_eq!(offset_of!(GdkEventButton, send_event), 16);
        assert_eq!(offset_of!(GdkEventButton, time), 20);
        assert_eq!(offset_of!(GdkEventButton, x), 24);
        assert_eq!(offset_of!(GdkEventButton, y), 32);
        assert_eq!(offset_of!(GdkEventButton, axes), 40);
        assert_eq!(offset_of!(GdkEventButton, state), 48);
        assert_eq!(offset_of!(GdkEventButton, button), 52);
        assert_eq!(offset_of!(GdkEventButton, device), 56);
        assert_eq!(offset_of!(GdkEventButton, x_root), 64);
        assert_eq!(offset_of!(GdkEventButton, y_root), 72);

        assert_eq!(size_of::<GdkEventMotion>(), 80);
        assert_eq!(offset_of!(GdkEventMotion, type_), 0);
        assert_eq!(offset_of!(GdkEventMotion, window), 8);
        assert_eq!(offset_of!(GdkEventMotion, send_event), 16);
        assert_eq!(offset_of!(GdkEventMotion, time), 20);
        assert_eq!(offset_of!(GdkEventMotion, x), 24);
        assert_eq!(offset_of!(GdkEventMotion, y), 32);
        assert_eq!(offset_of!(GdkEventMotion, axes), 40);
        assert_eq!(offset_of!(GdkEventMotion, state), 48);
        assert_eq!(offset_of!(GdkEventMotion, is_hint), 52);
        assert_eq!(offset_of!(GdkEventMotion, device), 56);
        assert_eq!(offset_of!(GdkEventMotion, x_root), 64);
        assert_eq!(offset_of!(GdkEventMotion, y_root), 72);
    }

    #[test]
    fn test_labels_match() {
        assert!(labels_match("_Data Fields", "Data Fields"));
        assert!(labels_match("Si_mulation", "Simulation"));
        assert!(labels_match("3 Fields A", "3 fields a"));
        assert!(labels_match("gtk-save", "gtk-save"));
        assert!(!labels_match("3 Fields A", "3 Fields B"));
    }

    #[test]
    fn test_parse_hook_cmds() {
        let script = r#"
            # Comment line
            MENU /_Data Fields/Layout/3 Fields A

            DIALOG_BUTTON Start
            CLICK 376.0 170.0 100
            SWIPE 208.0 130.0 52.0 130.0 360
            WAIT 1500
            SAVE /work/output.png
        "#;
        let cmds: Vec<String> = parse_hook_cmds(script).into_iter().collect();
        assert_eq!(
            cmds,
            vec![
                "MENU /_Data Fields/Layout/3 Fields A",
                "DIALOG_BUTTON Start",
                "CLICK 376.0 170.0 100",
                "SWIPE 208.0 130.0 52.0 130.0 360",
                "WAIT 1500",
                "SAVE /work/output.png",
            ]
        );

        let empty_cmds: Vec<String> = parse_hook_cmds("   \n# only comment\n").into_iter().collect();
        assert_eq!(empty_cmds, vec!["SAVE /work/output.png"]);
    }

    #[test]
    fn test_execute_save_and_load_fit_update_filename() {
        unsafe {
            state().lock().unwrap().cmds = parse_hook_cmds("LOAD_FIT /work/activity.fit");
            execute_next_cmd(ptr::null_mut());
            let raw = gtk_file_chooser_get_filename(ptr::null_mut());
            let filename = CString::from_raw(raw).into_string().unwrap();
            assert_eq!(filename, "/work/activity.fit");
            assert!(!state().lock().unwrap().exit_after_file_chooser);

            state().lock().unwrap().cmds = parse_hook_cmds("SAVE /tmp/custom_shot.png");
            execute_next_cmd(ptr::null_mut());
            let raw = gtk_file_chooser_get_filename(ptr::null_mut());
            let filename = CString::from_raw(raw).into_string().unwrap();
            assert_eq!(filename, "/tmp/custom_shot.png");
            assert!(state().lock().unwrap().exit_after_file_chooser);
        }
    }
}

