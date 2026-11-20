use crate::clock::{ClockEdge, Clocked};
use crate::logic_gates::LogicLevel;
/// RISC-V CPU シミュレーター - 複数ビット幅対応
use crate::riscv::{ArchWidth, RiscVInstruction, RiscVRegisterFile};
use crate::waveform::{WaveformRecorder, WaveformValue};
use std::collections::HashMap;

/// CPU実行状態
#[derive(Debug, Clone, PartialEq)]
pub enum CpuState {
    Reset,
    Fetch,
    Decode,
    Execute,
    Writeback,
    Halt,
    Exception(ExceptionType),
}

/// 例外タイプ
#[derive(Debug, Clone, PartialEq)]
pub enum ExceptionType {
    IllegalInstruction,
    AccessFault,
    Breakpoint,
    EnvironmentCall,
    Halt,
}

/// メモリアクセスタイプ
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAccessType {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

/// RISC-V CPU コア
#[derive(Debug)]
pub struct RiscVCpu {
    arch_width: ArchWidth,
    registers: RiscVRegisterFile,
    pc: u64,

    // パイプライン段階
    state: CpuState,
    current_instruction: Option<RiscVInstruction>,
    instruction_word: u32,

    // 制御・状態レジスタ (CSR)
    csr: HashMap<u16, u64>,

    // パフォーマンスカウンタ
    cycle_count: u64,
    instruction_count: u64,

    // 例外・割り込み
    pending_exception: Option<ExceptionType>,

    // メモリインターフェース
    memory_address: u64,
    memory_data_read: u64,
    memory_data_write: u64,
    memory_write_enable: bool,
    memory_access_type: MemoryAccessType,

    // デバッグ・トレース
    trace_enabled: bool,
    trace_buffer: Vec<TraceEntry>,
}

/// 実行トレースエントリ
#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub cycle: u64,
    pub pc: u64,
    pub instruction: RiscVInstruction,
    pub registers_before: Vec<u64>,
    pub registers_after: Vec<u64>,
}

impl RiscVCpu {
    pub fn new(arch_width: ArchWidth) -> Self {
        let mut csr = HashMap::new();

        // 基本CSR初期化
        csr.insert(0x300, 0); // mstatus
        csr.insert(0x305, 0); // mtvec
        csr.insert(0x340, 0); // mscratch
        csr.insert(0x341, 0); // mepc
        csr.insert(0x342, 0); // mcause
        csr.insert(0x343, 0); // mtval
        csr.insert(0x344, 0); // mip

        Self {
            arch_width,
            registers: RiscVRegisterFile::new(arch_width),
            pc: 0x80000000, // RISC-V標準開始アドレス
            state: CpuState::Reset,
            current_instruction: None,
            instruction_word: 0,
            csr,
            cycle_count: 0,
            instruction_count: 0,
            pending_exception: None,
            memory_address: 0,
            memory_data_read: 0,
            memory_data_write: 0,
            memory_write_enable: false,
            memory_access_type: MemoryAccessType::Word,
            trace_enabled: false,
            trace_buffer: Vec::new(),
        }
    }

    /// リセット
    pub fn reset(&mut self) {
        self.registers = RiscVRegisterFile::new(self.arch_width);
        self.pc = 0x80000000;
        self.state = CpuState::Reset;
        self.current_instruction = None;
        self.cycle_count = 0;
        self.instruction_count = 0;
        self.pending_exception = None;
        self.memory_write_enable = false;

        // CSRリセット
        for (_, value) in self.csr.iter_mut() {
            *value = 0;
        }

        println!("RISC-V CPU ({}) reset", self.arch_width);
    }

    /// クロックサイクル実行
    pub fn clock_cycle(&mut self, memory_data: u32) {
        self.cycle_count += 1;

        match self.state {
            CpuState::Reset => {
                self.state = CpuState::Fetch;
            }
            CpuState::Fetch => {
                self.fetch_instruction(memory_data);
            }
            CpuState::Decode => {
                self.decode_instruction();
            }
            CpuState::Execute => {
                self.execute_instruction();
            }
            CpuState::Writeback => {
                self.writeback();
            }
            CpuState::Halt => {
                // 停止状態
            }
            CpuState::Exception(ref exc_type) => {
                self.handle_exception(exc_type.clone());
            }
        }
    }

