/// QEMU連携とエミュレーション相互運用
use crate::riscv::{ArchWidth, RiscVInstruction};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::io::Write;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// QEMU連携の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QemuConfig {
    pub qemu_path: String,
    pub machine_type: String,
    pub cpu_type: String,
    pub memory_size: String,
    pub kernel_path: Option<String>,
    pub bios_path: Option<String>,
    pub enable_gdb: bool,
    pub gdb_port: u16,
    pub enable_monitor: bool,
    pub monitor_port: u16,
}

impl Default for QemuConfig {
    fn default() -> Self {
        Self {
            qemu_path: "qemu-system-riscv64".to_string(),
            machine_type: "virt".to_string(),
            cpu_type: "rv64".to_string(),
            memory_size: "128M".to_string(),
            kernel_path: None,
            bios_path: None,
            enable_gdb: true,
            gdb_port: 1234,
            enable_monitor: true,
            monitor_port: 55555,
        }
    }
}

/// QEMU GDB Remote Serial Protocol (RSP) 実装
#[derive(Debug)]
pub struct QemuGdbStub {
    config: QemuConfig,
    connected: bool,
    breakpoints: HashMap<u64, bool>, // アドレス -> 有効/無効
}

impl QemuGdbStub {
    pub fn new(config: QemuConfig) -> Self {
        Self {
            config,
            connected: false,
            breakpoints: HashMap::new(),
        }
    }

    /// QEMU起動
    pub fn start_qemu(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::new(&self.config.qemu_path);
        
        cmd.arg("-machine").arg(&self.config.machine_type)
           .arg("-cpu").arg(&self.config.cpu_type)
           .arg("-m").arg(&self.config.memory_size)
           .arg("-nographic")
           .arg("-serial").arg("stdio");
        
        if self.config.enable_gdb {
            cmd.arg("-gdb").arg(format!("tcp::{}", self.config.gdb_port));
            cmd.arg("-S"); // 起動時に停止
        }
        
        if self.config.enable_monitor {
            cmd.arg("-monitor").arg(format!("tcp::{},server,nowait", self.config.monitor_port));
        }
        
        if let Some(ref kernel) = self.config.kernel_path {
            cmd.arg("-kernel").arg(kernel);
        }
        
        if let Some(ref bios) = self.config.bios_path {
            cmd.arg("-bios").arg(bios);
        }
        
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .spawn()?;
        
        println!("QEMU started with GDB on port {}", self.config.gdb_port);
        Ok(())
    }

    /// ブレークポイント設定
    pub fn set_breakpoint(&mut self, address: u64) {
        self.breakpoints.insert(address, true);
        println!("Breakpoint set at 0x{address:016X}");
    }

    /// ブレークポイント削除
    pub fn remove_breakpoint(&mut self, address: u64) {
        self.breakpoints.remove(&address);
        println!("Breakpoint removed from 0x{address:016X}");
    }

    /// 実行継続
    pub fn continue_execution(&self) -> Result<(), Box<dyn std::error::Error>> {
        // GDB RSPコマンドの送信（実装簡略化）
        println!("Continuing execution...");
        Ok(())
    }

    /// 単一ステップ実行
    pub fn step_execution(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Single step execution...");
        Ok(())
    }

    /// レジスタ読み取り
    pub fn read_registers(&self) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
        // 実際の実装ではGDB RSPでレジスタを読み取り
        let mut registers = HashMap::new();
        registers.insert("pc".to_string(), 0x80000000);
        registers.insert("sp".to_string(), 0x80100000);
        Ok(registers)
    }

    /// メモリ読み取り
    pub fn read_memory(&self, address: u64, size: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // 実際の実装ではGDB RSPでメモリを読み取り
        println!("Reading {size} bytes from 0x{address:016X}");
        Ok(vec![0; size])
    }

    /// メモリ書き込み
    pub fn write_memory(&self, address: u64, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        println!("Writing {} bytes to 0x{:016X}", data.len(), address);
        Ok(())
    }
}

/// バイナリファイル生成（ELF形式）
#[derive(Debug)]
pub struct ElfGenerator {
    arch_width: ArchWidth,
    instructions: Vec<u32>,
    symbols: HashMap<String, u64>,
    entry_point: u64,
}

impl ElfGenerator {
    pub fn new(arch_width: ArchWidth) -> Self {
        Self {
            arch_width,
            instructions: Vec::new(),
            symbols: HashMap::new(),
            entry_point: 0x80000000, // RISC-V標準開始アドレス
        }
    }

