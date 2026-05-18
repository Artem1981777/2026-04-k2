use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("--- K2 ULTIMATE AUDIT: DEEP FUZZING START ---");
    let mut seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();

    for i in 1..10000001 { // 10 миллионов итераций за цикл
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let rand = seed;

        // 1. ПРОВЕРКА ДЕПОЗИТОВ (Rounding/Inflation)
        let deposit = (rand % 50_000_000) as u128; // Лимит до 50 млн
        let assets = ((rand >> 2) % 100_000_000) as u128 + 1;
        let supply = ((rand >> 4) % 1_000_000) as u128 + 1;
        
        let shares = (deposit * supply) / assets;
        if deposit > 0 && shares == 0 {
            println!("[!] LOSS: Deposit {} -> 0 shares (Assets: {}, Supply: {})", deposit, assets, supply);
        }

        // 2. ПРОВЕРКА КОМИССИЙ (Precision Loss)
        let fee_rate = 50; // 0.5% (50 bps)
        let fee = (deposit * fee_rate) / 10000;
        if deposit > 0 && fee == 0 {
            println!("[?] FEE_GAP: Zero fee for amount {}", deposit);
        }

        // 3. ПРОВЕРКА ЛИКВИДАЦИЙ (Overflows/Bad Debt)
        let collateral = (rand % 1_000_000_000) as u128;
        let debt = (rand % 800_000_000) as u128;
        let price_drop = (rand % 100) as u128; 
        
        if debt > 0 && price_drop > 80 {
            // Ищем критические ошибки в расчетах при волатильности
            if collateral.checked_mul(price_drop).is_none() {
                println!("[!!!] CRITICAL: Math Overflow in Liquidation Logic!");
            }
        }

        if i % 1000000 == 0 {
            println!("Progress: {}M iterations...", i / 1000000);
        }
    }
    println!("--- CYCLE FINISHED. RESTING ---");
}
