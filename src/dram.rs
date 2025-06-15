use crate::clock::{ClockEdge, Clocked};
/// DRAMシミュレーション - 現実の回路素子特性を模擬
use crate::logic_gates::LogicLevel;
use std::time::{Duration, Instant};

/// DRAMセルの状態
#[derive(Debug, Clone)]
pub struct DramCell {
    /// データ (true/false)
    data: bool,
    /// 最後のリフレッシュ時刻
    last_refresh_time: Option<Instant>,
    /// 現在アクセス中かどうか
    is_accessing: bool,
    /// 行選択線の状態
    row_select: bool,
    /// 列選択線の状態
    col_select: bool,
}

impl DramCell {
    pub fn new() -> Self {
        Self {
            data: false,
            last_refresh_time: None,
            is_accessing: false,
            row_select: false,
            col_select: false,
        }
    }
    /// データを書き込み
    pub fn write(&mut self, data: bool) {
        if self.is_accessible() {
            self.data = data;
            self.last_refresh_time = Some(Instant::now());
        }
    }
    /// データを読み出し（破壊読み出し）
    pub fn read(&mut self) -> Option<bool> {
        if !self.is_accessible() {
            return None;
        }

        let current_data = self.data;

        // 読み出し後のリストア（破壊読み出しの特性）
        self.last_refresh_time = Some(Instant::now());

        Some(current_data)
    }
    /// リフレッシュ動作
    pub fn refresh(&mut self) {
        if self.is_accessible() {
            // 現在の状態を保持してリフレッシュ時刻を更新
            self.last_refresh_time = Some(Instant::now());
        }
    }
    /// データが有効かチェック（リフレッシュ間隔を超えた場合は無効）
    pub fn is_data_valid(&self, refresh_interval_ms: u64) -> bool {
        match self.last_refresh_time {
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                elapsed.as_millis() as u64 <= refresh_interval_ms
            }
            None => false, // 未初期化のセルは無効
        }
    }

    pub fn set_row_select(&mut self, select: bool) {
        self.row_select = select;
    }

    pub fn set_col_select(&mut self, select: bool) {
        self.col_select = select;
    }

    pub fn is_accessible(&self) -> bool {
        self.row_select && self.col_select
    }
    pub fn get_data(&self) -> bool {
        self.data
    }

    pub fn needs_refresh(&self, refresh_interval_ms: u64) -> bool {
        match self.last_refresh_time {
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                elapsed.as_millis() as u64 > refresh_interval_ms
            }
            None => true,
        }
    }
}

/// DRAMメモリアレイ
#[derive(Debug)]
pub struct DramArray {
    /// メモリセル配列
    cells: Vec<Vec<DramCell>>,
    /// 行デコーダの状態
    row_decoder: Vec<bool>,
    /// 列デコーダの状態
    col_decoder: Vec<bool>,
    /// 行アドレス
    row_address: usize,
    /// 列アドレス
    col_address: usize,
    /// RAS信号 (Row Address Strobe)
    ras_signal: bool,
    /// CAS信号 (Column Address Strobe)
    cas_signal: bool,
    /// WE信号 (Write Enable)
    we_signal: bool,
    /// データバス
    data_bus: Vec<LogicLevel>,
    /// アドレスバス
    address_bus: Vec<LogicLevel>,
    /// リフレッシュカウンタ
    refresh_counter: usize,
    /// 最後のリフレッシュ時刻
    last_refresh_time: Instant,
    /// 寸法
    rows: usize,
    cols: usize,
}

impl DramArray {
    pub fn new(rows: usize, cols: usize) -> Self {
        let mut cells = Vec::new();
        for _ in 0..rows {
            let mut row = Vec::new();
            for _ in 0..cols {
                row.push(DramCell::new());
            }
            cells.push(row);
        }

        Self {
            cells,
            row_decoder: vec![false; rows],
            col_decoder: vec![false; cols],
            row_address: 0,
            col_address: 0,
            ras_signal: false,
            cas_signal: false,
            we_signal: false,
            data_bus: vec![LogicLevel::HighZ; 8], // 8ビットデータバス
            address_bus: vec![LogicLevel::Low; 16], // 16ビットアドレスバス
            refresh_counter: 0,
            last_refresh_time: Instant::now(),
            rows,
            cols,
        }
    }

    /// RAS信号の設定
    pub fn set_ras(&mut self, ras: bool) {
        if !self.ras_signal && ras {
            // RAS立ち上がりエッジで行アドレスをラッチ
            self.row_address = self.decode_address() & ((1 << self.rows.ilog2()) - 1);
            self.activate_row(self.row_address);
        }
        self.ras_signal = ras;
    }

    /// CAS信号の設定
    pub fn set_cas(&mut self, cas: bool) {
        if !self.cas_signal && cas {
            // CAS立ち上がりエッジで列アドレスをラッチ
            self.col_address = self.decode_address() & ((1 << self.cols.ilog2()) - 1);
            self.activate_col(self.col_address);

            // 読み書き実行
            if self.ras_signal {
                if self.we_signal {
                    self.write_data();
                } else {
                    self.read_data();
                }
            }
        }
        self.cas_signal = cas;
    }

    /// WE信号の設定
    pub fn set_we(&mut self, we: bool) {
        self.we_signal = we;
    }

    /// アドレスバスの設定
    pub fn set_address_bus(&mut self, address: Vec<LogicLevel>) {
        self.address_bus = address;
    }

    /// データバスの設定
    pub fn set_data_bus(&mut self, data: Vec<LogicLevel>) {
        self.data_bus = data;
    }

