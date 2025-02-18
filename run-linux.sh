#!/bin/bash
# 
# ██╗  ██╗███████╗ █████╗ ██████╗ ██████╗ ██╗   ██╗██╗     ███████╗███████╗
# ██║  ██║██╔════╝██╔══██╗██╔══██╗██╔══██╗██║   ██║██║     ██╔════╝██╔════╝
# ███████║█████╗  ███████║██████╔╝██████╔╝██║   ██║██║     ███████╗█████╗  
# ██╔══██║██╔══╝  ██╔══██║██╔═══╝ ██╔═══╝ ██║   ██║██║     ╚════██║██╔══╝  
# ██║  ██║███████╗██║  ██║██║     ██║     ╚██████╔╝███████╗███████║███████╗
# ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝      ╚═════╝ ╚══════╝╚══════╝╚══════╝
# 
# ███╗   ███╗███████╗███╗   ███╗ ██████╗ ██████╗ ██╗   ██╗                 
# ████╗ ████║██╔════╝████╗ ████║██╔═══██╗██╔══██╗╚██╗ ██╔╝                 
# ██╔████╔██║█████╗  ██╔████╔██║██║   ██║██████╔╝ ╚████╔╝                  
# ██║╚██╔╝██║██╔══╝  ██║╚██╔╝██║██║   ██║██╔══██╗  ╚██╔╝                   
# ██║ ╚═╝ ██║███████╗██║ ╚═╝ ██║╚██████╔╝██║  ██║   ██║                    
# ╚═╝     ╚═╝╚══════╝╚═╝     ╚═╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝                    
# 
# ██████╗ ██████╗  ██████╗ ███████╗██╗██╗     ███████╗██████╗              
# ██╔══██╗██╔══██╗██╔═══██╗██╔════╝██║██║     ██╔════╝██╔══██╗             
# ██████╔╝██████╔╝██║   ██║█████╗  ██║██║     █████╗  ██████╔╝             
# ██╔═══╝ ██╔══██╗██║   ██║██╔══╝  ██║██║     ██╔══╝  ██╔══██╗             
# ██║     ██║  ██║╚██████╔╝██║     ██║███████╗███████╗██║  ██║             
# ╚═╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝╚══════╝╚═╝  ╚═╝             
# 

if (( $EUID != 0 )); then
    echo "Please run as root"
    exit
fi

PROGRAM_TO_RUN="$1"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -z "$PROGRAM_TO_RUN" ]; then
    echo "Usage: $0 <program to run> <program arguments>"
    exit 1
fi

if [ ! -f "$PROGRAM_TO_RUN" ]; then
    echo "Program $PROGRAM_TO_RUN does not exist❌"
    exit 1
fi

if [ ! -f "$SCRIPT_DIR/libcompress.so" ]; then
    echo "libcompress.so does not exist in $SCRIPT_DIR❌"
    exit 1
fi

shift

# gdb setup:
# gdb -ex "set environment LD_PRELOAD=./libbkmalloc.so" -ex "set environment BKMALLOC_OPTS=--hooks-file=./hook.so"
# COMPRESSION_PATH="/usr/lib/x86_64-linux-gnu/libz.so:/usr/lib/x86_64-linux-gnu/liblz4.so:/usr/lib/x86_64-linux-gnu/liblzo2.so:/usr/lib/x86_64-linux-gnu/liblzf.so:/usr/lib/x86_64-linux-gnu/libsnappy.so:/usr/lib/x86_64-linux-gnu/libzstd.so"
# LD_PRELOAD="$LD_PRELOAD:$SCRIPT_DIR/libcompress.so" "$PROGRAM_TO_RUN" $@

result=$?
if [ $result -ne 0 ]; then
    echo "Program $PROGRAM_TO_RUN failed to terminate successfully❌"
fi

exit $result