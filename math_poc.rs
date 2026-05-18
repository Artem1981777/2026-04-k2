// Копируем константу напрямую из проекта для независимого теста
const WAD_PRECISION: u32 = 18;

pub fn calculate_oracle_to_wad_factor(oracle_precision: u32) -> u128 {
    if oracle_precision >= WAD_PRECISION {
        1
    } else {
        10_u128.pow(WAD_PRECISION - oracle_precision)
    }
}

fn main() {
    println!("--- OFFLINE MATHEMATICAL ANALYSIS START ---");
    let mut bugs_found = 0;

    for precision in 0..40 {
        let factor = calculate_oracle_to_wad_factor(precision);
        
        // Логика бага H-03: если точность > 18, фактор ДОЛЖЕН уменьшать число (быть < 1), 
        // но функция возвращает 1, что сохраняет слишком большое значение.
        if precision > 18 && factor == 1 {
            println!("!!! [H-03] CRITICAL BUG: Precision {} | Factor returns 1 (Should be decreasing!)", precision);
            bugs_found += 1;
        }
        
        if precision % 5 == 0 {
            println!("Checked precision {}... factor: {}", precision, factor);
        }
    }

    if bugs_found > 0 {
        println!("\nRESULT: Found {} instances of scale-mismatch vulnerability.", bugs_found);
    }
    println!("--- ANALYSIS COMPLETE ---");
}
