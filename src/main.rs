#![allow(unused)]

use std::cell::RefCell;
use std::rc::Rc;
use unicorn_engine::RegisterARM;
use unicorn_engine::Unicorn;
use unicorn_engine::unicorn_const::{Arch, Mode, Permission};

// 状态机共享上下文，用于在 Hook 与外层循环间通信
#[derive(Default)]
struct SimulationContext {
    pub hit_hook: bool,
    pub try_reentrant_start_failed: bool,
}

fn main() {
    // 1. 初始化 Unicorn (ARM32 小端模式)
    let mut uc = Unicorn::new(Arch::ARM, Mode::LITTLE_ENDIAN)
        .expect("Failed to initialize Unicorn instance");

    // 映射 4KB 内存 (0x1000)
    const CODE_ADDR: u64 = 0x1000;
    uc.mem_map(CODE_ADDR, 0x2000, Permission::ALL)
        .expect("Failed to map memory");

    // 编写两段指令：
    // 0x1000: NOP (0xe1a00000) -> 触发 Hook
    // 0x1004: NOP (0xe1a00000)
    // 0x2000: MOV R0, #42 (0xe3a0002a) -> 重启后要跳转的目标指令
    let code: [u8; 8] = [
        0x00, 0x00, 0xa0, 0xe1, // 0x1000: mov r0, r0 (nop)
        0x20, 0x00, 0xa0, 0xe3, // 0x1004: mov r0, r0 (nop)
    ];
    let new_target_code: [u8; 4] = [
        0x2a, 0x00, 0xa0, 0xe3, // 0x2000: mov r0, #42
    ];

    uc.mem_write(CODE_ADDR, &code).unwrap();
    uc.mem_write(CODE_ADDR + 0x1000, &new_target_code).unwrap();

    // 2. 设置共享状态，用于记录回调事件
    let ctx = Rc::new(RefCell::new(SimulationContext::default()));
    let ctx_clone = Rc::clone(&ctx);

    // 3. 注册 Code Hook，监听 0x1000
    uc.add_code_hook(CODE_ADDR, CODE_ADDR + 4, move |uc_hook, address, _size| {
        if address == CODE_ADDR {
            println!("[Hook] 触发目标地址: 0x{:x}", address);
            // ctx_clone.borrow_mut().hit_hook = true;

            // -------------------------------------------------------------
            // 实验 A：验证直接递归调用 emu_start 是否会失败 (UC_ERR_RESOURCE)
            // -------------------------------------------------------------
            let reentrant_res = uc_hook.emu_start(0x2000, 0x2004, 0, 0);
            if reentrant_res.is_err() {
                println!(
                    "[Hook] 验证成功: Hook 内部非法递归调用 emu_start 失败 -> {:?}",
                    reentrant_res.err()
                );
                // ctx_clone.borrow_mut().try_reentrant_start_failed = true;
            } else {
                println!("[Hook] 验证失败: Hook 内部非法递归调用 emu_start 成功");
                println!("R0: {:?}", uc_hook.reg_read(RegisterARM::R0).unwrap());
                println!("PC: 0x{:x}", uc_hook.reg_read(RegisterARM::PC).unwrap());
            }

            // -------------------------------------------------------------
            // 实验 B：正解！通过 emu_stop 打断当前执行流，安全交出控制权
            // -------------------------------------------------------------
            // println!("[Hook] 发送 emu_stop 指令请求安全打断当前循环...");
            // uc_hook.emu_stop().expect("Failed to stop emu");
        }
    })
    .expect("Failed to add code hook");

    // 4. 外层状态机控制循环（替代非法重入）
    let mut pc_to_run = CODE_ADDR;
    let mut end_addr = CODE_ADDR + 8;

    println!("\n[Main Loop] 启动模拟器程序流，PC: 0x{:x}", pc_to_run);
    let res = uc.emu_start(pc_to_run, end_addr, 0, 0);

    println!("Res {:?}", res);
    println!("R0: {:?}", uc.reg_read(RegisterARM::R0).unwrap());
    println!("PC: 0x{:x}", uc.reg_read(RegisterARM::PC).unwrap());

    return;

    loop {
        // 检查 Hook 是否被命中，以及是否触发了打断请求
        if ctx.borrow().hit_hook {
            println!("[Main Loop] 捕获到来自 Hook 的打断请求！");

            // 重置标记，准备下一次控制转移
            ctx.borrow_mut().hit_hook = false;

            // 更改 PC 和 运行终点，重新启动模拟器运行第二段代码 (0x2000)
            pc_to_run = 0x2000;
            end_addr = 0x2004;

            println!("[Main Loop] 准备切换上下文并重启模拟器...");
            continue;
        }

        // 如果模拟正常结束或出现其他错误，退出状态机
        if let Err(e) = res {
            println!("[Main Loop] 模拟器退出: {:?}", e);
        }
        break;
    }

    // 5. 验证重启后的执行结果
    assert!(
        ctx.borrow().try_reentrant_start_failed,
        "Hook 内部递归启动应该被阻止并报错"
    );

    let r0_val = uc.reg_read(RegisterARM::R0).unwrap();
    println!("\n[Final Check] 最终 R0 寄存器值: {} (预期: 42)", r0_val);
    assert_eq!(r0_val, 42, "说明重新启动后成功执行了 0x2000 处的代码！");
}
