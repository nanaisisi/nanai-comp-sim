use crate::clock::{ClockEdge, Clocked};
use crate::dram::DramController;
/// メモリ階層シミュレーション - CPU内部キャッシュ、SRAM、外部DRAM
use crate::logic_gates::LogicLevel;
use std::collections::HashMap;
use std::time::Instant;

/// メモリアクセスタイプ
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAccessType {
    Byte,
    HalfWord,
    Word,
    DoubleWord,
}

/// キャッシュラインの状態
#[derive(Debug, Clone, PartialEq)]
pub enum CacheLineState {
    Invalid,
    Valid,
    Dirty,
}

/// CPU内部のL1キャッシュライン
#[derive(Debug, Clone)]
pub struct CacheLine {
    tag: u64,
    data: Vec<u8>,
    state: CacheLineState,
    last_access: Instant,
}

impl CacheLine {
    pub fn new(line_size: usize) -> Self {
        Self {
            tag: 0,
            data: vec![0; line_size],
            state: CacheLineState::Invalid,
            last_access: Instant::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.state != CacheLineState::Invalid
    }

    pub fn is_dirty(&self) -> bool {
        self.state == CacheLineState::Dirty
    }

    pub fn read_data(&mut self, offset: usize, size: usize) -> Option<Vec<u8>> {
        if !self.is_valid() || offset + size > self.data.len() {
            return None;
        }
        self.last_access = Instant::now();
        Some(self.data[offset..offset + size].to_vec())
    }

    pub fn write_data(&mut self, offset: usize, data: &[u8]) -> bool {
        if offset + data.len() > self.data.len() {
            return false;
        }
        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.state = CacheLineState::Dirty;
        self.last_access = Instant::now();
        true
    }

    pub fn load_from_memory(&mut self, tag: u64, data: Vec<u8>) {
        self.tag = tag;
        self.data = data;
        self.state = CacheLineState::Valid;
        self.last_access = Instant::now();
    }

    pub fn invalidate(&mut self) {
        self.state = CacheLineState::Invalid;
    }
}

/// CPU内部のL1キャッシュ（命令キャッシュ・データキャッシュ）
#[derive(Debug)]
pub struct L1Cache {
    cache_lines: Vec<CacheLine>,
    line_size: usize,
    associativity: usize,
    total_size: usize,
    hits: u64,
    misses: u64,
    cache_type: String, // "ICache" or "DCache"
}

impl L1Cache {
    pub fn new(
        total_size: usize,
        line_size: usize,
        associativity: usize,
        cache_type: &str,
    ) -> Self {
        let num_lines = total_size / line_size;
        let mut cache_lines = Vec::new();
        for _ in 0..num_lines {
            cache_lines.push(CacheLine::new(line_size));
        }

        Self {
            cache_lines,
            line_size,
            associativity,
            total_size,
            hits: 0,
            misses: 0,
            cache_type: cache_type.to_string(),
        }
    }

    pub fn get_line_index(&self, address: u64) -> usize {
        let line_addr = address / self.line_size as u64;
        (line_addr as usize) % (self.cache_lines.len() / self.associativity)
    }

    pub fn get_tag(&self, address: u64) -> u64 {
        address / self.line_size as u64
    }

    pub fn read(&mut self, address: u64, size: usize) -> Option<Vec<u8>> {
        let line_index = self.get_line_index(address);
        let tag = self.get_tag(address);
        let offset = (address % self.line_size as u64) as usize;

        // キャッシュヒットをチェック
        for i in 0..self.associativity {
            let cache_line = &mut self.cache_lines[line_index * self.associativity + i];
            if cache_line.is_valid() && cache_line.tag == tag {
                // キャッシュヒット
                self.hits += 1;
                return cache_line.read_data(offset, size);
            }
        }

        // キャッシュミス
        self.misses += 1;
        None
    }

    pub fn write(&mut self, address: u64, data: &[u8]) -> bool {
        let line_index = self.get_line_index(address);
        let tag = self.get_tag(address);
        let offset = (address % self.line_size as u64) as usize;

        // キャッシュヒットをチェック
        for i in 0..self.associativity {
            let cache_line = &mut self.cache_lines[line_index * self.associativity + i];
            if cache_line.is_valid() && cache_line.tag == tag {
                // キャッシュヒット
                self.hits += 1;
                return cache_line.write_data(offset, data);
            }
        }

        // キャッシュミス
        self.misses += 1;
        false
    }

    pub fn load_line(&mut self, address: u64, data: Vec<u8>) {
        let line_index = self.get_line_index(address);
        let tag = self.get_tag(address);

        // LRU（Least Recently Used）でラインを選択
        let mut oldest_time = Instant::now();
        let mut oldest_index = 0;

        for i in 0..self.associativity {
            let idx = line_index * self.associativity + i;
            if !self.cache_lines[idx].is_valid() {
                // 空のラインを見つけた
                oldest_index = idx;
                break;
            }
            if self.cache_lines[idx].last_access < oldest_time {
                oldest_time = self.cache_lines[idx].last_access;
                oldest_index = idx;
            }
        }

        self.cache_lines[oldest_index].load_from_memory(tag, data);
    }

