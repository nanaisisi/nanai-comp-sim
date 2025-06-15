/// RISC-V アーキテクチャ定義とビット幅対応
use serde::{Deserialize, Serialize};
use std::fmt;

/// サポートするアーキテクチャビット幅
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchWidth {
    Bit8,
    Bit16,
    Bit32,
    Bit64,
}

impl ArchWidth {
    pub fn bytes(&self) -> usize {
        match self {
            ArchWidth::Bit8 => 1,
            ArchWidth::Bit16 => 2,
            ArchWidth::Bit32 => 4,
            ArchWidth::Bit64 => 8,
        }
    }

    pub fn bits(&self) -> usize {
        self.bytes() * 8
    }

    pub fn max_value(&self) -> u128 {
        match self {
            ArchWidth::Bit8 => u8::MAX as u128,
            ArchWidth::Bit16 => u16::MAX as u128,
            ArchWidth::Bit32 => u32::MAX as u128,
            ArchWidth::Bit64 => u64::MAX as u128,
        }
    }
}

impl fmt::Display for ArchWidth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ArchWidth::Bit8 => write!(f, "8-bit"),
            ArchWidth::Bit16 => write!(f, "16-bit"),
            ArchWidth::Bit32 => write!(f, "32-bit"),
            ArchWidth::Bit64 => write!(f, "64-bit"),
        }
    }
}

/// RISC-V 命令フォーマット
#[derive(Debug, Clone, PartialEq)]
pub enum RiscVInstFormat {
    R {
        rd: u8,
        rs1: u8,
        rs2: u8,
        funct3: u8,
        funct7: u8,
    },
    I {
        rd: u8,
        rs1: u8,
        imm: i32,
        funct3: u8,
    },
    S {
        rs1: u8,
        rs2: u8,
        imm: i32,
        funct3: u8,
    },
    B {
        rs1: u8,
        rs2: u8,
        imm: i32,
        funct3: u8,
    },
    U {
        rd: u8,
        imm: i32,
    },
    J {
        rd: u8,
        imm: i32,
    },
}

/// RISC-V 基本命令セット (RV32I/RV64I)
#[derive(Debug, Clone, PartialEq)]
pub enum RiscVInstruction {
    // 算術演算
    Add(u8, u8, u8),   // add rd, rs1, rs2
    Sub(u8, u8, u8),   // sub rd, rs1, rs2
    Addi(u8, u8, i32), // addi rd, rs1, imm

    // 論理演算
    And(u8, u8, u8),   // and rd, rs1, rs2
    Or(u8, u8, u8),    // or rd, rs1, rs2
    Xor(u8, u8, u8),   // xor rd, rs1, rs2
    Andi(u8, u8, i32), // andi rd, rs1, imm
    Ori(u8, u8, i32),  // ori rd, rs1, imm
    Xori(u8, u8, i32), // xori rd, rs1, imm

    // シフト
    Sll(u8, u8, u8),  // sll rd, rs1, rs2
    Srl(u8, u8, u8),  // srl rd, rs1, rs2
    Sra(u8, u8, u8),  // sra rd, rs1, rs2
    Slli(u8, u8, u8), // slli rd, rs1, shamt
    Srli(u8, u8, u8), // srli rd, rs1, shamt
    Srai(u8, u8, u8), // srai rd, rs1, shamt

    // 比較
    Slt(u8, u8, u8),    // slt rd, rs1, rs2
    Sltu(u8, u8, u8),   // sltu rd, rs1, rs2
    Slti(u8, u8, i32),  // slti rd, rs1, imm
    Sltiu(u8, u8, i32), // sltiu rd, rs1, imm

    // メモリアクセス
    Lb(u8, u8, i32),  // lb rd, offset(rs1)
    Lh(u8, u8, i32),  // lh rd, offset(rs1)
    Lw(u8, u8, i32),  // lw rd, offset(rs1)
    Ld(u8, u8, i32),  // ld rd, offset(rs1) - RV64I only
    Lbu(u8, u8, i32), // lbu rd, offset(rs1)
    Lhu(u8, u8, i32), // lhu rd, offset(rs1)
    Lwu(u8, u8, i32), // lwu rd, offset(rs1) - RV64I only
    Sb(u8, u8, i32),  // sb rs2, offset(rs1)
    Sh(u8, u8, i32),  // sh rs2, offset(rs1)
    Sw(u8, u8, i32),  // sw rs2, offset(rs1)
    Sd(u8, u8, i32),  // sd rs2, offset(rs1) - RV64I only

