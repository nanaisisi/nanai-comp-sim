mod clock;
mod dram;
mod extended_simulation;
mod logic_gates;
mod memory_hierarchy;
mod qemu;
mod riscv;
mod riscv_cpu;
mod simulation;
mod waveform;

use clock::*;
use dram::*;
use extended_simulation::{ExtendedComputerSimulation, ExtendedSimConfig};
use logic_gates::*;
use memory_hierarchy::MemoryHierarchy;
use qemu::*;
use riscv::*;
use riscv_cpu::{CpuState, RiscVCpu};
use std::io::{self, Write};

fn main() {
    println!("=== Rust Advanced Computer Simulator ===");
    println!("RISC-V Architecture Support: 8/16/32/64-bit");
    println!("Features: Waveform Recording, QEMU Integration");
    println!();

    // インタラクティブメニュー
    loop {
        println!("Choose simulation mode:");
        println!("1. Basic Logic Gates & Clock Test");
        println!("2. DRAM Simulation Test");
        println!("3. Memory Hierarchy Test");
        println!("4. RISC-V CPU Demo (8-bit)");
        println!("5. RISC-V CPU Demo (16-bit)");
        println!("6. RISC-V CPU Demo (32-bit)");
        println!("7. RISC-V CPU Demo (64-bit)");
        println!("8. Extended Simulation with Waveforms");
        println!("9. QEMU Integration Demo");
        println!("A. Performance Benchmark");
        println!("0. Exit");
        print!("Enter choice: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" => run_basic_tests(),
            "2" => run_dram_test(),
            "3" => run_memory_hierarchy_test(),
            "4" => run_riscv_demo(ArchWidth::Bit8),
            "5" => run_riscv_demo(ArchWidth::Bit16),
            "6" => run_riscv_demo(ArchWidth::Bit32),
            "7" => run_riscv_demo(ArchWidth::Bit64),
            "8" => run_extended_simulation(),
            "9" => run_qemu_demo(),
            "A" | "a" => run_performance_benchmark(),
            "0" => break,
            _ => println!("Invalid choice. Please try again."),
        }

        println!("\nPress Enter to continue...");
        let mut _input = String::new();
        io::stdin().read_line(&mut _input).unwrap();
    }

    println!("Thank you for using Rust Computer Simulator!");
}

fn run_basic_tests() {
    println!("\n=== Basic Logic Gates & Clock Test ===");

    // 基本論理ゲートのテスト
    println!("1. Logic Gates Test:");
    println!(
        "  AND(1, 1) = {:?}",
        LogicGates::and(LogicLevel::High, LogicLevel::High)
    );
    println!(
        "  OR(0, 1) = {:?}",
        LogicGates::or(LogicLevel::Low, LogicLevel::High)
    );
    println!("  NOT(1) = {:?}", LogicGates::not(LogicLevel::High));
    println!(
        "  XOR(1, 0) = {:?}",
        LogicGates::xor(LogicLevel::High, LogicLevel::Low)
    );

    // 複合ゲートのテスト
    let (sum, carry) = CompositeGates::half_adder(LogicLevel::High, LogicLevel::High);
    println!("  Half Adder(1, 1): sum={sum:?}, carry={carry:?}");

    // クロックシステムのテスト
    println!("\n2. Clock System Test:");
    let mut sim_clock = SimulationClock::new(1_000_000.0, 500);
    sim_clock.add_clock("cpu".to_string(), 100_000.0);

    println!(
        "  Master Clock: {:.1}MHz",
        sim_clock.get_master_clock().get_frequency() / 1_000_000.0
    );

    for i in 0..5 {
        sim_clock.step();
        let master_edge = sim_clock.get_master_clock().get_edge();
        let cpu_edge = sim_clock.get_clock("cpu").unwrap().get_edge();

        println!(
            "  Step {}: Time={:.1}μs, Master={:?}, CPU={:?}",
            i + 1,
            sim_clock.get_current_time_us(),
            master_edge,
            cpu_edge
        );
    }
}