    /// 命令を追加
    pub fn add_instruction(&mut self, instruction: RiscVInstruction) -> Result<(), String> {
        if !instruction.is_valid_for_arch(self.arch_width) {
            return Err(format!("Instruction {:?} not valid for {}", instruction, self.arch_width));
        }
        
        let encoded = self.encode_instruction(&instruction)?;
        self.instructions.push(encoded);
        Ok(())
    }

    /// 命令をエンコード（簡略化実装）
    fn encode_instruction(&self, instruction: &RiscVInstruction) -> Result<u32, String> {
        match instruction {
            RiscVInstruction::Add(rd, rs1, rs2) => {
                // R-type: add rd, rs1, rs2
                // 31-25: funct7 (0000000), 24-20: rs2, 19-15: rs1, 14-12: funct3 (000), 11-7: rd, 6-0: opcode (0110011)
                Ok(0x00000033 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20))
            }
            RiscVInstruction::Addi(rd, rs1, imm) => {
                // I-type: addi rd, rs1, imm
                // 31-20: imm[11:0], 19-15: rs1, 14-12: funct3 (000), 11-7: rd, 6-0: opcode (0010011)
                let imm_masked = (*imm as u32) & 0xFFF;
                Ok(0x00000013 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | (imm_masked << 20))
            }
            RiscVInstruction::Lw(rd, rs1, imm) => {
                // I-type: lw rd, imm(rs1)
                // 31-20: imm[11:0], 19-15: rs1, 14-12: funct3 (010), 11-7: rd, 6-0: opcode (0000011)
                let imm_masked = (*imm as u32) & 0xFFF;
                Ok(0x00002003 | ((*rd as u32) << 7) | ((*rs1 as u32) << 15) | (imm_masked << 20))
            }
            RiscVInstruction::Sw(rs2, rs1, imm) => {
                // S-type: sw rs2, imm(rs1)
                // 31-25: imm[11:5], 24-20: rs2, 19-15: rs1, 14-12: funct3 (010), 11-7: imm[4:0], 6-0: opcode (0100011)
                let imm_val = *imm as u32;
                let imm_low = imm_val & 0x1F;
                let imm_high = (imm_val >> 5) & 0x7F;
                Ok(0x00002023 | (imm_low << 7) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20) | (imm_high << 25))
            }
            RiscVInstruction::Beq(rs1, rs2, imm) => {
                // B-type: beq rs1, rs2, imm
                let imm_val = *imm as u32;
                let imm_11 = (imm_val >> 11) & 1;
                let imm_4_1 = (imm_val >> 1) & 0xF;
                let imm_10_5 = (imm_val >> 5) & 0x3F;
                let imm_12 = (imm_val >> 12) & 1;
                Ok(0x00000063 | (imm_11 << 7) | (imm_4_1 << 8) | ((*rs1 as u32) << 15) | ((*rs2 as u32) << 20) | (imm_10_5 << 25) | (imm_12 << 31))
            }
            RiscVInstruction::Jal(rd, imm) => {
                // J-type: jal rd, imm
                let imm_val = *imm as u32;
                let imm_19_12 = (imm_val >> 12) & 0xFF;
                let imm_11 = (imm_val >> 11) & 1;
                let imm_10_1 = (imm_val >> 1) & 0x3FF;
                let imm_20 = (imm_val >> 20) & 1;
                Ok(0x0000006F | ((*rd as u32) << 7) | (imm_19_12 << 12) | (imm_11 << 20) | (imm_10_1 << 21) | (imm_20 << 31))
            }
            RiscVInstruction::Nop => {
                // addi x0, x0, 0
                Ok(0x00000013)
            }
            RiscVInstruction::Halt => {
                // ebreak
                Ok(0x00100073)
            }
            _ => Err(format!("Instruction encoding not implemented for {instruction:?}")),
        }
    }

    /// シンボル追加
    pub fn add_symbol(&mut self, name: String, address: u64) {
        self.symbols.insert(name, address);
    }

    /// バイナリファイル生成
    pub fn generate_binary(&self, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(output_path)?;
        
        // 簡単なバイナリ形式（実際のELFではない）
        for &instruction in &self.instructions {
            file.write_all(&instruction.to_le_bytes())?;
        }
        
        println!("Binary generated: {} ({} instructions)", output_path.display(), self.instructions.len());
        Ok(())
    }

    /// アセンブリリスト生成
    pub fn generate_assembly_listing(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Assembly Listing ({})\n", self.arch_width));
        output.push_str("==================================\n");
        
        for (i, &instruction) in self.instructions.iter().enumerate() {
            let address = self.entry_point + (i as u64 * 4);
            output.push_str(&format!("{address:08X}: {instruction:08X}  ; Instruction {i}\n"));
        }
        
        if !self.symbols.is_empty() {
            output.push_str("\nSymbol Table:\n");
            for (name, address) in &self.symbols {
                output.push_str(&format!("  {name}: 0x{address:016X}\n"));
            }
        }
        
        output
    }

    pub fn get_instruction_count(&self) -> usize {
        self.instructions.len()
    }

    pub fn get_entry_point(&self) -> u64 {
        self.entry_point
    }

    pub fn set_entry_point(&mut self, address: u64) {
        self.entry_point = address;
    }
}

