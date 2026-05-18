#[cfg(test)]
mod tests {
    use crate::constants::{calculate_oracle_to_wad_factor, WAD_PRECISION};

    #[test]
    fn test_fuzz_oracle_scaling() {
        println!("\n--- STARTING TARGETED FUZZING ---");
        for precision in 0..100 {
            let factor = calculate_oracle_to_wad_factor(precision);
            
            // Если точность больше 18, а фактор = 1, это баг H-03
            if precision > 18 && factor == 1 {
                println!("!!! [CRITICAL] H-03 FOUND: Precision {} | Factor: {}", precision, factor);
            }
            
            // Логируем прогресс каждые 10 шагов
            if precision % 10 == 0 {
                println!("Status: Testing precision {}...", precision);
            }
        }
        println!("--- FUZZING COMPLETE ---\n");
    }
}