fn run_dram_test() {
    println!("\n=== DRAM Simulation Test ===");

    let mut dram_controller = DramController::new(16, 16, 64);
    let dram = dram_controller.get_dram_mut();

    // 書き込み操作
    println!("1. DRAM Write Operation:");
    dram.set_address_bus(vec![
        LogicLevel::Low,
        LogicLevel::High,
        LogicLevel::Low,
        LogicLevel::High,
    ]);
    dram.set_data_bus(vec![LogicLevel::High, LogicLevel::Low, LogicLevel::High]);
    dram.set_we(true);
    dram.set_ras(true);
    dram.set_cas(true);
    println!("  Data written to address [5]");

    // 読み出し操作
    println!("2. DRAM Read Operation:");
    dram.set_we(false);
    dram.set_ras(false);
    dram.set_cas(false);
    dram.set_ras(true);
    dram.set_cas(true);

    let data_bus = dram.get_data_bus();
    println!("  Read data: {:?}", data_bus[0]);

    // データ有効性チェック
    println!("3. Data Validity Check:");
    if let Some(data) = dram.get_cell_data(5, 5) {
        let validity = dram.is_cell_data_valid(5, 5, 64).unwrap_or(false);
        println!("  Cell data: {}, Valid: {}", data, validity);
    }

    // リフレッシュ間隔を超えた場合の動作をシミュレート
    std::thread::sleep(std::time::Duration::from_millis(100)); // 100ms待機
    if let Some(data) = dram.get_cell_data(5, 5) {
        let validity = dram.is_cell_data_valid(5, 5, 64).unwrap_or(false);
        println!("  After 100ms - Cell data: {}, Valid: {}", data, validity);
    }
}

fn run_riscv_demo(arch_width: ArchWidth) {
    println!("\n=== RISC-V {arch_width} CPU Demo ===");

    let mut cpu = RiscVCpu::new(arch_width);
    cpu.enable_trace(true);

    println!("1. CPU Initialization:");
    println!("  Architecture: {arch_width}");
    println!("  Initial PC: 0x{:016X}", cpu.get_pc());
    println!("  Register x0: {}", cpu.get_register(0)); // 常に0

    println!("\n2. Manual Instruction Execution:");

    // 手動でいくつかの命令を実行
    let instructions = vec![
        (0x02A00093, "addi x1, x0, 42"), // x1 = 42
        (0x01800113, "addi x2, x0, 24"), // x2 = 24
        (0x002081B3, "add x3, x1, x2"),  // x3 = x1 + x2
        (0x40208233, "sub x4, x1, x2"),  // x4 = x1 - x2
        (0x0020F2B3, "and x5, x1, x2"),  // x5 = x1 & x2
        (0x0020E333, "or x6, x1, x2"),   // x6 = x1 | x2
        (0x00100073, "ebreak"),          // halt
    ];

    for (inst_word, description) in instructions {
        println!("  Executing: {description}");
        cpu.clock_cycle(inst_word);

        println!(
            "    PC: 0x{:016X}, State: {:?}",
            cpu.get_pc(),
            cpu.get_state()
        );

        if matches!(cpu.get_state(), CpuState::Halt) {
            break;
        }
    }

    println!("\n3. Final Register Values:");
    for i in 0..8 {
        let value = cpu.get_register(i);
        match arch_width {
            ArchWidth::Bit8 => println!("  x{}: 0x{:02X} ({})", i, value as u8, value as u8),
            ArchWidth::Bit16 => println!("  x{}: 0x{:04X} ({})", i, value as u16, value as u16),
            ArchWidth::Bit32 => println!("  x{}: 0x{:08X} ({})", i, value as u32, value as u32),
            ArchWidth::Bit64 => println!("  x{i}: 0x{value:016X} ({value})"),
        }
    }

    println!("\n4. Performance Statistics:");
    println!("  Total Cycles: {}", cpu.get_cycle_count());
    println!("  Instructions Executed: {}", cpu.get_instruction_count());

    let ipc = if cpu.get_cycle_count() > 0 {
        cpu.get_instruction_count() as f64 / cpu.get_cycle_count() as f64
    } else {
        0.0
    };
    println!("  Instructions Per Cycle: {ipc:.3}");

    println!("\n5. Execution Trace:");
    let trace = cpu.get_trace();
    for (i, entry) in trace.iter().enumerate().take(5) {
        println!(
            "  #{}: PC=0x{:08X}, Inst={:?}",
            i, entry.pc, entry.instruction
        );
    }
    if trace.len() > 5 {
        println!("  ... and {} more entries", trace.len() - 5);
    }
}

fn run_extended_simulation() {
    println!("\n=== Extended Simulation with Waveforms ===");

    let config = ExtendedSimConfig {
        arch_width: ArchWidth::Bit32,
        enable_waveform_recording: true,
        waveform_output_file: Some("extended_sim.vcd".to_string()),
        enable_qemu_integration: false,
        qemu_config: None,
        max_cycles: Some(500),
        enable_performance_analysis: true,
    };

    let mut sim = ExtendedComputerSimulation::new(config);

    println!("1. Running Demo Programs:");
    match sim.run_demo_programs() {
        Ok(_) => println!("  All demo programs completed successfully!"),
        Err(e) => println!("  Error running demos: {e}"),
    }

    println!("\n2. Waveform Analysis:");
    let recorder = &sim.waveform_recorder;
    println!("  VCD file generated: extended_sim.vcd");
    println!("  Recorded signals: {}", recorder.get_signal_names().len());
    println!("  Recording duration: {}ns", recorder.get_recording_time());

    // テキスト波形の表示例
    let text_waveform = recorder.generate_text_waveform("pc", 0, 1000, 80);
    println!("\n3. PC Waveform (first 1000ns):");
    println!("{text_waveform}");
}

