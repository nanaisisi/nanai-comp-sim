// メモリ階層テスト専用プログラム
mod memory_hierarchy;
mod dram;

use memory_hierarchy::MemoryHierarchy;
use dram::DramController;

fn main() {
    println!("\n=== Memory Hierarchy Test ===");

    // マイコン構成のテスト
    println!("1. Microcontroller Memory Hierarchy:");
    let mut mcu_memory = MemoryHierarchy::new_microcontroller(
        8 * 1024,   // 8KB L1 ICache
        8 * 1024,   // 8KB L1 DCache
        128 * 1024, // 128KB Internal SRAM (Main Memory)
    );

    println!("  {}", mcu_memory.get_memory_info());

    // メモリアクセステスト
    println!("\n  Memory Access Tests:");

    // 命令読み込みテスト
    println!("  - Instruction fetch test...");
    for i in 0..10 {
        let addr = 0x0000_1000 + i * 4;
        if let Some(_instr) = mcu_memory.read_instruction(addr) {
            println!("    Fetched instruction from 0x{:08X}", addr);
        }
    }

    // データ読み書きテスト
    println!("  - Data access test...");
    let test_data = vec![0x12, 0x34, 0x56, 0x78];
    let sram_addr = 0x2000_0100;

    if mcu_memory.write_data(sram_addr, &test_data) {
        println!("    Wrote data to SRAM at 0x{:08X}", sram_addr);
    }

    if let Some(read_data) = mcu_memory.read_data(sram_addr, 4) {
        println!("    Read data from SRAM: {:02X?}", read_data);
    }

    let (icache_hit, dcache_hit) = mcu_memory.get_cache_statistics();
    println!("  Cache Performance:");
    println!("    I-Cache Hit Rate: {:.2}%", icache_hit * 100.0);
    println!("    D-Cache Hit Rate: {:.2}%", dcache_hit * 100.0);

    println!("\n2. PC/Server Memory Hierarchy:");
    let external_dram = DramController::new(256, 256, 64); // 256x256 DRAMアレイ
    let mut pc_memory = MemoryHierarchy::new_pc_server(
        32 * 1024, // 32KB L1 ICache
        32 * 1024, // 32KB L1 DCache
        64 * 1024, // 64KB Internal SRAM
        external_dram,
    );

    println!("  {}", pc_memory.get_memory_info());

    // PC/サーバーのメモリアクセステスト
    println!("\n  Memory Access Tests:");

    // より多くの命令フェッチ（キャッシュ効果を確認）
    println!("  - Intensive instruction fetch test...");
    for i in 0..100 {
        let addr = 0x8000_0000 + i * 4;
        pc_memory.read_instruction(addr);
    }

    // 同じアドレスに再アクセス（キャッシュヒット確認）
    for i in 0..50 {
        let addr = 0x8000_0000 + i * 4;
        pc_memory.read_instruction(addr);
    }

    let (icache_hit2, dcache_hit2) = pc_memory.get_cache_statistics();
    println!("  Cache Performance after intensive access:");
    println!("    I-Cache Hit Rate: {:.2}%", icache_hit2 * 100.0);
    println!("    D-Cache Hit Rate: {:.2}%", dcache_hit2 * 100.0);

    println!("\n3. Memory Architecture Comparison:");
    println!("  Microcontroller:");
    println!("    - CPU内部に小容量キャッシュ");
    println!("    - 内部SRAMがメインメモリ");
    println!("    - 高速アクセス、低消費電力");
    println!("    - 外部メモリ不要（オールインワン）");

    println!("  PC/Server:");
    println!("    - CPU内部に大容量キャッシュ");
    println!("    - 外部DRAMで大容量");
    println!("    - 独立したDRAMコントローラー");
    println!("    - 異なるクロックドメイン");

    println!("\n4. Clock Domain Analysis:");
    println!("  - CPU内部メモリ（SRAM/キャッシュ）: CPUクロック同期");
    println!("  - 外部DRAM: 独立したメモリクロック");
    println!("  - メモリコントローラー: クロックドメイン間の橋渡し");
}
