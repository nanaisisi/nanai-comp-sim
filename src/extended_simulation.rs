use crate::clock::*;
use crate::dram::*;
use crate::riscv::{ArchWidth, RiscVInstruction};
use crate::riscv_cpu::RiscVCpu;
use crate::waveform::WaveformRecorder;
use crate::qemu::{ElfGenerator, QemuConfig, QemuGdbStub};
use std::collections::HashMap;

/// 拡張シミュレーション構成
#[derive(Debug, Clone)]
pub struct ExtendedSimConfig {
    pub arch_width: ArchWidth,
    pub enable_waveform_recording: bool,
    pub waveform_output_file: Option<String>,
    pub enable_qemu_integration: bool,
    pub qemu_config: Option<QemuConfig>,
    pub max_cycles: Option<u64>,
    pub enable_performance_analysis: bool,
}

impl Default for ExtendedSimConfig {
    fn default() -> Self {
        Self {
            arch_width: ArchWidth::Bit32,
            enable_waveform_recording: true,
            waveform_output_file: Some("simulation.vcd".to_string()),
            enable_qemu_integration: false,
            qemu_config: None,
            max_cycles: Some(10000),
            enable_performance_analysis: true,
        }
    }
}

/// パフォーマンス統計
#[derive(Debug, Default)]
pub struct PerformanceStats {
    pub total_cycles: u64,
    pub total_instructions: u64,
    pub memory_accesses: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub branch_predictions: u64,
    pub branch_mispredictions: u64,
}

/// 拡張コンピュータシミュレーション
#[derive(Debug)]
pub struct ExtendedComputerSimulation {
    config: ExtendedSimConfig,
    simulation_clock: SimulationClock,
    riscv_cpu: RiscVCpu,
    dram_controller: DramController,
    pub waveform_recorder: WaveformRecorder,
    qemu_stub: Option<QemuGdbStub>,
    elf_generator: ElfGenerator,
    
    // シミュレーション状態
    running: bool,
    step_count: u64,
    memory: HashMap<u64, u8>, // 簡単なメモリモデル
    
    // パフォーマンス解析
    performance_stats: PerformanceStats,
}

impl ExtendedComputerSimulation {
    pub fn new(config: ExtendedSimConfig) -> Self {
        let mut simulation_clock = SimulationClock::new(100_000_000.0, 10); // 100MHz, 10ns step
        simulation_clock.add_clock("cpu".to_string(), 100_000_000.0); // 100MHz CPU
        simulation_clock.add_clock("memory".to_string(), 200_000_000.0); // 200MHz memory
        
        let riscv_cpu = RiscVCpu::new(config.arch_width);
        let dram_controller = DramController::new(1024, 1024, 64); // 1MB DRAM
        
        let mut waveform_recorder = WaveformRecorder::new();
        if config.enable_waveform_recording {
            // 波形記録用信号を追加
            waveform_recorder.add_signal("clk".to_string(), 1);
            waveform_recorder.add_signal("reset".to_string(), 1);
            waveform_recorder.add_signal("pc".to_string(), 64);
            waveform_recorder.add_signal("cpu_state".to_string(), 8);
            waveform_recorder.add_signal("mem_addr".to_string(), 64);
            waveform_recorder.add_signal("mem_data".to_string(), 64);
            waveform_recorder.add_signal("mem_write".to_string(), 1);
            
            // レジスタ
            for i in 0..32 {
                waveform_recorder.add_signal(format!("x{i}"), 64);
            }
        }
        
        let qemu_stub = if config.enable_qemu_integration {
            config.qemu_config.as_ref().map(|qemu_config| {
                QemuGdbStub::new(qemu_config.clone())
            })
        } else {
            None
        };
        
        let elf_generator = ElfGenerator::new(config.arch_width);
        
        Self {
            config,
            simulation_clock,
            riscv_cpu,
            dram_controller,
            waveform_recorder,
            qemu_stub,
            elf_generator,
            running: false,
            step_count: 0,
            memory: HashMap::new(),
            performance_stats: PerformanceStats::default(),
        }
    }

