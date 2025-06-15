/// 基本論理ゲートの実装
/// 真偽値とクロックベースの動作をサポート

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicLevel {
    Low,
    High,
    HighZ,   // ハイインピーダンス状態
    Unknown, // 未定義状態
}

impl LogicLevel {
    pub fn to_bool(self) -> Option<bool> {
        match self {
            LogicLevel::Low => Some(false),
            LogicLevel::High => Some(true),
            _ => None,
        }
    }

    pub fn from_bool(value: bool) -> Self {
        if value {
            LogicLevel::High
        } else {
            LogicLevel::Low
        }
    }
}

/// 基本論理ゲートの実装
pub struct LogicGates;

impl LogicGates {
    pub fn and(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        match (a.to_bool(), b.to_bool()) {
            (Some(true), Some(true)) => LogicLevel::High,
            (Some(false), _) | (_, Some(false)) => LogicLevel::Low,
            _ => LogicLevel::Unknown,
        }
    }

    pub fn or(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        match (a.to_bool(), b.to_bool()) {
            (Some(true), _) | (_, Some(true)) => LogicLevel::High,
            (Some(false), Some(false)) => LogicLevel::Low,
            _ => LogicLevel::Unknown,
        }
    }

    pub fn not(a: LogicLevel) -> LogicLevel {
        match a.to_bool() {
            Some(true) => LogicLevel::Low,
            Some(false) => LogicLevel::High,
            _ => LogicLevel::Unknown,
        }
    }

    pub fn nand(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        Self::not(Self::and(a, b))
    }

    pub fn nor(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        Self::not(Self::or(a, b))
    }

    pub fn xor(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        match (a.to_bool(), b.to_bool()) {
            (Some(a_val), Some(b_val)) => LogicLevel::from_bool(a_val != b_val),
            _ => LogicLevel::Unknown,
        }
    }

    pub fn xnor(a: LogicLevel, b: LogicLevel) -> LogicLevel {
        Self::not(Self::xor(a, b))
    }
}

/// 複合論理回路要素
pub struct CompositeGates;

impl CompositeGates {
    /// 半加算器 (Half Adder)
    pub fn half_adder(a: LogicLevel, b: LogicLevel) -> (LogicLevel, LogicLevel) {
        let sum = LogicGates::xor(a, b);
        let carry = LogicGates::and(a, b);
        (sum, carry)
    }

    /// 全加算器 (Full Adder)
    pub fn full_adder(
        a: LogicLevel,
        b: LogicLevel,
        carry_in: LogicLevel,
    ) -> (LogicLevel, LogicLevel) {
        let (sum1, carry1) = Self::half_adder(a, b);
        let (sum, carry2) = Self::half_adder(sum1, carry_in);
        let carry_out = LogicGates::or(carry1, carry2);
        (sum, carry_out)
    }

    /// DフリップフロップのSRラッチ部分
    pub fn sr_latch(set: LogicLevel, reset: LogicLevel) -> (LogicLevel, LogicLevel) {
        // 簡単なSRラッチの実装（実用的には状態を保持する必要がある）
        let q = LogicGates::nor(reset, LogicLevel::Low);
        let q_not = LogicGates::nor(set, q);
        (q, q_not)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_gates() {
        assert_eq!(
            LogicGates::and(LogicLevel::High, LogicLevel::High),
            LogicLevel::High
        );
        assert_eq!(
            LogicGates::and(LogicLevel::High, LogicLevel::Low),
            LogicLevel::Low
        );
        assert_eq!(
            LogicGates::or(LogicLevel::Low, LogicLevel::High),
            LogicLevel::High
        );
        assert_eq!(LogicGates::not(LogicLevel::High), LogicLevel::Low);
    }

    #[test]
    fn test_composite_gates() {
        let (sum, carry) = CompositeGates::half_adder(LogicLevel::High, LogicLevel::High);
        assert_eq!(sum, LogicLevel::Low);
        assert_eq!(carry, LogicLevel::High);
    }
}