    pub fn get_hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    pub fn flush(&mut self) {
        for line in &mut self.cache_lines {
            line.invalidate();
        }
    }
}

/// CPU内部SRAM（スクラッチパッドメモリ）
#[derive(Debug)]
pub struct InternalSram {
    data: Vec<u8>,
    size: usize,
    access_cycles: u32,
}

impl InternalSram {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
            size,
            access_cycles: 1, // 1サイクルでアクセス可能
        }
    }

    pub fn read(&self, address: usize, size: usize) -> Option<Vec<u8>> {
        if address + size > self.size {
            return None;
        }
        Some(self.data[address..address + size].to_vec())
    }

    pub fn write(&mut self, address: usize, data: &[u8]) -> bool {
        if address + data.len() > self.size {
            return false;
        }
        self.data[address..address + data.len()].copy_from_slice(data);
        true
    }

    pub fn get_access_cycles(&self) -> u32 {
        self.access_cycles
    }
}

/// メモリ階層管理
#[derive(Debug)]
pub struct MemoryHierarchy {
    // CPU内部メモリ
    instruction_cache: L1Cache,  // 命令キャッシュ
    data_cache: L1Cache,         // データキャッシュ
    internal_sram: InternalSram, // 内部SRAM

    // 外部メモリ（独立クロック）
    external_dram: Option<DramController>, // 外部DRAM（大型システム用）    // メモリコントローラー設定
    is_microcontroller: bool,              // マイコン（内部SRAM）かどうか
    memory_map: HashMap<u64, String>,      // メモリマップ

    // パフォーマンス統計
    total_accesses: u64,
    cache_hits: u64,
    cache_misses: u64,
    dram_accesses: u64,
}

impl MemoryHierarchy {
    /// マイコン用の構成（内部SRAM）
    pub fn new_microcontroller(
        icache_size: usize,
        dcache_size: usize,
        internal_sram_size: usize,
    ) -> Self {
        Self {
            instruction_cache: L1Cache::new(icache_size, 32, 2, "ICache"),
            data_cache: L1Cache::new(dcache_size, 32, 2, "DCache"),
            internal_sram: InternalSram::new(internal_sram_size),
            external_dram: None, // マイコンは外部DRAM無し
            is_microcontroller: true,
            memory_map: Self::create_microcontroller_memory_map(),
            total_accesses: 0,
            cache_hits: 0,
            cache_misses: 0,
            dram_accesses: 0,
        }
    }

    /// PC/サーバー用の構成（外部DRAM）
    pub fn new_pc_server(
        icache_size: usize,
        dcache_size: usize,
        sram_size: usize,
        external_dram: DramController,
    ) -> Self {
        Self {
            instruction_cache: L1Cache::new(icache_size, 64, 4, "ICache"),
            data_cache: L1Cache::new(dcache_size, 64, 4, "DCache"),
            internal_sram: InternalSram::new(sram_size),
            external_dram: Some(external_dram),
            is_microcontroller: false,
            memory_map: Self::create_pc_server_memory_map(),
            total_accesses: 0,
            cache_hits: 0,
            cache_misses: 0,
            dram_accesses: 0,
        }
    }
    fn create_microcontroller_memory_map() -> HashMap<u64, String> {
        let mut map = HashMap::new();
        map.insert(0x0000_0000, "Internal Flash".to_string());
        map.insert(0x2000_0000, "Internal SRAM (Main Memory)".to_string());
        map.insert(0x4000_0000, "Peripheral Registers".to_string());
        map.insert(0x6000_0000, "Extended Internal SRAM".to_string());
        map
    }

    fn create_pc_server_memory_map() -> HashMap<u64, String> {
        let mut map = HashMap::new();
        map.insert(0x0000_0000, "Low Memory (DRAM)".to_string());
        map.insert(0x8000_0000, "High Memory (DRAM)".to_string());
        map.insert(0xF000_0000, "Memory Mapped I/O".to_string());
        map.insert(0xFFFF_0000, "Boot ROM".to_string());
        map
    }

