mod logic_gates;
mod clock;
mod dram;
mod simulation;

use logic_gates::*;
use clock::*;
use dram::*;
use simulation::*;

fn main() {
    println!("=== Rustコンピュータシミュレーション ===");
    println!("論理回路とDRAMを含む真偽値・クロック駆動シミュレーション");
    println!();

    // 基本論理ゲートのテスト
    println!("1. 基本論理ゲートのテスト");
    test_logic_gates();
    println!();

    // クロックシステムのテスト
    println!("2. クロックシステムのテスト");
    test_clock_system();
    println!();

    // DRAMシミュレーションのテスト
    println!("3. DRAMシミュレーションのテスト");
    test_dram_simulation();
    println!();

    // 統合シミュレーションの実行
    println!("4. 統合コンピュータシミュレーション");
    run_computer_simulation();
}

fn test_logic_gates() {
    println!("  AND(1, 1) = {:?}", LogicGates::and(LogicLevel::High, LogicLevel::High));
    println!("  OR(0, 1) = {:?}", LogicGates::or(LogicLevel::Low, LogicLevel::High));
    println!("  NOT(1) = {:?}", LogicGates::not(LogicLevel::High));
    println!("  XOR(1, 0) = {:?}", LogicGates::xor(LogicLevel::High, LogicLevel::Low));
    
    // 半加算器のテスト
    let (sum, carry) = CompositeGates::half_adder(LogicLevel::High, LogicLevel::High);
    println!("  半加算器(1, 1): sum={:?}, carry={:?}", sum, carry);
    
    // 全加算器のテスト
    let (sum, carry) = CompositeGates::full_adder(LogicLevel::High, LogicLevel::High, LogicLevel::Low);
    println!("  全加算器(1, 1, 0): sum={:?}, carry={:?}", sum, carry);
}

fn test_clock_system() {
    let mut sim_clock = SimulationClock::new(1_000_000.0, 500); // 1MHz, 0.5μs step
    sim_clock.add_clock("cpu".to_string(), 100_000.0); // 100kHz CPU
    
    println!("  マスタークロック周波数: {:.1}MHz", sim_clock.get_master_clock().get_frequency() / 1_000_000.0);
    
    // Dフリップフロップのテスト
    let mut dff = DFlipFlop::new();
    dff.set_d(true);
    
    for i in 0..5 {
        sim_clock.step();
        let master_edge = sim_clock.get_master_clock().get_edge();
        let cpu_edge = sim_clock.get_clock("cpu").unwrap().get_edge();
        
        dff.on_clock_edge(master_edge, "master");
        
        println!("  ステップ{}: 時間={:.1}μs, マスタークロック={:?}, CPU={:?}, DFF_Q={}", 
                 i + 1, 
                 sim_clock.get_current_time_us(),
                 master_edge,
                 cpu_edge,
                 dff.get_q());
    }
}

fn test_dram_simulation() {
    let mut dram_controller = DramController::new(16, 16, 64); // 16x16 DRAM, 64ms refresh
    let dram = dram_controller.get_dram_mut();
    
    // アドレス[2, 3]にデータ書き込み
    dram.set_address_bus(vec![
        LogicLevel::Low, LogicLevel::High, // 行アドレス = 2
        LogicLevel::High, LogicLevel::High // 列アドレス = 3
    ]);
    dram.set_data_bus(vec![LogicLevel::High]); // データ = 1
    dram.set_we(true);
    
    println!("  DRAM書き込み動作:");
    dram.set_ras(true);
    println!("    RAS信号アクティブ");
    dram.set_cas(true);
    println!("    CAS信号アクティブ - データ書き込み完了");
    
    // 読み出し動作
    dram.set_we(false);
    dram.set_ras(false);
    dram.set_cas(false);
    
    println!("  DRAM読み出し動作:");
    dram.set_ras(true);
    println!("    RAS信号アクティブ");
    dram.set_cas(true);
    println!("    CAS信号アクティブ - データ読み出し完了");
    
    let data_bus = dram.get_data_bus();
    println!("    読み出しデータ: {:?}", data_bus[0]);
    
    // 電荷レベルの表示
    if let Some(charge) = dram.get_cell_charge(2, 3) {
        println!("    セル[2,3]の電荷レベル: {:.3}", charge);
    }
    
    // 電荷漏れシミュレーション
    dram.simulate_leakage(10.0); // 10ms経過
    if let Some(charge) = dram.get_cell_charge(2, 3) {
        println!("    10ms後の電荷レベル: {:.3}", charge);
    }
}

fn run_computer_simulation() {
    let mut simulation = ComputerSimulation::new();
    
    // ワイヤ接続の設定
    simulation.add_wire("address_bus".to_string());
    simulation.add_wire("data_bus".to_string());
    simulation.add_wire("control_bus".to_string());
    
    println!("  シミュレーション環境構築完了");
    println!("  - CPU: 簡易8ビットプロセッサ");
    println!("  - DRAM: 256x256 (64KB)、64ms リフレッシュ間隔");
    println!("  - クロック: 1MHz マスター、100kHz CPU");
    
    // DRAMにテストデータを書き込み
    let dram = simulation.get_dram_mut().get_dram_mut();
    dram.set_address_bus(vec![LogicLevel::Low; 16]); // アドレス0
    dram.set_data_bus(vec![LogicLevel::High, LogicLevel::Low, LogicLevel::High]); // 0x05
    dram.set_we(true);
    dram.set_ras(true);
    dram.set_cas(true);
    
    println!("  初期データをDRAMに書き込み完了");
    
    // シミュレーション実行
    println!("  シミュレーション開始 (100ステップ実行)");
    simulation.run(Some(100));
    
    // 結果表示
    println!();
    simulation.print_status();
    
    println!();
    println!("=== シミュレーション完了 ===");
    println!("Rustによる論理回路・DRAM統合シミュレーションが正常に動作しました！");
}
