#!/bin/bash

# 跳转到工作目录

cd /home/ai801/code/rule_engine/guidang

# 编译项目

# cargo build --release

# 启动 serve

serve ./config/register &

# 启动 Python 脚本（可选）

# python3 /home/ai801/code/rule_engine/guidang/python/register.py &

# 启动 cloud

./target/release/cloud &

# 启动 deno_executor

./target/release/deno_executor  "http://127.0.0.1:8001" 
