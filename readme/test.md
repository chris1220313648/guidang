"测试大规模能力"
'''
cd $project_name
cargo build --release
serve ./config/register
cd python
python3 add_script.py
python3 add_js.py
python3 register.py
python3 ability_mock.py
'''
“测试正式plc”
'''
cd /home/rule-user/rule_engine/guidang
cargo build --release
serve ./config/register
python3 register.py#后续对接真实log前端就不用启动
./target/release/cloud
./target/release/deno_executor  "http://127.0.0.1:8001" 
'''