/// QEMU統合テストランナー
pub struct QemuTestRunner {
    config: QemuConfig,
    test_programs: Vec<String>,
}

impl QemuTestRunner {
    pub fn new(config: QemuConfig) -> Self {
        Self {
            config,
            test_programs: Vec::new(),
        }
    }

    /// テストプログラム追加
    pub fn add_test_program(&mut self, program_path: String) {
        self.test_programs.push(program_path);
    }

    /// 全テスト実行
    pub fn run_all_tests(&self) -> Result<Vec<TestResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        for program in &self.test_programs {
            println!("Running test: {program}");
            let result = self.run_single_test(program)?;
            results.push(result);
        }
        
        Ok(results)
    }

    fn run_single_test(&self, program_path: &str) -> Result<TestResult, Box<dyn std::error::Error>> {
        // QEMUでテストプログラムを実行
        let mut cmd = Command::new(&self.config.qemu_path);
        cmd.arg("-machine").arg(&self.config.machine_type)
           .arg("-cpu").arg(&self.config.cpu_type)
           .arg("-m").arg(&self.config.memory_size)
           .arg("-nographic")
           .arg("-kernel").arg(program_path)
           .arg("-serial").arg("stdio");
        
        let output = cmd.output()?;
        
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(TestResult {
            program_path: program_path.to_string(),
            success,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            execution_time_ms: 0, // 実際の実装では時間計測
        })
    }

    /// テスト結果のレポート生成
    pub fn generate_report(&self, results: &[TestResult]) -> String {
        let mut report = String::new();
        report.push_str("QEMU Test Report\n");
        report.push_str("================\n");
        
        let passed = results.iter().filter(|r| r.success).count();
        let total = results.len();
        
        report.push_str(&format!("Total tests: {total}\n"));
        report.push_str(&format!("Passed: {passed}\n"));
        report.push_str(&format!("Failed: {}\n", total - passed));
        report.push_str(&format!("Success rate: {:.1}%\n", (passed as f64 / total as f64) * 100.0));
        
        report.push_str("\nTest Details:\n");
        for result in results {
            let status = if result.success { "PASS" } else { "FAIL" };
            report.push_str(&format!("  [{}] {}\n", status, result.program_path));
            
            if !result.success && !result.stderr.is_empty() {
                report.push_str(&format!("    Error: {}\n", result.stderr.trim()));
            }
        }
        
        report
    }
}

/// テスト結果
#[derive(Debug, Clone)]
pub struct TestResult {
    pub program_path: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_generator() {
        let mut generator = ElfGenerator::new(ArchWidth::Bit32);
        
        generator.add_instruction(RiscVInstruction::Addi(1, 0, 42)).unwrap();
        generator.add_instruction(RiscVInstruction::Add(2, 1, 1)).unwrap();
        generator.add_instruction(RiscVInstruction::Halt).unwrap();
        
        assert_eq!(generator.get_instruction_count(), 3);
        
        let listing = generator.generate_assembly_listing();
        assert!(listing.contains("Assembly Listing"));
    }

    #[test]
    fn test_qemu_config() {
        let config = QemuConfig::default();
        assert_eq!(config.machine_type, "virt");
        assert_eq!(config.gdb_port, 1234);
    }

    #[test]
    fn test_instruction_encoding() {
        let generator = ElfGenerator::new(ArchWidth::Bit32);
        
        // add x1, x2, x3
        let encoded = generator.encode_instruction(&RiscVInstruction::Add(1, 2, 3)).unwrap();
        assert_eq!(encoded, 0x003100B3);
        
        // addi x1, x2, 100
        let encoded = generator.encode_instruction(&RiscVInstruction::Addi(1, 2, 100)).unwrap();
        assert_eq!(encoded, 0x06410093);
    }
}
