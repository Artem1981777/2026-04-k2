#!/bin/bash
termux-wake-lock
echo "Starting 24/7 Fuzzing at $(date)" > findings.log

# Компилируем один раз
rustc talos_fuzzer.rs -O

while true; do
  ./talos_fuzzer >> findings.log 2>&1
  echo "Cycle finished at $(date). Resting..." >> findings.log
  sleep 60 # Отдых 1 минута, чтобы не "жарить" процессор
done