    pub fn read_instruction(&mut self, address: u64) -> Option<u32> {
        self.total_accesses += 1;

        // L1命令キャッシュから読み込み試行
        if let Some(data) = self.instruction_cache.read(address, 4) {
            self.cache_hits += 1;
            return Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
        }

        // キャッシュミス - メモリから読み込み
        self.cache_misses += 1;
        if let Some(data) = self.read_from_memory(address, 4) {
            // キャッシュラインにロード
            let mut line_data = vec![0u8; 32]; // 32バイトのキャッシュライン
            let line_start = address & !31; // 32バイト境界に揃える
            if let Some(full_data) = self.read_from_memory(line_start, 32) {
                line_data = full_data;
            }
            self.instruction_cache.load_line(address, line_data);

            return Some(u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
        }

        None
    }

    pub fn read_data(&mut self, address: u64, size: usize) -> Option<Vec<u8>> {
        self.total_accesses += 1;

        // L1データキャッシュから読み込み試行
        if let Some(data) = self.data_cache.read(address, size) {
            self.cache_hits += 1;
            return Some(data);
        }

        // キャッシュミス - メモリから読み込み
        self.cache_misses += 1;
        if let Some(data) = self.read_from_memory(address, size) {
            // キャッシュラインにロード
            let line_size = if self.is_microcontroller { 32 } else { 64 };
            let mut line_data = vec![0u8; line_size];
            let line_start = address & !(line_size as u64 - 1);
            if let Some(full_data) = self.read_from_memory(line_start, line_size) {
                line_data = full_data;
            }
            self.data_cache.load_line(address, line_data);

            return Some(data);
        }

        None
    }

    pub fn write_data(&mut self, address: u64, data: &[u8]) -> bool {
        self.total_accesses += 1;

        // L1データキャッシュに書き込み試行
        if self.data_cache.write(address, data) {
            self.cache_hits += 1;
            return true;
        }

        // キャッシュミス - メモリに直接書き込み
        self.cache_misses += 1;
        self.write_to_memory(address, data)
    }

    fn read_from_memory(&mut self, address: u64, size: usize) -> Option<Vec<u8>> {
        // 内部SRAMの範囲チェック
        if address >= 0x2000_0000 && address < 0x2000_0000 + self.internal_sram.size as u64 {
            let offset = (address - 0x2000_0000) as usize;
            return self.internal_sram.read(offset, size);
        } // 外部DRAMアクセス
        if !self.is_microcontroller {
            if let Some(ref mut _dram) = self.external_dram {
                self.dram_accesses += 1;
                // 簡略化されたDRAMアクセス
                // 実際にはDRAMコントローラーを通じてアクセス
                return Some(vec![0; size]); // プレースホルダー
            }
        }

        None
    }

    fn write_to_memory(&mut self, address: u64, data: &[u8]) -> bool {
        // 内部SRAMの範囲チェック
        if address >= 0x2000_0000 && address < 0x2000_0000 + self.internal_sram.size as u64 {
            let offset = (address - 0x2000_0000) as usize;
            return self.internal_sram.write(offset, data);
        } // 外部DRAMアクセス
        if !self.is_microcontroller {
            if let Some(ref mut _dram) = self.external_dram {
                self.dram_accesses += 1;
                // 簡略化されたDRAMアクセス
                return true; // プレースホルダー
            }
        }

        false
    }

    pub fn get_cache_statistics(&self) -> (f64, f64) {
        let icache_hit_rate = self.instruction_cache.get_hit_rate();
        let dcache_hit_rate = self.data_cache.get_hit_rate();
        (icache_hit_rate, dcache_hit_rate)
    }
    pub fn get_memory_info(&self) -> String {
        let architecture = if self.is_microcontroller {
            "Microcontroller"
        } else {
            "PC/Server"
        };

        let (icache_hit_rate, dcache_hit_rate) = self.get_cache_statistics();

        let sram_description = if self.is_microcontroller {
            "Internal SRAM (Main Memory)"
        } else {
            "Internal SRAM"
        };

        format!(
            "Memory Architecture: {}\n\
             L1 ICache: {}KB (Hit Rate: {:.2}%)\n\
             L1 DCache: {}KB (Hit Rate: {:.2}%)\n\
             {}: {}KB\n\
             External DRAM: {}\n\
             Total Accesses: {}\n\
             DRAM Accesses: {}",
            architecture,
            self.instruction_cache.total_size / 1024,
            icache_hit_rate * 100.0,
            self.data_cache.total_size / 1024,
            dcache_hit_rate * 100.0,
            sram_description,
            self.internal_sram.size / 1024,
            if self.external_dram.is_some() {
                "Present"
            } else {
                "None"
            },
            self.total_accesses,
            self.dram_accesses
        )
    }

    pub fn flush_caches(&mut self) {
        self.instruction_cache.flush();
        self.data_cache.flush();
    }
}

impl Clocked for MemoryHierarchy {
    fn on_clock_edge(&mut self, edge: ClockEdge, _clock_name: &str) {
        if edge == ClockEdge::Rising {
            // 外部DRAMは独立したクロックで動作
            if let Some(ref mut _dram) = self.external_dram {
                // DRAMの独立クロック処理はここで行わない
                // 実際には別のクロックドメインで処理される
            }
        }
    }

    fn reset(&mut self) {
        self.flush_caches();
        self.total_accesses = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.dram_accesses = 0;
    }
}