    /// データバスの取得
    pub fn get_data_bus(&self) -> &Vec<LogicLevel> {
        &self.data_bus
    }

    /// アドレスデコード
    fn decode_address(&self) -> usize {
        let mut address = 0;
        for (i, bit) in self.address_bus.iter().enumerate() {
            if let Some(bit_val) = bit.to_bool()
                && bit_val
            {
                address |= 1 << i;
            }
        }
        address
    }

    /// 行の活性化
    fn activate_row(&mut self, row: usize) {
        // 全行を非活性化
        for decoder_bit in &mut self.row_decoder {
            *decoder_bit = false;
        }
        for row_cells in &mut self.cells {
            for cell in row_cells {
                cell.set_row_select(false);
            }
        }

        // 指定行を活性化
        if row < self.rows {
            self.row_decoder[row] = true;
            for cell in &mut self.cells[row] {
                cell.set_row_select(true);
            }
        }
    }

    /// 列の活性化
    fn activate_col(&mut self, col: usize) {
        // 全列を非活性化
        for decoder_bit in &mut self.col_decoder {
            *decoder_bit = false;
        }
        for row_cells in &mut self.cells {
            for cell in row_cells {
                cell.set_col_select(false);
            }
        }

        // 指定列を活性化
        if col < self.cols {
            self.col_decoder[col] = true;
            for row_cells in &mut self.cells {
                if col < row_cells.len() {
                    row_cells[col].set_col_select(true);
                }
            }
        }
    }

    /// データ書き込み
    fn write_data(&mut self) {
        if self.row_address < self.rows && self.col_address < self.cols {
            let cell = &mut self.cells[self.row_address][self.col_address];

            // データバスの最初のビットを書き込み（簡略化）
            if let Some(data_bit) = self.data_bus.first().and_then(|level| level.to_bool()) {
                cell.write(data_bit);
            }
        }
    }

    /// データ読み出し
    fn read_data(&mut self) {
        if self.row_address < self.rows && self.col_address < self.cols {
            let cell = &mut self.cells[self.row_address][self.col_address];

            if let Some(data) = cell.read() {
                // データバスに出力
                self.data_bus[0] = LogicLevel::from_bool(data);
            } else {
                self.data_bus[0] = LogicLevel::Unknown;
            }
        }
    }

    /// 自動リフレッシュ
    pub fn auto_refresh(&mut self) {
        if self.refresh_counter >= self.rows {
            self.refresh_counter = 0;
        }

        // 現在の行をリフレッシュ
        for cell in &mut self.cells[self.refresh_counter] {
            cell.refresh();
        }

        self.refresh_counter += 1;
        self.last_refresh_time = Instant::now();
    }
    pub fn get_cell_data(&self, row: usize, col: usize) -> Option<bool> {
        self.cells.get(row)?.get(col).map(|cell| cell.get_data())
    }

    pub fn is_cell_data_valid(
        &self,
        row: usize,
        col: usize,
        refresh_interval_ms: u64,
    ) -> Option<bool> {
        self.cells
            .get(row)?
            .get(col)
            .map(|cell| cell.is_data_valid(refresh_interval_ms))
    }
}

/// DRAMコントローラ
#[derive(Debug)]
pub struct DramController {
    dram: DramArray,
    refresh_interval_ms: u64,
    next_refresh_time: Instant,
    auto_refresh_enabled: bool,
}

impl DramController {
    pub fn new(rows: usize, cols: usize, refresh_interval_ms: u64) -> Self {
        Self {
            dram: DramArray::new(rows, cols),
            refresh_interval_ms,
            next_refresh_time: Instant::now() + Duration::from_millis(refresh_interval_ms),
            auto_refresh_enabled: true,
        }
    }

    pub fn get_dram_mut(&mut self) -> &mut DramArray {
        &mut self.dram
    }

    pub fn get_dram(&self) -> &DramArray {
        &self.dram
    }

    pub fn set_auto_refresh(&mut self, enabled: bool) {
        self.auto_refresh_enabled = enabled;
    }
}

impl Clocked for DramController {
    fn on_clock_edge(&mut self, edge: ClockEdge, _clock_name: &str) {
        if edge == ClockEdge::Rising {
            // リフレッシュタイマーのチェック
            if self.auto_refresh_enabled && Instant::now() >= self.next_refresh_time {
                self.dram.auto_refresh();
                self.next_refresh_time =
                    Instant::now() + Duration::from_millis(self.refresh_interval_ms);
            }
        }
    }

    fn reset(&mut self) {
        // リセット処理
        self.dram = DramArray::new(self.dram.rows, self.dram.cols);
        self.next_refresh_time = Instant::now() + Duration::from_millis(self.refresh_interval_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dram_cell() {
        let mut cell = DramCell::new();

        // 書き込みテスト
        cell.set_row_select(true);
        cell.set_col_select(true);
        cell.write(true);

        assert_eq!(cell.get_data(), true);

        // 読み出しテスト
        let data = cell.read();
        assert_eq!(data, Some(true));
    }

    #[test]
    fn test_dram_array() {
        let mut dram = DramArray::new(4, 4);

        // アドレス設定
        dram.set_address_bus(vec![LogicLevel::Low, LogicLevel::High]); // アドレス2
        dram.set_data_bus(vec![LogicLevel::High]); // データ1

        // 書き込み
        dram.set_we(true);
        dram.set_ras(true);
        dram.set_cas(true);

        // 読み出し
        dram.set_we(false);
        dram.set_ras(false);
        dram.set_cas(false);
        dram.set_ras(true);
        dram.set_cas(true);

        let data_bus = dram.get_data_bus();
        assert_eq!(data_bus[0], LogicLevel::High);
    }
}
