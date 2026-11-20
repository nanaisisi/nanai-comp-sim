use crate::clock::ClockEdge;
/// グラフィカル波形表示とVCDファイル出力
use crate::logic_gates::LogicLevel;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// 信号の値変化記録
#[derive(Debug, Clone)]
pub struct SignalChange {
    pub timestamp_ns: u64,
    pub value: WaveformValue,
}

/// 波形表示用の値
#[derive(Debug, Clone, PartialEq)]
pub enum WaveformValue {
    Logic(LogicLevel),
    Binary(Vec<bool>), // バス信号用
    Integer(u64),      // 整数値
    Float(f64),        // アナログ信号用
    String(String),    // 文字列（状態表示など）
}

impl WaveformValue {
    pub fn from_logic_bus(levels: &[LogicLevel]) -> Self {
        let binary: Vec<bool> = levels
            .iter()
            .map(|level| match level {
                LogicLevel::High => true,
                LogicLevel::Low => false,
                _ => false, // HighZ, Unknownは0として扱う
            })
            .collect();
        WaveformValue::Binary(binary)
    }

    pub fn to_integer(&self) -> Option<u64> {
        match self {
            WaveformValue::Logic(LogicLevel::High) => Some(1),
            WaveformValue::Logic(LogicLevel::Low) => Some(0),
            WaveformValue::Binary(bits) => {
                let mut value = 0u64;
                for (i, &bit) in bits.iter().enumerate() {
                    if bit {
                        value |= 1 << i;
                    }
                }
                Some(value)
            }
            WaveformValue::Integer(val) => Some(*val),
            _ => None,
        }
    }

    pub fn to_vcd_char(&self) -> char {
        match self {
            WaveformValue::Logic(LogicLevel::High) => '1',
            WaveformValue::Logic(LogicLevel::Low) => '0',
            WaveformValue::Logic(LogicLevel::HighZ) => 'z',
            WaveformValue::Logic(LogicLevel::Unknown) => 'x',
            _ => '0',
        }
    }
}

/// 波形信号定義
#[derive(Debug, Clone)]
pub struct WaveformSignal {
    pub name: String,
    pub width: usize, // ビット幅（1=単一信号、>1=バス）
    pub changes: Vec<SignalChange>,
    pub vcd_symbol: String, // VCD用のシンボル
}

impl WaveformSignal {
    pub fn new(name: String, width: usize) -> Self {
        // VCD用の一意シンボルを生成
        let vcd_symbol = Self::generate_vcd_symbol(&name);

        Self {
            name,
            width,
            changes: Vec::new(),
            vcd_symbol,
        }
    }

    pub fn add_change(&mut self, timestamp_ns: u64, value: WaveformValue) {
        self.changes.push(SignalChange {
            timestamp_ns,
            value,
        });
    }

    fn generate_vcd_symbol(name: &str) -> String {
        // 簡単なハッシュ関数でシンボル生成
        let mut hash = 0u32;
        for byte in name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }

        // ASCII文字に変換
        let mut symbol = String::new();
        let mut val = hash;
        while val > 0 {
            let char_code = (val % 94) + 33; // 印刷可能文字の範囲
            symbol.push(char_code as u8 as char);
            val /= 94;
        }

        if symbol.is_empty() {
            symbol.push('!');
        }

        symbol
    }

    pub fn get_latest_value(&self) -> Option<&WaveformValue> {
        self.changes.last().map(|change| &change.value)
    }
}

/// 波形記録システム
#[derive(Debug)]
pub struct WaveformRecorder {
    signals: HashMap<String, WaveformSignal>,
    current_time_ns: u64,
    recording: bool,
    vcd_file: Option<BufWriter<File>>,
}

impl WaveformRecorder {
    pub fn new() -> Self {
        Self {
            signals: HashMap::new(),
            current_time_ns: 0,
            recording: false,
            vcd_file: None,
        }
    }

