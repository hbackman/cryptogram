#!/bin/bash

PORTS=("5001" "5002" "5003")

for i in "${!PORTS[@]}"; do
  kitty @ launch \
    --cwd "$(pwd)" \
    --title "Node ${PORTS[$i]}" \
    --hold \
    -- zsh -c "source ~/.zshrc && cargo run -- --port ${PORTS[$i]}"
  sleep 0.5
done