    /// プログラムをメモリにロード
    pub fn load_program(&mut self, program: &[RiscVInstruction], start_address: u64) -> Result<(), String> {
        println!("Loading program with {} instructions at 0x{:016X}", program.len(), start_address);
        
        let mut address = start_address;
        for instruction in program {
            // 命令をELFジェネレータに追加
            self.elf_generator.add_instruction(instruction.clone())?;
            
            // 命令をメモリに配置（簡略化）
            let encoded = self.encode_instruction_simple(instruction)?;
            let bytes = encoded.to_le_bytes();
            for (i, &byte) in bytes.iter().enumerate() {
                self.memory.insert(address + i as u64, byte);
            }
            address += 4;
        }
        
        // CPUのPCを設定
        self.riscv_cpu.set_pc(start_address);
        
        println!("Program loaded successfully");
        Ok(())
    }

    /// 簡単な命令エンコード（デモ用）
    fn encode_instruction_simple(&self, instruction: &RiscVInstruction) -> Result<u32, String> {
        match instruction {
            RiscVInstruction::Addi(rd, rs1, imm) => {
                Ok(0x00000013 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | (((*imm as u32) & 0xFFF) << 20))
            }
            RiscVInstruction::Add(rd, rs1, rs2) => {
                Ok(0x00000033 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20))
            }
            RiscVInstruction::Bne(rs1, rs2, imm) => {
                let imm_val = *imm as u32;
                let imm_11 = (imm_val >> 11) & 1;
                let imm_4_1 = (imm_val >> 1) & 0xF;
                let imm_10_5 = (imm_val >> 5) & 0x3F;
                let imm_12 = (imm_val >> 12) & 1;
                Ok(0x00001063 | (imm_11 << 7) | (imm_4_1 << 8) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20) | (imm_10_5 << 25) | (imm_12 << 31))
            }
            RiscVInstruction::Halt => {
                Ok(0x00100073) // ebreak
            }
            RiscVInstruction::Nop => {
                Ok(0x00000013) // addi x0, x0, 0
            }
            RiscVInstruction::Lw(rd, rs1, imm) => {
                let imm_masked = (*imm as u32) & 0xFFF;
                Ok(0x00002003 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | (imm_masked << 20))
            }
            RiscVInstruction::Sw(rs2, rs1, imm) => {
                let imm_val = *imm as u32;
                let imm_low = imm_val & 0x1F;
                let imm_high = (imm_val >> 5) & 0x7F;
                Ok(0x00002023 | (imm_low << 7) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20) | (imm_high << 25))
            }
            _ => Err(format!("Instruction encoding not implemented: {instruction:?}")),
        }
    }

    /// メモリ読み取り
    fn read_memory(&mut self, address: u64, size: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for i in 0..size {
            let byte = self.memory.get(&(address + i as u64)).copied().unwrap_or(0);
            data.push(byte);
        }
        self.performance_stats.memory_accesses += 1;
        data
    }

    /// メモリ書き込み
    fn write_memory(&mut self, address: u64, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.memory.insert(address + i as u64, byte);
        }
        self.performance_stats.memory_accesses += 1;
    }

    /// シミュレーション開始
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running = true;
        
        if self.config.enable_waveform_recording {
            let vcd_file = self.config.waveform_output_file.as_deref();
            self.waveform_recorder.start_recording(vcd_file)?;
        }
        
        if let Some(ref mut qemu_stub) = self.qemu_stub {
            qemu_stub.start_qemu()?;
        }
        
        println!("Extended simulation started with {} configuration", self.config.arch_width);
        Ok(())
    }

    /// シミュレーション停止
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running = false;
        
        if self.config.enable_waveform_recording {
            self.waveform_recorder.stop_recording()?;
            println!("Waveform data saved to: {:?}", self.config.waveform_output_file);
        }
        
        println!("Extended simulation stopped");
        Ok(())
    }

    /// 単一ステップ実行
    pub fn step(&mut self) -> bool {
        if !self.running {
            return false;
        }

        if let Some(max_cycles) = self.config.max_cycles
            && self.step_count >= max_cycles {
                return false;
            }        // クロック進行
        self.simulation_clock.step();
        
        // メモリアクセス処理
        let memory_data = if self.riscv_cpu.is_memory_write() {
            // メモリ書き込み
            let addr = self.riscv_cpu.get_memory_address();
            let data = self.riscv_cpu.get_memory_data_write();
            self.write_memory(addr, &data.to_le_bytes());
            0
        } else {
            // メモリ読み込み
            let addr = self.riscv_cpu.get_memory_address();
            let bytes = self.read_memory(addr, 4);
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        };

        // CPU実行
        self.riscv_cpu.clock_cycle(memory_data);
        
        // DRAM更新 - cpu_clockを後から取得
        let cpu_clock = self.simulation_clock.get_clock("cpu").unwrap();
        self.dram_controller.on_clock_edge(cpu_clock.get_edge(), "cpu");

        // 波形記録
        if self.config.enable_waveform_recording {
            self.waveform_recorder.advance_time(self.simulation_clock.time_step_ns);
            self.record_waveforms();
        }

        // パフォーマンス統計更新
        if self.config.enable_performance_analysis {
            self.update_performance_stats();
        }

        self.step_count += 1;

        // 停止条件チェック
        matches!(self.riscv_cpu.get_state(), crate::riscv_cpu::CpuState::Halt)
    }

    /// 波形記録
    fn record_waveforms(&mut self) {
        // クロック信号
        let cpu_clock = self.simulation_clock.get_clock("cpu").unwrap();
        self.waveform_recorder.record_clock("clk", cpu_clock.get_edge());
        
        // CPU状態記録
        self.riscv_cpu.record_waveforms(&mut self.waveform_recorder);
    }

    /// パフォーマンス統計更新
    fn update_performance_stats(&mut self) {
        self.performance_stats.total_cycles = self.riscv_cpu.get_cycle_count();
        self.performance_stats.total_instructions = self.riscv_cpu.get_instruction_count();
    }

    /// シミュレーション実行
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start()?;
        
        let start_time = std::time::Instant::now();
        let mut last_report_time = start_time;
        
        while self.step() {
            // 定期レポート（大きなプログラムの場合）
            if last_report_time.elapsed() >= std::time::Duration::from_secs(5) {
                self.print_progress_report();
                last_report_time = std::time::Instant::now();
            }
        }
        
        let elapsed = start_time.elapsed();
        self.stop()?;
        
        println!("Simulation completed in {:.2}ms", elapsed.as_millis());
        
        Ok(())
    }

    /// 進捗レポート
    fn print_progress_report(&self) {
        let ipc = if self.performance_stats.total_cycles > 0 {
            self.performance_stats.total_instructions as f64 / self.performance_stats.total_cycles as f64
        } else {
            0.0
        };
        
        println!("=== Progress Report ===");
        println!("Steps: {}", self.step_count);
        println!("CPU Cycles: {}", self.performance_stats.total_cycles);
        println!("Instructions: {}", self.performance_stats.total_instructions);
        println!("IPC: {ipc:.3}");
        println!("Memory Accesses: {}", self.performance_stats.memory_accesses);
        println!("PC: 0x{:016X}", self.riscv_cpu.get_pc());
        println!("CPU State: {:?}", self.riscv_cpu.get_state());
    }

    /// デモプログラム実行
    pub fn run_demo_programs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Running Demo Programs ===");
        
        // デモ1: フィボナッチ数列
        self.run_fibonacci_demo()?;
        
        // デモ2: 基本算術演算
        self.run_arithmetic_demo()?;
        
        // デモ3: メモリアクセステスト
        self.run_memory_test_demo()?;
        
        Ok(())
    }

    fn run_fibonacci_demo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n--- Fibonacci Demo ---");
        
        let program = vec![
            RiscVInstruction::Addi(1, 0, 1),    // x1 = 1 (fib(1))
            RiscVInstruction::Addi(2, 0, 1),    // x2 = 1 (fib(2))
            RiscVInstruction::Addi(3, 0, 10),   // x3 = 10 (counter)
            // loop:
            RiscVInstruction::Add(4, 1, 2),     // x4 = x1 + x2 (next fib)
            RiscVInstruction::Add(1, 2, 0),     // x1 = x2
            RiscVInstruction::Add(2, 4, 0),     // x2 = x4
            RiscVInstruction::Addi(3, 3, -1),   // x3 = x3 - 1
            RiscVInstruction::Bne(3, 0, -16),   // if x3 != 0 goto loop
            RiscVInstruction::Halt,
        ];
        
        self.reset_simulation();
        self.load_program(&program, 0x80000000)?;
        self.config.max_cycles = Some(1000);
        self.run()?;
        
        println!("Fibonacci result in x2: {}", self.riscv_cpu.get_register(2));
        Ok(())
    }

    fn run_arithmetic_demo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n--- Arithmetic Demo ---");
        
        let program = vec![
            RiscVInstruction::Addi(1, 0, 42),   // x1 = 42
            RiscVInstruction::Addi(2, 0, 24),   // x2 = 24
            RiscVInstruction::Add(3, 1, 2),     // x3 = x1 + x2
            RiscVInstruction::Sub(4, 1, 2),     // x4 = x1 - x2
            RiscVInstruction::And(5, 1, 2),     // x5 = x1 & x2
            RiscVInstruction::Or(6, 1, 2),      // x6 = x1 | x2
            RiscVInstruction::Xor(7, 1, 2),     // x7 = x1 ^ x2
            RiscVInstruction::Halt,
        ];
        
        self.reset_simulation();
        self.load_program(&program, 0x80000000)?;
        self.config.max_cycles = Some(100);
        self.run()?;
        
        println!("Results:");
        println!("  42 + 24 = {}", self.riscv_cpu.get_register(3));
        println!("  42 - 24 = {}", self.riscv_cpu.get_register(4));
        println!("  42 & 24 = {}", self.riscv_cpu.get_register(5));
        println!("  42 | 24 = {}", self.riscv_cpu.get_register(6));
        println!("  42 ^ 24 = {}", self.riscv_cpu.get_register(7));
        
        Ok(())
    }

    fn run_memory_test_demo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n--- Memory Test Demo ---");
        
        let program = vec![
            RiscVInstruction::Addi(1, 0, 0x100),  // x1 = 0x100 (address)
            RiscVInstruction::Addi(2, 0, 0xDEAD),  // x2 = 0xDEAD (data)
            RiscVInstruction::Sw(2, 1, 0),         // store x2 to [x1]
            RiscVInstruction::Lw(3, 1, 0),         // load x3 from [x1]
            RiscVInstruction::Halt,
        ];
        
        self.reset_simulation();
        self.load_program(&program, 0x80000000)?;
        self.config.max_cycles = Some(100);
        self.run()?;
        
        println!("Memory test result: 0x{:X}", self.riscv_cpu.get_register(3));
        
        Ok(())
    }

    fn reset_simulation(&mut self) {
        self.riscv_cpu.reset();
        self.step_count = 0;
        self.memory.clear();
        self.performance_stats = PerformanceStats::default();
        self.elf_generator = ElfGenerator::new(self.config.arch_width);
    }

    // ゲッター
    pub fn get_config(&self) -> &ExtendedSimConfig { &self.config }
    pub fn get_riscv_cpu(&self) -> &RiscVCpu { &self.riscv_cpu }
    pub fn get_performance_stats(&self) -> &PerformanceStats { &self.performance_stats }
    pub fn get_step_count(&self) -> u64 { self.step_count }
}

