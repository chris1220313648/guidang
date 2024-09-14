
cd $project_name
cargo build --release
serve ./config/register
cd python
python3 add_script.py
python3 add_js.py
python3 register.py
python3 ability_mock.py