    // 分岐
    Beq(u8, u8, i32),  // beq rs1, rs2, offset
    Bne(u8, u8, i32),  // bne rs1, rs2, offset
    Blt(u8, u8, i32),  // blt rs1, rs2, offset
    Bge(u8, u8, i32),  // bge rs1, rs2, offset
    Bltu(u8, u8, i32), // bltu rs1, rs2, offset
    Bgeu(u8, u8, i32), // bgeu rs1, rs2, offset

    // ジャンプ
    Jal(u8, i32),      // jal rd, offset
    Jalr(u8, u8, i32), // jalr rd, rs1, offset

    // 上位即値ロード
    Lui(u8, i32),   // lui rd, imm
    Auipc(u8, i32), // auipc rd, imm

    // システム・環境呼び出し
    Ecall,  // ecall
    Ebreak, // ebreak

    // Fence (メモリ順序制御)
    Fence(u8, u8), // fence pred, succ

    // CSR (Control and Status Register)
    Csrrw(u8, u16, u8),  // csrrw rd, csr, rs1
    Csrrs(u8, u16, u8),  // csrrs rd, csr, rs1
    Csrrc(u8, u16, u8),  // csrrc rd, csr, rs1
    Csrrwi(u8, u16, u8), // csrrwi rd, csr, imm
    Csrrsi(u8, u16, u8), // csrrsi rd, csr, imm
    Csrrci(u8, u16, u8), // csrrci rd, csr, imm

    // 疑似命令
    Nop,         // nop (実際は addi x0, x0, 0)
    Li(u8, i64), // li rd, imm (疑似命令)
    Mv(u8, u8),  // mv rd, rs (実際は addi rd, rs, 0)

    // ハルト
    Halt,
}

impl RiscVInstruction {
    /// 命令がアーキテクチャ幅で有効かチェック
    pub fn is_valid_for_arch(&self, arch: ArchWidth) -> bool {
        match self {
            // RV64I専用命令
            RiscVInstruction::Ld(_, _, _)
            | RiscVInstruction::Lwu(_, _, _)
            | RiscVInstruction::Sd(_, _, _) => arch == ArchWidth::Bit64,

            // RV32I以上で有効
            RiscVInstruction::Lw(_, _, _) | RiscVInstruction::Sw(_, _, _) => arch.bits() >= 32,

            // RV16以上で有効
            RiscVInstruction::Lh(_, _, _)
            | RiscVInstruction::Lhu(_, _, _)
            | RiscVInstruction::Sh(_, _, _) => arch.bits() >= 16,

            // 全アーキテクチャで有効
            _ => true,
        }
    }

    /// 実行サイクル数の見積もり
    pub fn estimated_cycles(&self) -> u32 {
        match self {
            // メモリアクセス命令は複数サイクル
            RiscVInstruction::Lb(_, _, _)
            | RiscVInstruction::Lh(_, _, _)
            | RiscVInstruction::Lw(_, _, _)
            | RiscVInstruction::Ld(_, _, _)
            | RiscVInstruction::Lbu(_, _, _)
            | RiscVInstruction::Lhu(_, _, _)
            | RiscVInstruction::Lwu(_, _, _) => 3,

            RiscVInstruction::Sb(_, _, _)
            | RiscVInstruction::Sh(_, _, _)
            | RiscVInstruction::Sw(_, _, _)
            | RiscVInstruction::Sd(_, _, _) => 2,

            // 乗除算は複数サイクル（拡張で実装予定）

            // 分岐・ジャンプはパイプライン影響
            RiscVInstruction::Beq(_, _, _)
            | RiscVInstruction::Bne(_, _, _)
            | RiscVInstruction::Blt(_, _, _)
            | RiscVInstruction::Bge(_, _, _)
            | RiscVInstruction::Bltu(_, _, _)
            | RiscVInstruction::Bgeu(_, _, _)
            | RiscVInstruction::Jal(_, _)
            | RiscVInstruction::Jalr(_, _, _) => 2,

            // システム呼び出し
            RiscVInstruction::Ecall | RiscVInstruction::Ebreak => 10,

            // その他の算術・論理演算は1サイクル
            _ => 1,
        }
    }
}