fn run_qemu_demo() {
    println!("\n=== QEMU Integration Demo ===");

    let qemu_config = QemuConfig {
        qemu_path: "qemu-system-riscv32".to_string(),
        machine_type: "virt".to_string(),
        cpu_type: "rv32".to_string(),
        memory_size: "64M".to_string(),
        kernel_path: None,
        bios_path: None,
        enable_gdb: true,
        gdb_port: 1234,
        enable_monitor: true,
        monitor_port: 55555,
    };

    println!("1. QEMU Configuration:");
    println!("  QEMU Path: {}", qemu_config.qemu_path);
    println!("  Machine: {}", qemu_config.machine_type);
    println!("  CPU: {}", qemu_config.cpu_type);
    println!("  Memory: {}", qemu_config.memory_size);
    println!("  GDB Port: {}", qemu_config.gdb_port);

    println!("\n2. ELF Binary Generation:");
    let mut elf_gen = ElfGenerator::new(ArchWidth::Bit32);

    // 簡単なプログラムを作成
    let program = vec![
        RiscVInstruction::Addi(1, 0, 100),
        RiscVInstruction::Addi(2, 0, 200),
        RiscVInstruction::Add(3, 1, 2),
        RiscVInstruction::Halt,
    ];

    for instruction in program {
        if let Err(e) = elf_gen.add_instruction(instruction) {
            println!("  Error adding instruction: {e}");
            return;
        }
    }

    println!("  Instructions added: {}", elf_gen.get_instruction_count());
    println!("  Entry point: 0x{:016X}", elf_gen.get_entry_point());

    println!("\n3. Assembly Listing:");
    let listing = elf_gen.generate_assembly_listing();
    println!("{listing}");

    println!("\n4. Binary Generation:");
    match elf_gen.generate_binary(std::path::Path::new("test_program.bin")) {
        Ok(_) => println!("  Binary file generated: test_program.bin"),
        Err(e) => println!("  Error generating binary: {e}"),
    }

    println!("\n5. QEMU Integration:");
    println!("  To run with QEMU, use:");
    println!("  qemu-system-riscv32 -machine virt -cpu rv32 -m 64M \\");
    println!("    -kernel test_program.bin -nographic -gdb tcp::1234 -S");
    println!("  Then connect with GDB on port 1234");
}

fn run_performance_benchmark() {
    println!("\n=== Performance Benchmark ===");

    let architectures = vec![
        ArchWidth::Bit8,
        ArchWidth::Bit16,
        ArchWidth::Bit32,
        ArchWidth::Bit64,
    ];

    for arch in architectures {
        println!("\n--- {arch} Architecture ---");

        let config = ExtendedSimConfig {
            arch_width: arch,
            enable_waveform_recording: false,
            waveform_output_file: None,
            enable_qemu_integration: false,
            qemu_config: None,
            max_cycles: Some(10000),
            enable_performance_analysis: true,
        };

        let mut sim = ExtendedComputerSimulation::new(config);

        // ベンチマークプログラム（フィボナッチ数列）
        let program = vec![
            RiscVInstruction::Addi(1, 0, 1),  // x1 = 1
            RiscVInstruction::Addi(2, 0, 1),  // x2 = 1
            RiscVInstruction::Addi(3, 0, 20), // x3 = 20 (iterations)
            // loop:
            RiscVInstruction::Add(4, 1, 2),   // x4 = x1 + x2
            RiscVInstruction::Add(1, 2, 0),   // x1 = x2
            RiscVInstruction::Add(2, 4, 0),   // x2 = x4
            RiscVInstruction::Addi(3, 3, -1), // x3--
            RiscVInstruction::Bne(3, 0, -16), // branch if x3 != 0
            RiscVInstruction::Halt,
        ];

        match sim.load_program(&program, 0x80000000) {
            Ok(_) => {
                let start_time = std::time::Instant::now();
                match sim.run() {
                    Ok(_) => {
                        let elapsed = start_time.elapsed();
                        let stats = sim.get_performance_stats();

                        println!("  Execution time: {:.2}ms", elapsed.as_millis());
                        println!("  Total cycles: {}", stats.total_cycles);
                        println!("  Instructions: {}", stats.total_instructions);
                        println!("  Memory accesses: {}", stats.memory_accesses);

                        let ipc = if stats.total_cycles > 0 {
                            stats.total_instructions as f64 / stats.total_cycles as f64
                        } else {
                            0.0
                        };
                        println!("  IPC: {ipc:.3}");

                        let mips = if elapsed.as_secs_f64() > 0.0 {
                            stats.total_instructions as f64 / elapsed.as_secs_f64() / 1_000_000.0
                        } else {
                            0.0
                        };
                        println!("  MIPS: {mips:.3}");

                        println!("  Final result: {}", sim.get_riscv_cpu().get_register(2));
                    }
                    Err(e) => println!("  Simulation error: {e}"),
                }
            }
            Err(e) => println!("  Program load error: {e}"),
        }
    }
}

