#!/bin/bash

graphics=$(nvidia-smi --query-supported-clocks=graphics --format=csv | sed -n 2p | tr -d -c 0-9)
memory=$(nvidia-smi --query-supported-clocks=memory --format=csv | sed -n 2p | tr -d -c 0-9)

if ! nvidia-smi --lock-gpu-clocks=$graphics; then
    echo -e "\nRerun with administrator privileges."
    exit
fi
nvidia-smi --lock-memory-clocks=$memory

reset() {
    nvidia-smi --reset-gpu-clocks
    nvidia-smi --reset-memory-clocks
}
trap reset EXIT

export GPU_LOCKED=magic
$@