/// RISC-V レジスタファイル
#[derive(Debug, Clone)]
pub struct RiscVRegisterFile {
    arch_width: ArchWidth,
    registers: Vec<u64>, // 最大64bitまで対応、下位ビットのみ使用
}

impl RiscVRegisterFile {
    pub fn new(arch_width: ArchWidth) -> Self {
        Self {
            arch_width,
            registers: vec![0; 32], // x0-x31
        }
    }

    pub fn read(&self, reg: u8) -> u64 {
        if reg == 0 {
            0 // x0は常に0
        } else if (reg as usize) < self.registers.len() {
            self.mask_value(self.registers[reg as usize])
        } else {
            0
        }
    }

    pub fn write(&mut self, reg: u8, value: u64) {
        if reg != 0 && (reg as usize) < self.registers.len() {
            self.registers[reg as usize] = self.mask_value(value);
        }
    }

    fn mask_value(&self, value: u64) -> u64 {
        match self.arch_width {
            ArchWidth::Bit8 => value & 0xFF,
            ArchWidth::Bit16 => value & 0xFFFF,
            ArchWidth::Bit32 => value & 0xFFFFFFFF,
            ArchWidth::Bit64 => value,
        }
    }

    pub fn get_arch_width(&self) -> ArchWidth {
        self.arch_width
    }

    /// デバッグ用：全レジスタの表示
    pub fn dump_registers(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("RISC-V {} Register File:\n", self.arch_width));

        for i in 0..32 {
            let reg_name = match i {
                0 => "zero",
                1 => "ra",
                2 => "sp",
                3 => "gp",
                4 => "tp",
                5..=7 => "t0-2",
                8..=9 => "s0-1",
                10..=17 => "a0-7",
                18..=27 => "s2-11",
                28..=31 => "t3-6",
                _ => "unknown",
            };

            match self.arch_width {
                ArchWidth::Bit8 => output.push_str(&format!(
                    "x{:2} ({:>4}): 0x{:02X}\n",
                    i,
                    reg_name,
                    self.read(i) as u8
                )),
                ArchWidth::Bit16 => output.push_str(&format!(
                    "x{:2} ({:>4}): 0x{:04X}\n",
                    i,
                    reg_name,
                    self.read(i) as u16
                )),
                ArchWidth::Bit32 => output.push_str(&format!(
                    "x{:2} ({:>4}): 0x{:08X}\n",
                    i,
                    reg_name,
                    self.read(i) as u32
                )),
                ArchWidth::Bit64 => output.push_str(&format!(
                    "x{:2} ({:>4}): 0x{:016X}\n",
                    i,
                    reg_name,
                    self.read(i)
                )),
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_width() {
        assert_eq!(ArchWidth::Bit32.bytes(), 4);
        assert_eq!(ArchWidth::Bit64.bits(), 64);
        assert_eq!(ArchWidth::Bit32.max_value(), u32::MAX as u128);
    }

    #[test]
    fn test_register_file() {
        let mut rf = RiscVRegisterFile::new(ArchWidth::Bit32);

        // x0は常に0
        rf.write(0, 0xFFFFFFFF);
        assert_eq!(rf.read(0), 0);

        // 通常のレジスタ
        rf.write(1, 0x12345678);
        assert_eq!(rf.read(1), 0x12345678);

        // 8bitアーキテクチャではマスク
        let mut rf8 = RiscVRegisterFile::new(ArchWidth::Bit8);
        rf8.write(1, 0x12345678);
        assert_eq!(rf8.read(1), 0x78);
    }

    #[test]
    fn test_instruction_validity() {
        let inst_32 = RiscVInstruction::Lw(1, 2, 0);
        let inst_64 = RiscVInstruction::Ld(1, 2, 0);

        assert!(inst_32.is_valid_for_arch(ArchWidth::Bit32));
        assert!(!inst_64.is_valid_for_arch(ArchWidth::Bit32));
        assert!(inst_64.is_valid_for_arch(ArchWidth::Bit64));
    }
}