fn run_memory_hierarchy_test() {
    println!("\n=== Memory Hierarchy Test ===");

    // マイコン構成のテスト
    println!("1. Microcontroller Memory Hierarchy:");
    let mut mcu_memory = MemoryHierarchy::new_microcontroller(
        8 * 1024,   // 8KB L1 ICache
        8 * 1024,   // 8KB L1 DCache
        32 * 1024,  // 32KB Internal SRAM
        128 * 1024, // 128KB Internal DRAM
    );

    println!("  {}", mcu_memory.get_memory_info());

    // メモリアクセステスト
    println!("\n  Memory Access Tests:");

    // 命令読み込みテスト
    println!("  - Instruction fetch test...");
    for i in 0..10 {
        let addr = 0x0000_1000 + i * 4;
        if let Some(_instr) = mcu_memory.read_instruction(addr) {
            println!("    Fetched instruction from 0x{:08X}", addr);
        }
    }

    // データ読み書きテスト
    println!("  - Data access test...");
    let test_data = vec![0x12, 0x34, 0x56, 0x78];
    let sram_addr = 0x2000_0100;

    if mcu_memory.write_data(sram_addr, &test_data) {
        println!("    Wrote data to SRAM at 0x{:08X}", sram_addr);
    }

    if let Some(read_data) = mcu_memory.read_data(sram_addr, 4) {
        println!("    Read data from SRAM: {:02X?}", read_data);
    }

    let (icache_hit, dcache_hit) = mcu_memory.get_cache_statistics();
    println!("  Cache Performance:");
    println!("    I-Cache Hit Rate: {:.2}%", icache_hit * 100.0);
    println!("    D-Cache Hit Rate: {:.2}%", dcache_hit * 100.0);

    println!("\n2. PC/Server Memory Hierarchy:");
    let external_dram = DramController::new(256, 256, 64); // 256x256 DRAMアレイ
    let pc_memory = MemoryHierarchy::new_pc_server(
        32 * 1024, // 32KB L1 ICache
        32 * 1024, // 32KB L1 DCache
        64 * 1024, // 64KB Internal SRAM
        external_dram,
    );

    println!("  {}", pc_memory.get_memory_info());

    // PC/サーバーのメモリアクセステスト
    println!("\n  Memory Access Tests:");

    // より多くの命令フェッチ（キャッシュ効果を確認）
    println!("  - Intensive instruction fetch test...");
    for i in 0..100 {
        let addr = 0x8000_0000 + i * 4;
        mcu_memory.read_instruction(addr);
    }

    // 同じアドレスに再アクセス（キャッシュヒット確認）
    for i in 0..50 {
        let addr = 0x8000_0000 + i * 4;
        mcu_memory.read_instruction(addr);
    }

    let (icache_hit2, dcache_hit2) = pc_memory.get_cache_statistics();
    println!("  Cache Performance after intensive access:");
    println!("    I-Cache Hit Rate: {:.2}%", icache_hit2 * 100.0);
    println!("    D-Cache Hit Rate: {:.2}%", dcache_hit2 * 100.0);

    println!("\n3. Memory Architecture Comparison:");
    println!("  Microcontroller:");
    println!("    - CPU内部に小容量キャッシュ");
    println!("    - 内部SRAMで高速アクセス");
    println!("    - 内部DRAMでコスト効率");
    println!("    - 外部メモリ不要");

    println!("  PC/Server:");
    println!("    - CPU内部に大容量キャッシュ");
    println!("    - 外部DRAMで大容量");
    println!("    - 独立したDRAMコントローラー");
    println!("    - 異なるクロックドメイン");

    println!("\n4. Clock Domain Analysis:");
    println!("  - CPU内部メモリ（SRAM/キャッシュ）: CPUクロック同期");
    println!("  - 外部DRAM: 独立したメモリクロック");
    println!("  - メモリコントローラー: クロックドメイン間の橋渡し");
}