    /// 命令フェッチ
    fn fetch_instruction(&mut self, memory_data: u32) {
        self.memory_address = self.pc;
        self.memory_write_enable = false;
        self.memory_access_type = MemoryAccessType::Word;

        self.instruction_word = memory_data;
        self.state = CpuState::Decode;
    }

    /// 命令デコード
    fn decode_instruction(&mut self) {
        self.current_instruction = self.decode_instruction_word(self.instruction_word);

        if self.current_instruction.is_none() {
            self.pending_exception = Some(ExceptionType::IllegalInstruction);
            self.state = CpuState::Exception(ExceptionType::IllegalInstruction);
        } else {
            self.state = CpuState::Execute;
        }
    }

    /// 命令実行
    fn execute_instruction(&mut self) {
        if let Some(ref instruction) = self.current_instruction {
            if self.trace_enabled {
                let regs_before: Vec<u64> = (0..32).map(|i| self.registers.read(i)).collect();

                self.trace_buffer.push(TraceEntry {
                    cycle: self.cycle_count,
                    pc: self.pc,
                    instruction: instruction.clone(),
                    registers_before: regs_before,
                    registers_after: Vec::new(), // 実行後に更新
                });
            }

            match instruction {
                RiscVInstruction::Add(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    let result = val1.wrapping_add(val2);
                    self.registers.write(*rd, result);
                }
                RiscVInstruction::Sub(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    let result = val1.wrapping_sub(val2);
                    self.registers.write(*rd, result);
                }
                RiscVInstruction::Addi(rd, rs1, imm) => {
                    let val = self.registers.read(*rs1);
                    let result = val.wrapping_add(*imm as i64 as u64);
                    self.registers.write(*rd, result);
                }
                RiscVInstruction::And(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    self.registers.write(*rd, val1 & val2);
                }
                RiscVInstruction::Or(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    self.registers.write(*rd, val1 | val2);
                }
                RiscVInstruction::Xor(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    self.registers.write(*rd, val1 ^ val2);
                }
                RiscVInstruction::Sll(rd, rs1, rs2) => {
                    let val = self.registers.read(*rs1);
                    let shift = self.registers.read(*rs2) & 0x3F; // 6ビットマスク
                    self.registers.write(*rd, val << shift);
                }
                RiscVInstruction::Srl(rd, rs1, rs2) => {
                    let val = self.registers.read(*rs1);
                    let shift = self.registers.read(*rs2) & 0x3F;
                    self.registers.write(*rd, val >> shift);
                }
                RiscVInstruction::Slt(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1) as i64;
                    let val2 = self.registers.read(*rs2) as i64;
                    self.registers.write(*rd, if val1 < val2 { 1 } else { 0 });
                }
                RiscVInstruction::Sltu(rd, rs1, rs2) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    self.registers.write(*rd, if val1 < val2 { 1 } else { 0 });
                }
                RiscVInstruction::Beq(rs1, rs2, imm) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    if val1 == val2 {
                        self.pc = self.pc.wrapping_add(*imm as i64 as u64).wrapping_sub(4);
                    }
                }
                RiscVInstruction::Bne(rs1, rs2, imm) => {
                    let val1 = self.registers.read(*rs1);
                    let val2 = self.registers.read(*rs2);
                    if val1 != val2 {
                        self.pc = self.pc.wrapping_add(*imm as i64 as u64).wrapping_sub(4);
                    }
                }
                RiscVInstruction::Jal(rd, imm) => {
                    self.registers.write(*rd, self.pc + 4);
                    self.pc = self.pc.wrapping_add(*imm as i64 as u64).wrapping_sub(4);
                }
                RiscVInstruction::Jalr(rd, rs1, imm) => {
                    let target = self.registers.read(*rs1).wrapping_add(*imm as i64 as u64) & !1;
                    self.registers.write(*rd, self.pc + 4);
                    self.pc = target.wrapping_sub(4);
                }
                RiscVInstruction::Lui(rd, imm) => {
                    self.registers.write(*rd, (*imm as u64) << 12);
                }
                RiscVInstruction::Auipc(rd, imm) => {
                    let result = self.pc.wrapping_add((*imm as u64) << 12);
                    self.registers.write(*rd, result);
                }
                RiscVInstruction::Lw(_rd, rs1, imm) => {
                    self.memory_address =
                        self.registers.read(*rs1).wrapping_add(*imm as i64 as u64);
                    self.memory_access_type = MemoryAccessType::Word;
                    self.memory_write_enable = false;
                    // メモリアクセスは次のサイクルで完了
                    self.state = CpuState::Writeback;
                    return;
                }
                RiscVInstruction::Sw(rs2, rs1, imm) => {
                    self.memory_address =
                        self.registers.read(*rs1).wrapping_add(*imm as i64 as u64);
                    self.memory_data_write = self.registers.read(*rs2);
                    self.memory_access_type = MemoryAccessType::Word;
                    self.memory_write_enable = true;
                }
                RiscVInstruction::Ecall => {
                    self.pending_exception = Some(ExceptionType::EnvironmentCall);
                    self.state = CpuState::Exception(ExceptionType::EnvironmentCall);
                    return;
                }
                RiscVInstruction::Ebreak => {
                    self.pending_exception = Some(ExceptionType::Breakpoint);
                    self.state = CpuState::Exception(ExceptionType::Breakpoint);
                    return;
                }
                RiscVInstruction::Nop => {
                    // 何もしない
                }
                RiscVInstruction::Halt => {
                    self.state = CpuState::Halt;
                    return;
                }
                _ => {
                    self.pending_exception = Some(ExceptionType::IllegalInstruction);
                    self.state = CpuState::Exception(ExceptionType::IllegalInstruction);
                    return;
                }
            }

            if self.trace_enabled && !self.trace_buffer.is_empty() {
                let regs_after: Vec<u64> = (0..32).map(|i| self.registers.read(i)).collect();
                if let Some(last_entry) = self.trace_buffer.last_mut() {
                    last_entry.registers_after = regs_after;
                }
            }

            self.instruction_count += 1;
        }

        self.pc += 4;
        self.state = CpuState::Fetch;
    }

    /// ライトバック段階
    fn writeback(&mut self) {
        // メモリロード命令の結果をレジスタに書き込み
        if let Some(RiscVInstruction::Lw(rd, _, _)) = self.current_instruction {
            self.registers.write(rd, self.memory_data_read);
        }

        self.pc += 4;
        self.state = CpuState::Fetch;
    }

    /// 例外処理
    fn handle_exception(&mut self, exc_type: ExceptionType) {
        println!("Exception: {:?} at PC 0x{:016X}", exc_type, self.pc);

        // 簡単な例外処理
        match exc_type {
            ExceptionType::Halt => {
                self.state = CpuState::Halt;
            }
            _ => {
                // 例外ハンドラにジャンプ（簡略化）
                self.pc = 0x80000004;
                self.state = CpuState::Fetch;
            }
        }

        self.pending_exception = None;
    }

    /// 命令語のデコード（簡略化実装）
    fn decode_instruction_word(&self, word: u32) -> Option<RiscVInstruction> {
        let opcode = word & 0x7F;
        let rd = ((word >> 7) & 0x1F) as u8;
        let funct3 = (word >> 12) & 0x7;
        let rs1 = ((word >> 15) & 0x1F) as u8;
        let rs2 = ((word >> 20) & 0x1F) as u8;
        let funct7 = word >> 25;

        match opcode {
            0b0110011 => {
                // R-type
                match (funct3, funct7) {
                    (0, 0) => Some(RiscVInstruction::Add(rd, rs1, rs2)),
                    (0, 32) => Some(RiscVInstruction::Sub(rd, rs1, rs2)),
                    (7, 0) => Some(RiscVInstruction::And(rd, rs1, rs2)),
                    (6, 0) => Some(RiscVInstruction::Or(rd, rs1, rs2)),
                    (4, 0) => Some(RiscVInstruction::Xor(rd, rs1, rs2)),
                    (1, 0) => Some(RiscVInstruction::Sll(rd, rs1, rs2)),
                    (5, 0) => Some(RiscVInstruction::Srl(rd, rs1, rs2)),
                    (2, 0) => Some(RiscVInstruction::Slt(rd, rs1, rs2)),
                    (3, 0) => Some(RiscVInstruction::Sltu(rd, rs1, rs2)),
                    _ => None,
                }
            }
            0b0010011 => {
                // I-type
                let imm = (word as i32) >> 20;
                match funct3 {
                    0 => Some(RiscVInstruction::Addi(rd, rs1, imm)),
                    _ => None,
                }
            }
            0b0000011 => {
                // Load
                let imm = (word as i32) >> 20;
                match funct3 {
                    2 => Some(RiscVInstruction::Lw(rd, rs1, imm)),
                    _ => None,
                }
            }
            0b0100011 => {
                // Store
                let imm = (((word as i32) >> 25) << 5) | ((word >> 7) & 0x1F) as i32;
                match funct3 {
                    2 => Some(RiscVInstruction::Sw(rs2, rs1, imm)),
                    _ => None,
                }
            }
            0b1100011 => {
                // Branch
                let imm = (((word as i32) >> 31) << 12)
                    | (((word >> 7) & 1) << 11) as i32
                    | (((word >> 25) & 0x3F) << 5) as i32
                    | (((word >> 8) & 0xF) << 1) as i32;
                match funct3 {
                    0 => Some(RiscVInstruction::Beq(rs1, rs2, imm)),
                    1 => Some(RiscVInstruction::Bne(rs1, rs2, imm)),
                    _ => None,
                }
            }
            0b1101111 => {
                // JAL
                let imm = (((word as i32) >> 31) << 20)
                    | (((word >> 12) & 0xFF) << 12) as i32
                    | (((word >> 20) & 1) << 11) as i32
                    | (((word >> 21) & 0x3FF) << 1) as i32;
                Some(RiscVInstruction::Jal(rd, imm))
            }
            0b1100111 => {
                // JALR
                let imm = (word as i32) >> 20;
                Some(RiscVInstruction::Jalr(rd, rs1, imm))
            }
            0b0110111 => {
                // LUI
                let imm = (word as i32) >> 12;
                Some(RiscVInstruction::Lui(rd, imm))
            }
            0b0010111 => {
                // AUIPC
                let imm = (word as i32) >> 12;
                Some(RiscVInstruction::Auipc(rd, imm))
            }
            0b1110011 => {
                // System
                match funct3 {
                    0 => match word {
                        0x00000073 => Some(RiscVInstruction::Ecall),
                        0x00100073 => Some(RiscVInstruction::Ebreak),
                        _ => None,
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // ゲッター・セッター
    pub fn get_pc(&self) -> u64 {
        self.pc
    }
    pub fn set_pc(&mut self, pc: u64) {
        self.pc = pc;
    }

    pub fn get_register(&self, reg: u8) -> u64 {
        self.registers.read(reg)
    }
    pub fn set_register(&mut self, reg: u8, value: u64) {
        self.registers.write(reg, value);
    }

    pub fn get_state(&self) -> &CpuState {
        &self.state
    }
    pub fn get_cycle_count(&self) -> u64 {
        self.cycle_count
    }
    pub fn get_instruction_count(&self) -> u64 {
        self.instruction_count
    }

    pub fn get_memory_address(&self) -> u64 {
        self.memory_address
    }
    pub fn get_memory_data_write(&self) -> u64 {
        self.memory_data_write
    }
    pub fn is_memory_write(&self) -> bool {
        self.memory_write_enable
    }
    pub fn set_memory_data_read(&mut self, data: u64) {
        self.memory_data_read = data;
    }

    pub fn enable_trace(&mut self, enable: bool) {
        self.trace_enabled = enable;
    }
    pub fn get_trace(&self) -> &[TraceEntry] {
        &self.trace_buffer
    }
    pub fn clear_trace(&mut self) {
        self.trace_buffer.clear();
    }

    pub fn get_arch_width(&self) -> ArchWidth {
        self.arch_width
    }

    /// 波形記録
    pub fn record_waveforms(&self, recorder: &mut WaveformRecorder) {
        // PC記録
        recorder.record_integer("pc", self.pc);

        // 状態記録
        let state_str = format!("{:?}", self.state);
        recorder.record_string("cpu_state", state_str);

        // レジスタファイル記録（主要レジスタのみ）
        for i in 0..8 {
            let reg_name = format!("x{i}");
            recorder.record_integer(&reg_name, self.registers.read(i));
        }

        // メモリインターフェース記録
        recorder.record_integer("mem_addr", self.memory_address);
        recorder.record_integer("mem_data_write", self.memory_data_write);
        recorder.record_change(
            "mem_write_enable",
            WaveformValue::Logic(if self.memory_write_enable {
                LogicLevel::High
            } else {
                LogicLevel::Low
            }),
        );
    }

    /// デバッグ情報出力
    pub fn dump_state(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("RISC-V CPU ({}) State:\n", self.arch_width));
        output.push_str("========================\n");
        output.push_str(&format!("PC: 0x{:016X}\n", self.pc));
        output.push_str(&format!("State: {:?}\n", self.state));
        output.push_str(&format!("Cycles: {}\n", self.cycle_count));
        output.push_str(&format!("Instructions: {}\n", self.instruction_count));

        if let Some(ref inst) = self.current_instruction {
            output.push_str(&format!("Current Instruction: {inst:?}\n"));
        }

        output.push('\n');
        output.push_str(&self.registers.dump_registers());

        output
    }
}

impl Clocked for RiscVCpu {
    fn on_clock_edge(&mut self, edge: ClockEdge, _clock_name: &str) {
        if edge == ClockEdge::Rising {
            self.clock_cycle(0); // メモリデータは外部から供給される
        }
    }

    fn reset(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_riscv_cpu_basic() {
        let mut cpu = RiscVCpu::new(ArchWidth::Bit32);

        // 初期状態確認
        assert_eq!(cpu.get_pc(), 0x80000000);
        assert_eq!(cpu.get_register(0), 0); // x0は常に0

        // レジスタ書き込みテスト
        cpu.set_register(1, 0x12345678);
        assert_eq!(cpu.get_register(1), 0x12345678);

        // x0への書き込みは無視される
        cpu.set_register(0, 0xFFFFFFFF);
        assert_eq!(cpu.get_register(0), 0);
    }

    #[test]
    fn test_instruction_decode() {
        let cpu = RiscVCpu::new(ArchWidth::Bit32);

        // add x1, x2, x3 (0x003100B3)
        let inst = cpu.decode_instruction_word(0x003100B3);
        assert_eq!(inst, Some(RiscVInstruction::Add(1, 2, 3)));

        // addi x1, x2, 100 (0x06410093)
        let inst = cpu.decode_instruction_word(0x06410093);
        assert_eq!(inst, Some(RiscVInstruction::Addi(1, 2, 100)));
    }

    #[test]
    fn test_instruction_execution() {
        let mut cpu = RiscVCpu::new(ArchWidth::Bit64);

        // レジスタ初期化
        cpu.set_register(2, 10);
        cpu.set_register(3, 20);

        // add命令実行のシミュレーション
        cpu.current_instruction = Some(RiscVInstruction::Add(1, 2, 3));
        cpu.state = CpuState::Execute;
        cpu.execute_instruction();

        // 結果確認
        assert_eq!(cpu.get_register(1), 30);
    }
}