/// 従来のComputerSimulationクラス（後方互換性のため）
#[derive(Debug)]
pub struct ComputerSimulation {
    extended_sim: ExtendedComputerSimulation,
}

impl ComputerSimulation {
    pub fn new() -> Self {
        let config = ExtendedSimConfig::default();
        Self {
            extended_sim: ExtendedComputerSimulation::new(config),
        }
    }

    pub fn run(&mut self, steps: Option<u64>) {
        if let Some(max_steps) = steps {
            self.extended_sim.config.max_cycles = Some(max_steps);
        }
        
        self.extended_sim.run().unwrap_or_else(|e| {
            println!("Simulation error: {e}");
        });
    }

    pub fn print_status(&self) {
        let stats = self.extended_sim.get_performance_stats();
        println!("=== Simulation Status ===");
        println!("Steps: {}", self.extended_sim.get_step_count());
        println!("CPU Cycles: {}", stats.total_cycles);
        println!("Instructions: {}", stats.total_instructions);
        println!("PC: 0x{:016X}", self.extended_sim.get_riscv_cpu().get_pc());
        println!("CPU State: {:?}", self.extended_sim.get_riscv_cpu().get_state());
    }    pub fn add_wire(&mut self, _name: String) {
        // 後方互換性のため、何もしない
    }

    pub fn get_dram_mut(&mut self) -> &mut DramController {
        &mut self.extended_sim.dram_controller
    }
}