    /// 信号を追加
    pub fn add_signal(&mut self, name: String, width: usize) {
        self.signals
            .insert(name.clone(), WaveformSignal::new(name, width));
    }

    /// 信号値の変化を記録
    pub fn record_change(&mut self, signal_name: &str, value: WaveformValue) {
        if !self.recording {
            return;
        }
        if let Some(signal) = self.signals.get_mut(signal_name) {
            signal.add_change(self.current_time_ns, value.clone());
        }

        // VCDファイルにも記録（借用の競合を避けるため、スコープを分ける）
        if let Some(ref mut vcd_writer) = self.vcd_file
            && let Some(signal) = self.signals.get(signal_name)
        {
            Self::write_vcd_change_static(
                vcd_writer,
                &signal.vcd_symbol,
                self.current_time_ns,
                &value,
            )
            .ok();
        }
    }

    /// クロック信号の記録
    pub fn record_clock(&mut self, signal_name: &str, edge: ClockEdge) {
        let value = match edge {
            ClockEdge::Rising => WaveformValue::Logic(LogicLevel::High),
            ClockEdge::Falling => WaveformValue::Logic(LogicLevel::Low),
            ClockEdge::None => return, // 変化なしの場合は記録しない
        };
        self.record_change(signal_name, value);
    }

    /// バス信号の記録
    pub fn record_bus(&mut self, signal_name: &str, values: &[LogicLevel]) {
        let value = WaveformValue::from_logic_bus(values);
        self.record_change(signal_name, value);
    }

    /// 整数値の記録
    pub fn record_integer(&mut self, signal_name: &str, value: u64) {
        self.record_change(signal_name, WaveformValue::Integer(value));
    }

    /// 状態文字列の記録
    pub fn record_string(&mut self, signal_name: &str, value: String) {
        self.record_change(signal_name, WaveformValue::String(value));
    }

    /// 時間を進める
    pub fn advance_time(&mut self, delta_ns: u64) {
        self.current_time_ns += delta_ns;
    }

    /// 記録開始
    pub fn start_recording(&mut self, vcd_filename: Option<&str>) -> std::io::Result<()> {
        self.recording = true;

        if let Some(filename) = vcd_filename {
            let file = File::create(filename)?;
            let mut writer = BufWriter::new(file);

            // VCDヘッダを書き込み
            self.write_vcd_header(&mut writer)?;
            self.vcd_file = Some(writer);
        }

        Ok(())
    }

    /// 記録停止
    pub fn stop_recording(&mut self) -> std::io::Result<()> {
        self.recording = false;

        if let Some(mut vcd_writer) = self.vcd_file.take() {
            writeln!(vcd_writer, "$end")?;
            vcd_writer.flush()?;
        }

        Ok(())
    }

    /// VCDヘッダの書き込み
    fn write_vcd_header(&mut self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        writeln!(writer, "$date")?;
        let date_str = jiff::Timestamp::from_second(timestamp as i64)
            .map(|ts| {
                ts.to_zoned(jiff::tz::TimeZone::system())
                    .strftime("%a %b %d %H:%M:%S %Y")
                    .to_string()
            })
            .unwrap_or_default();
        writeln!(writer, "    {}", date_str)?;
        writeln!(writer, "$end")?;

        writeln!(writer, "$version")?;
        writeln!(writer, "    Rust Computer Simulator v1.0")?;
        writeln!(writer, "$end")?;

        writeln!(writer, "$timescale")?;
        writeln!(writer, "    1ns")?;
        writeln!(writer, "$end")?;

        writeln!(writer, "$scope module top $end")?;

        // 信号定義
        for signal in self.signals.values() {
            if signal.width == 1 {
                writeln!(
                    writer,
                    "$var wire 1 {} {} $end",
                    signal.vcd_symbol, signal.name
                )?;
            } else {
                writeln!(
                    writer,
                    "$var wire {} {} {} $end",
                    signal.width, signal.vcd_symbol, signal.name
                )?;
            }
        }

        writeln!(writer, "$upscope $end")?;
        writeln!(writer, "$enddefinitions $end")?;
        writeln!(writer, "$dumpvars")?;

        // 初期値を書き込み
        for signal in self.signals.values() {
            if signal.width == 1 {
                writeln!(writer, "0{}", signal.vcd_symbol)?;
            } else {
                writeln!(
                    writer,
                    "b{} {}",
                    "0".repeat(signal.width),
                    signal.vcd_symbol
                )?;
            }
        }

        writeln!(writer, "$end")?;

        Ok(())
    }

