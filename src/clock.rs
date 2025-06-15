/// クロック信号とタイミング管理システム
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockEdge {
    Rising,   // 立ち上がりエッジ
    Falling,  // 立ち下がりエッジ
    None,     // エッジなし
}

#[derive(Debug, Clone)]
pub struct ClockSignal {
    current_state: bool,
    previous_state: bool,
    frequency_hz: f64,
    current_time_ns: u64,
    period_ns: u64,
}

impl ClockSignal {
    pub fn new(frequency_hz: f64) -> Self {
        let period_ns = (1_000_000_000.0 / frequency_hz) as u64;
        Self {
            current_state: false,
            previous_state: false,
            frequency_hz,
            current_time_ns: 0,
            period_ns,
        }
    }

    pub fn tick(&mut self, delta_time_ns: u64) {
        self.previous_state = self.current_state;
        self.current_time_ns += delta_time_ns;
        
        let cycle_position = self.current_time_ns % self.period_ns;
        self.current_state = cycle_position < (self.period_ns / 2);
    }

    pub fn get_edge(&self) -> ClockEdge {
        match (self.previous_state, self.current_state) {
            (false, true) => ClockEdge::Rising,
            (true, false) => ClockEdge::Falling,
            _ => ClockEdge::None,
        }
    }

    pub fn is_high(&self) -> bool {
        self.current_state
    }

    pub fn is_low(&self) -> bool {
        !self.current_state
    }

    pub fn get_frequency(&self) -> f64 {
        self.frequency_hz
    }
}

/// シミュレーション時間管理
#[derive(Debug)]
pub struct SimulationClock {
    master_clock: ClockSignal,
    secondary_clocks: HashMap<String, ClockSignal>,
    current_simulation_time_ns: u64,
    time_step_ns: u64,
}

impl SimulationClock {
    pub fn new(master_frequency_hz: f64, time_step_ns: u64) -> Self {
        Self {
            master_clock: ClockSignal::new(master_frequency_hz),
            secondary_clocks: HashMap::new(),
            current_simulation_time_ns: 0,
            time_step_ns,
        }
    }

    pub fn add_clock(&mut self, name: String, frequency_hz: f64) {
        self.secondary_clocks.insert(name, ClockSignal::new(frequency_hz));
    }

    pub fn step(&mut self) {
        self.current_simulation_time_ns += self.time_step_ns;
        self.master_clock.tick(self.time_step_ns);
        
        for clock in self.secondary_clocks.values_mut() {
            clock.tick(self.time_step_ns);
        }
    }

    pub fn get_master_clock(&self) -> &ClockSignal {
        &self.master_clock
    }

    pub fn get_clock(&self, name: &str) -> Option<&ClockSignal> {
        self.secondary_clocks.get(name)
    }

    pub fn get_current_time_ns(&self) -> u64 {
        self.current_simulation_time_ns
    }

    pub fn get_current_time_us(&self) -> f64 {
        self.current_simulation_time_ns as f64 / 1000.0
    }

    pub fn get_current_time_ms(&self) -> f64 {
        self.current_simulation_time_ns as f64 / 1_000_000.0
    }
}

/// クロック同期機能を持つ要素のトレイト
pub trait Clocked {
    fn on_clock_edge(&mut self, edge: ClockEdge, clock_name: &str);
    fn reset(&mut self);
}

/// Dフリップフロップの実装例
#[derive(Debug, Clone)]
pub struct DFlipFlop {
    q: bool,
    q_not: bool,
    d_input: bool,
    enable: bool,
}

impl DFlipFlop {
    pub fn new() -> Self {
        Self {
            q: false,
            q_not: true,
            d_input: false,
            enable: true,
        }
    }

    pub fn set_d(&mut self, value: bool) {
        self.d_input = value;
    }

    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
    }

    pub fn get_q(&self) -> bool {
        self.q
    }

    pub fn get_q_not(&self) -> bool {
        self.q_not
    }
}

impl Clocked for DFlipFlop {
    fn on_clock_edge(&mut self, edge: ClockEdge, _clock_name: &str) {
        if edge == ClockEdge::Rising && self.enable {
            self.q = self.d_input;
            self.q_not = !self.d_input;
        }
    }

    fn reset(&mut self) {
        self.q = false;
        self.q_not = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_signal() {
        let mut clock = ClockSignal::new(1_000_000.0); // 1MHz
        assert_eq!(clock.is_low(), true);
        
        clock.tick(500); // 0.5μs
        assert_eq!(clock.is_high(), true);
        assert_eq!(clock.get_edge(), ClockEdge::Rising);
    }

    #[test]
    fn test_d_flip_flop() {
        let mut dff = DFlipFlop::new();
        dff.set_d(true);
        
        dff.on_clock_edge(ClockEdge::Rising, "master");
        assert_eq!(dff.get_q(), true);
        assert_eq!(dff.get_q_not(), false);
    }

    #[test]
    fn test_simulation_clock() {
        let mut sim_clock = SimulationClock::new(1_000_000.0, 100); // 1MHz, 100ns step
        sim_clock.add_clock("cpu".to_string(), 100_000_000.0); // 100MHz CPU clock
        
        sim_clock.step();
        assert_eq!(sim_clock.get_current_time_ns(), 100);
    }
}
