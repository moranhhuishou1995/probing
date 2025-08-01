use signal_hook_registry::register;
use lazy_static::lazy_static;
use nix::libc::{self, c_int};
use std::collections::HashMap;
use std::sync::Mutex;
use signal_hook_registry::SigId;
use nix::sys::signal::Signal; // 显式导入Signal类型


lazy_static! {
    static ref ORIGINAL_HANDLER_IDS: Mutex<HashMap<c_int, SigId>> = Mutex::new(HashMap::new());
}

pub fn register_signal_handler<F>(sig: std::ffi::c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe {
        match signal_hook_registry::register_unchecked(sig, move |_: &_| handler()) {
            Ok(_) => {
                log::debug!("Registered signal handler for signal {sig}");
            }
            Err(e) => log::error!("Failed to register signal handler: {e}"),
        }
    };
}

pub fn register_exit_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe {
        // 使用不接受参数的闭包
        let original_handler = register(sig, || {}); 
        if let Ok(original_id) = original_handler {
            ORIGINAL_HANDLER_IDS.lock().unwrap().insert(sig, original_id);
            
            // 使用不接受参数的闭包
            let result = register(sig, move || {
                // 1. 执行自定义逻辑
                handler();
                
                // 2. 调用原始信号处理器
                let original_id = ORIGINAL_HANDLER_IDS.lock().unwrap().remove(&sig);
                if let Some(original_id) = original_id {
                    // 恢复原始处理器
                    signal_hook_registry::unregister(original_id);
                    // 重新触发信号 - 正确的类型转换：使用i32而非u32
                    let signal = Signal::try_from(sig)
                        .expect("Invalid signal number");
                    let _ = nix::sys::signal::kill(
                        nix::unistd::getpid(), 
                        signal
                    );
                }
            });
            
            if let Err(e) = result {
                log::error!("Failed to register exit signal handler for signal {sig}: {e}");
            } else {
                log::debug!("Registered exit signal handler for signal {sig}");
            }
        } else if let Err(e) = original_handler {
            log::error!("Failed to get original signal handler for {sig}: {e}");
        }
    }
}

#[ctor]
fn setup() {
    // 使用USR信号专用注册函数
    register_signal_handler(
        libc::SIGUSR2,
        crate::features::stack_tracer::backtrace_signal_handler,
    );
    register_signal_handler(
        libc::SIGUSR1,
        crate::features::stack_tracer::exit_signal_handler,
    );

    // 使用其他信号专用注册函数
    register_exit_signal_handler(
        libc::SIGABRT,
        crate::features::stack_tracer::exit_signal_handler,
    );
    
    register_exit_signal_handler(
        libc::SIGFPE,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_exit_signal_handler(
        libc::SIGILL,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_exit_signal_handler(
        libc::SIGSEGV,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_exit_signal_handler(
        libc::SIGINT,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_exit_signal_handler(
        libc::SIGBUS,
        crate::features::stack_tracer::exit_signal_handler,
    );
}
    