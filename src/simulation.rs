/// コンピュータシミュレーション実行エンジン
use crate::logic_gates::*;
use crate::clock::*;
use crate::dram::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// シミュレーション対象のコンポーネントトレイト
pub trait SimulationComponent {
    fn update(&mut self, delta_time_ns: u64);
    fn get_name(&self) -> &str;
    fn reset(&mut self);
}

/// ワイヤ接続システム
#[derive(Debug, Clone)]
pub struct Wire {
    pub name: String,
    pub current_level: LogicLevel,
    pub previous_level: LogicLevel,
    pub connected_outputs: Vec<String>, // 出力コンポーネントのID
    pub connected_inputs: Vec<String>,  // 入力コンポーネントのID
}

impl Wire {
    pub fn new(name: String) -> Self {
        Self {
            name,
            current_level: LogicLevel::HighZ,
            previous_level: LogicLevel::HighZ,
            connected_outputs: Vec::new(),
            connected_inputs: Vec::new(),
        }
    }

    pub fn set_level(&mut self, level: LogicLevel) {
        self.previous_level = self.current_level;
        self.current_level = level;
    }

    pub fn has_changed(&self) -> bool {
        self.current_level != self.previous_level
    }
}

/// CPUの簡単な実装（レジスタとALU）
#[derive(Debug)]
pub struct SimpleCpu {
    name: String,
    accumulator: u8,
    program_counter: u16,
    memory_address_register: u16,
    memory_data_register: u8,
    instruction_register: u8,
    flags: CpuFlags,
    state: CpuState,
}

#[derive(Debug, Clone)]
pub struct CpuFlags {
    zero: bool,
    carry: bool,
    negative: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CpuState {
    Fetch,
    Decode,
    Execute,
    Halt,
}

impl SimpleCpu {
    pub fn new(name: String) -> Self {
        Self {
            name,
            accumulator: 0,
            program_counter: 0,
            memory_address_register: 0,
            memory_data_register: 0,
            instruction_register: 0,
            flags: CpuFlags {
                zero: false,
                carry: false,
                negative: false,
            },
            state: CpuState::Fetch,
        }
    }

    pub fn get_accumulator(&self) -> u8 {
        self.accumulator
    }

    pub fn get_program_counter(&self) -> u16 {
        self.program_counter
    }

    pub fn get_state(&self) -> &CpuState {
        &self.state
    }

    pub fn load_accumulator(&mut self, value: u8) {
        self.accumulator = value;
        self.update_flags();
    }

    pub fn add(&mut self, value: u8) {
        let (result, carry) = self.accumulator.overflowing_add(value);
        self.accumulator = result;
        self.flags.carry = carry;
        self.update_flags();
    }

    pub fn subtract(&mut self, value: u8) {
        let (result, borrow) = self.accumulator.overflowing_sub(value);
        self.accumulator = result;
        self.flags.carry = borrow;
        self.update_flags();
    }

    fn update_flags(&mut self) {
        self.flags.zero = self.accumulator == 0;
        self.flags.negative = (self.accumulator & 0x80) != 0;
    }

    pub fn fetch_instruction(&mut self, memory_data: u8) {
        if self.state == CpuState::Fetch {
            self.instruction_register = memory_data;
            self.state = CpuState::Decode;
        }
    }

    pub fn execute_cycle(&mut self) {
        match self.state {
            CpuState::Fetch => {
                // メモリからの命令フェッチを要求
                self.memory_address_register = self.program_counter;
            }
            CpuState::Decode => {
                // 命令デコード（簡略化）
                self.state = CpuState::Execute;
            }
            CpuState::Execute => {
                // 命令実行（簡略化）
                match self.instruction_register {
                    0x00 => { /* NOP */ }
                    0x01 => { self.load_accumulator(0x42); } // LDA immediate
                    0x02 => { self.add(0x01); }              // ADD immediate
                    0xFF => { self.state = CpuState::Halt; }  // HALT
                    _ => { /* Unknown instruction */ }
                }
                
                if self.state != CpuState::Halt {
                    self.program_counter += 1;
                    self.state = CpuState::Fetch;
                }
            }
            CpuState::Halt => {
                // 停止状態
            }
        }
    }
}

impl SimulationComponent for SimpleCpu {
    fn update(&mut self, _delta_time_ns: u64) {
        // CPUの更新ロジック
        self.execute_cycle();
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn reset(&mut self) {
        self.accumulator = 0;
        self.program_counter = 0;
        self.memory_address_register = 0;
        self.memory_data_register = 0;
        self.instruction_register = 0;
        self.flags = CpuFlags {
            zero: false,
            carry: false,
            negative: false,
        };
        self.state = CpuState::Fetch;
    }
}

impl Clocked for SimpleCpu {
    fn on_clock_edge(&mut self, edge: ClockEdge, _clock_name: &str) {
        if edge == ClockEdge::Rising {
            self.execute_cycle();
        }
    }

