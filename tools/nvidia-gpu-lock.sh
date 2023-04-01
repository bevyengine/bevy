#!/bin/bash

graphics=$(nvidia-smi --query-supported-clocks=graphics --format=csv | sed -n 2p | tr -d -c 0-9)
memory=$(nvidia-smi --query-supported-clocks=memory --format=csv | sed -n 2p | tr -d -c 0-9)

nvidia-smi --lock-gpu-clocks=$graphics
nvidia-smi --lock-memory-clocks=$memory

export GPU_LOCKED=magic
$@

nvidia-smi --reset-gpu-clocks
nvidia-smi --reset-memory-clocks
