//! Process management syscalls
use crate::task::{change_program_brk, current_addr_readable, current_addr_writable, current_mmap, current_munmap, current_user_token, exit_current_and_run_next, suspend_current_and_run_next};
use crate::mm::MapPermission;
use crate::config::PAGE_SIZE;
use crate::mm::translated_byte_buffer;
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if ts.is_null() {
        return -1;
    }
    
    let token = current_user_token();
    let ts_buffer = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    
    if ts_buffer.is_empty() {
        return -1;
    }
    
    let current_time_us = get_time_us();
    let sec = current_time_us / 1_000_000;
    let usec = current_time_us % 1_000_000;
    
    let time_val = TimeVal { sec, usec };
    
    // Copy TimeVal to user space through translated buffer
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            &time_val as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    
    let mut offset = 0;
    for buffer in ts_buffer {
        let copy_len = (time_val_bytes.len() - offset).min(buffer.len());
        buffer[..copy_len].copy_from_slice(&time_val_bytes[offset..offset + copy_len]);
        offset += copy_len;
        if offset >= time_val_bytes.len() {
            break;
        }
    }
    
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    
    match trace_request {
        0 => {
            // 读取操作: trace_request为0，id为地址
            let addr = id;
            if !current_addr_readable(addr) {
                return -1; // 地址不可见或不可读
            }
            
            let token = current_user_token();
            let buffer = translated_byte_buffer(token, addr as *const u8, 1);
            if buffer.is_empty() {
                return -1;
            }
            
            buffer[0][0] as isize // 返回读取到的字节值
        }
        1 => {
            // 写入操作: trace_request为1，id为地址，data为要写入的数据
            let addr = id;
            let value = data as u8;
            
            if !current_addr_writable(addr) {
                return -1; // 地址不可见或不可写
            }
            
            let token = current_user_token();
            let mut buffer = translated_byte_buffer(token, addr as *const u8, 1);
            if buffer.is_empty() {
                return -1;
            }
            
            buffer[0][0] = value;
            0 // 写入成功
        }
        _ => -1, // 无效的trace_request
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap start={:#x}, len={}, prot={:#x}", start, len, prot);
    
    // 检查参数错误
    // 1. start 必须按页对齐
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    
    // 2. prot 其余位必须为0
    if prot & !0x7 != 0 {
        return -1;
    }
    
    // 3. prot 不能为0 (这样的内存无意义)
    if prot & 0x7 == 0 {
        return -1;
    }
    
    // len 可以为0，如果为0直接返回成功
    if len == 0 {
        return 0;
    }
    
    // 向上取整到页边界
    let len_aligned = (len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    
    // 转换prot参数到MapPermission
    // prot: bit 0=可读, bit 1=可写, bit 2=可执行
    // MapPermission: R=1<<1, W=1<<2, X=1<<3, U=1<<4
    let mut map_perm = MapPermission::U; // 用户可访问
    if prot & 0x1 != 0 { // 可读
        map_perm |= MapPermission::R;
    }
    if prot & 0x2 != 0 { // 可写
        map_perm |= MapPermission::W;
    }
    if prot & 0x4 != 0 { // 可执行
        map_perm |= MapPermission::X;
    }
    
    // 执行映射
    if current_mmap(start, len_aligned, map_perm) {
        0
    } else {
        -1 // 映射失败，可能是地址已被占用或物理内存不足
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap start={:#x}, len={}", start, len);
    
    // 检查参数错误
    // 1. start 必须按页对齐
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    
    // len 可以为0，如果为0直接返回成功
    if len == 0 {
        return 0;
    }
    
    // 向上取整到页边界
    let len_aligned = (len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    
    // 执行解除映射
    if current_munmap(start, len_aligned) {
        0
    } else {
        -1 // 解除映射失败，可能是有未映射的虚存
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