    fn reset(&mut self) {
        SimulationComponent::reset(self);
    }
}

/// メインシミュレーションエンジン
pub struct ComputerSimulation {
    simulation_clock: SimulationClock,
    dram_controller: DramController,
    cpu: SimpleCpu,
    wires: HashMap<String, Wire>,
    components: Vec<Box<dyn SimulationComponent>>,
    running: bool,
    step_count: u64,
    max_steps: Option<u64>,
}

impl ComputerSimulation {
    pub fn new() -> Self {
        let mut simulation_clock = SimulationClock::new(1_000_000.0, 1000); // 1MHz, 1μs step
        simulation_clock.add_clock("cpu".to_string(), 100_000.0); // 100kHz CPU
        simulation_clock.add_clock("dram_refresh".to_string(), 1000.0); // 1kHz refresh

        Self {
            simulation_clock,
            dram_controller: DramController::new(256, 256, 64), // 64KB DRAM, 64ms refresh
            cpu: SimpleCpu::new("main_cpu".to_string()),
            wires: HashMap::new(),
            components: Vec::new(),
            running: false,
            step_count: 0,
            max_steps: None,
        }
    }

    pub fn add_wire(&mut self, name: String) {
        self.wires.insert(name.clone(), Wire::new(name));
    }

    pub fn connect_wire(&mut self, wire_name: &str, output_component: String, input_component: String) {
        if let Some(wire) = self.wires.get_mut(wire_name) {
            wire.connected_outputs.push(output_component);
            wire.connected_inputs.push(input_component);
        }
    }

    pub fn set_wire_level(&mut self, wire_name: &str, level: LogicLevel) {
        if let Some(wire) = self.wires.get_mut(wire_name) {
            wire.set_level(level);
        }
    }

    pub fn get_wire_level(&self, wire_name: &str) -> Option<LogicLevel> {
        self.wires.get(wire_name).map(|wire| wire.current_level)
    }

    pub fn add_component(&mut self, component: Box<dyn SimulationComponent>) {
        self.components.push(component);
    }

    pub fn start(&mut self) {
        self.running = true;
        println!("シミュレーション開始");
    }

    pub fn stop(&mut self) {
        self.running = false;
        println!("シミュレーション停止");
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.dram_controller.reset();
        for component in &mut self.components {
            component.reset();
        }
        self.step_count = 0;
        println!("シミュレーション リセット");
    }

    pub fn set_max_steps(&mut self, max_steps: u64) {
        self.max_steps = Some(max_steps);
    }

    pub fn step(&mut self) -> bool {
        if !self.running {
            return false;
        }

        if let Some(max) = self.max_steps {
            if self.step_count >= max {
                self.stop();
                return false;
            }
        }

        // クロック更新
        self.simulation_clock.step();
        let master_clock = self.simulation_clock.get_master_clock();
        let cpu_clock = self.simulation_clock.get_clock("cpu");

        // CPU更新
        if let Some(cpu_clk) = cpu_clock {
            self.cpu.on_clock_edge(cpu_clk.get_edge(), "cpu");
        }

        // DRAM更新
        self.dram_controller.on_clock_edge(master_clock.get_edge(), "master");

        // その他のコンポーネント更新
        for component in &mut self.components {
            component.update(self.simulation_clock.time_step_ns);
        }

        self.step_count += 1;
        true
    }

    pub fn run(&mut self, steps: Option<u64>) {
        if let Some(step_count) = steps {
            self.set_max_steps(step_count);
        }

        self.start();
        
        let start_time = Instant::now();
        let mut last_status_time = start_time;

        while self.step() {
            // 定期的にステータス出力
            if last_status_time.elapsed() >= Duration::from_secs(1) {
                self.print_status();
                last_status_time = Instant::now();
            }
        }

        let elapsed = start_time.elapsed();
        println!("シミュレーション完了: {}ステップ、実行時間: {:.2}秒", 
                 self.step_count, elapsed.as_secs_f64());
    }

    pub fn print_status(&self) {
        println!("=== シミュレーション状態 ===");
        println!("ステップ数: {}", self.step_count);
        println!("シミュレーション時間: {:.2}ms", self.simulation_clock.get_current_time_ms());
        println!("CPU状態: {:?}", self.cpu.get_state());
        println!("CPU アキュムレータ: 0x{:02X}", self.cpu.get_accumulator());
        println!("CPU プログラムカウンタ: 0x{:04X}", self.cpu.get_program_counter());
        
        // DRAMの一部のセル状態を表示
        if let Some(charge) = self.dram_controller.get_dram().get_cell_charge(0, 0) {
            println!("DRAM[0,0] 電荷レベル: {:.3}", charge);
        }
        
        println!("ワイヤ数: {}", self.wires.len());
        println!("========================");
    }

    pub fn get_step_count(&self) -> u64 {
        self.step_count
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn get_cpu(&self) -> &SimpleCpu {
        &self.cpu
    }

    pub fn get_dram(&self) -> &DramController {
        &self.dram_controller
    }

    pub fn get_dram_mut(&mut self) -> &mut DramController {
        &mut self.dram_controller
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire() {
        let mut wire = Wire::new("test_wire".to_string());
        wire.set_level(LogicLevel::High);
        assert_eq!(wire.current_level, LogicLevel::High);
        assert!(wire.has_changed());
    }

    #[test]
    fn test_simple_cpu() {
        let mut cpu = SimpleCpu::new("test_cpu".to_string());
        cpu.load_accumulator(0x10);
        cpu.add(0x05);
        assert_eq!(cpu.get_accumulator(), 0x15);
    }

    #[test]
    fn test_simulation() {
        let mut sim = ComputerSimulation::new();
        sim.add_wire("data_bus".to_string());
        sim.set_wire_level("data_bus", LogicLevel::High);
        
        assert_eq!(sim.get_wire_level("data_bus"), Some(LogicLevel::High));
        
        // 短期間実行
        sim.run(Some(10));
        assert_eq!(sim.get_step_count(), 10);
    }
}