    /// VCD値変化の書き込み
    fn write_vcd_change(
        &self,
        writer: &mut BufWriter<File>,
        signal: &WaveformSignal,
        value: &WaveformValue,
    ) -> std::io::Result<()> {
        writeln!(writer, "#{}", self.current_time_ns)?;

        match value {
            WaveformValue::Logic(_) => {
                writeln!(writer, "{}{}", value.to_vcd_char(), signal.vcd_symbol)?;
            }
            WaveformValue::Binary(bits) => {
                let bit_string: String = bits
                    .iter()
                    .rev() // MSBファースト
                    .map(|&b| if b { '1' } else { '0' })
                    .collect();
                writeln!(writer, "b{} {}", bit_string, signal.vcd_symbol)?;
            }
            WaveformValue::Integer(val) => {
                let bit_string = format!("{val:08b}"); // 固定8ビット幅
                writeln!(writer, "b{} {}", bit_string, signal.vcd_symbol)?;
            }
            _ => {
                // その他の型は0として出力
                writeln!(writer, "0{}", signal.vcd_symbol)?;
            }
        }

        Ok(())
    }
    /// VCD値変化の直接書き込み（静的メソッド版）
    fn write_vcd_change_static(
        writer: &mut BufWriter<File>,
        vcd_symbol: &str,
        timestamp: u64,
        value: &WaveformValue,
    ) -> std::io::Result<()> {
        writeln!(writer, "#{timestamp}")?;

        match value {
            WaveformValue::Logic(_) => {
                writeln!(writer, "{}{}", value.to_vcd_char(), vcd_symbol)?;
            }
            WaveformValue::Binary(bits) => {
                let bit_string: String = bits
                    .iter()
                    .rev() // MSBファースト
                    .map(|&b| if b { '1' } else { '0' })
                    .collect();
                writeln!(writer, "b{bit_string} {vcd_symbol}")?;
            }
            WaveformValue::Integer(val) => {
                let bit_string = format!("{val:08b}"); // 固定8ビット幅
                writeln!(writer, "b{bit_string} {vcd_symbol}")?;
            }
            WaveformValue::Float(val) => {
                writeln!(writer, "r{val} {vcd_symbol}")?;
            }
            WaveformValue::String(s) => {
                writeln!(writer, "s{s} {vcd_symbol}")?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// VCD値変化の直接書き込み（借用チェッカー回避用） - 非推奨
    fn write_vcd_change_direct(
        &self,
        writer: &mut BufWriter<File>,
        vcd_symbol: &str,
        timestamp: u64,
        value: &WaveformValue,
    ) -> std::io::Result<()> {
        Self::write_vcd_change_static(writer, vcd_symbol, timestamp, value)
    }

    /// テキスト形式の波形出力
    pub fn generate_text_waveform(
        &self,
        signal_name: &str,
        start_ns: u64,
        end_ns: u64,
        width: usize,
    ) -> String {
        if let Some(signal) = self.signals.get(signal_name) {
            let mut output = String::new();
            output.push_str(&format!(
                "Signal: {signal_name} ({start_ns}ns - {end_ns}ns)\n"
            ));
            output.push_str(&format!("{}\n", "=".repeat(width + 20)));

            let time_scale = (end_ns - start_ns) as f64 / width as f64;
            let mut current_value = WaveformValue::Logic(LogicLevel::Low);

            for i in 0..width {
                let time_point = start_ns + (i as f64 * time_scale) as u64;

                // この時点での値を探す
                for change in &signal.changes {
                    if change.timestamp_ns <= time_point {
                        current_value = change.value.clone();
                    } else {
                        break;
                    }
                }

                let char_repr = match &current_value {
                    WaveformValue::Logic(LogicLevel::High) => '▁',
                    WaveformValue::Logic(LogicLevel::Low) => '▔',
                    WaveformValue::Logic(LogicLevel::HighZ) => 'Z',
                    WaveformValue::Logic(LogicLevel::Unknown) => 'X',
                    WaveformValue::Integer(val) => {
                        if *val > 0 {
                            '▁'
                        } else {
                            '▔'
                        }
                    }
                    _ => '?',
                };

                output.push(char_repr);
            }

            output.push('\n');
            output.push_str(&format!("{start_ns}ns"));
            for _ in 0..(width - 20) {
                output.push(' ');
            }
            output.push_str(&format!("{end_ns}ns\n"));

            output
        } else {
            format!("Signal '{signal_name}' not found\n")
        }
    }

    /// 全信号のサマリー
    pub fn generate_summary(&self) -> String {
        let mut output = String::new();
        output.push_str("Waveform Recording Summary:\n");
        output.push_str("==========================\n");
        output.push_str(&format!(
            "Total recording time: {}ns\n",
            self.current_time_ns
        ));
        output.push_str(&format!("Number of signals: {}\n", self.signals.len()));
        output.push_str("\nSignals:\n");

        for (name, signal) in &self.signals {
            output.push_str(&format!(
                "  - {} ({}bit): {} changes\n",
                name,
                signal.width,
                signal.changes.len()
            ));

            if let Some(latest) = signal.get_latest_value() {
                match latest {
                    WaveformValue::Logic(level) => {
                        output.push_str(&format!("    Latest: {level:?}\n"))
                    }
                    WaveformValue::Integer(val) => {
                        output.push_str(&format!("    Latest: 0x{val:X}\n"))
                    }
                    WaveformValue::String(s) => output.push_str(&format!("    Latest: {s}\n")),
                    _ => output.push_str("    Latest: (complex)\n"),
                }
            }
        }

        output
    }

    pub fn get_signal_names(&self) -> Vec<String> {
        self.signals.keys().cloned().collect()
    }

    pub fn get_recording_time(&self) -> u64 {
        self.current_time_ns
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }
}

impl Default for WaveformRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_value() {
        let levels = vec![LogicLevel::High, LogicLevel::Low, LogicLevel::High];
        let value = WaveformValue::from_logic_bus(&levels);

        assert_eq!(value.to_integer(), Some(0b101));
    }

    #[test]
    fn test_waveform_recorder() {
        let mut recorder = WaveformRecorder::new();
        recorder.add_signal("clk".to_string(), 1);
        recorder.add_signal("data".to_string(), 8);

        recorder.start_recording(None).unwrap();

        recorder.record_change("clk", WaveformValue::Logic(LogicLevel::High));
        recorder.advance_time(1000);
        recorder.record_change("clk", WaveformValue::Logic(LogicLevel::Low));
        recorder.record_integer("data", 0x42);

        let signal = recorder.signals.get("clk").unwrap();
        assert_eq!(signal.changes.len(), 2);

        recorder.stop_recording().unwrap();
    }

    #[test]
    fn test_text_waveform() {
        let mut recorder = WaveformRecorder::new();
        recorder.add_signal("test".to_string(), 1);
        recorder.start_recording(None).unwrap();

        recorder.record_change("test", WaveformValue::Logic(LogicLevel::Low));
        recorder.advance_time(1000);
        recorder.record_change("test", WaveformValue::Logic(LogicLevel::High));
        recorder.advance_time(1000);
        recorder.record_change("test", WaveformValue::Logic(LogicLevel::Low));

        let waveform = recorder.generate_text_waveform("test", 0, 2000, 40);
        assert!(waveform.contains("Signal: test"));
    }
}
