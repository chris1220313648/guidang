# -*- coding: utf-8 -*-
# 数据格式
{
  
  "name": "sensor", 
  "attributes": {
      "camera": {
          "properties": {"status":0}, 
          "address": "127.0.0.1:8079"}
  }
}
# -*- coding: utf-8 -*-
from flask import Flask, request, jsonify
from datetime import datetime

app = Flask(__name__)

# 存储能力的列表
abilities = {}
# 存储事件的列表
event = {}

def log_event(action, ability_name,ability_attributes):
    """记录事件信息"""
    event[datetime.now().isoformat()] = {
        "event": action,
        "name": ability_name,
        "attributes": ability_attributes
    }
@app.route('/event', methods=['GET'])
def get_event():
    global event  # 声明使用全局的 event 变量
    
    # 返回当前的事件字典
    current_events = event.copy()  # 复制当前事件字典以避免并发修改问题
    
    # 清空事件字典
    event.clear()
    
    return jsonify(current_events), 200
@app.route('/clear_events', methods=['DELETE'])
def clear_events():
    """清空所有事件记录"""
    event.clear()  # 清空事件字典
    return jsonify({"message": "All events have been cleared"}), 200
@app.route('/register', methods=['POST'])
def register_ability():
    # 从请求中获取 JSON 数据
    data = request.get_json()
    ability_name = data.get('name')
    attributes = data.get('attributes')

    if not ability_name or not attributes:
        return jsonify({"error": "Invalid data"}), 400

    # 将能力存储到 abilities 字典中
    abilities[ability_name] = attributes
    # 记录事件
    log_event("create", ability_name,attributes)
    print(abilities)

    return jsonify({"message": "Ability '{}' registered successfully".format(ability_name)}), 201

@app.route('/update', methods=['PUT'])
def update_ability():
    # 获取更新的能力信息
    data = request.get_json()
    ability_name = data.get('name')
    attributes = data.get('attributes')

    if not ability_name or ability_name not in abilities:
        return jsonify({"error": "Ability not found"}), 404

    # 更新能力的属性
    abilities[ability_name] = attributes
    # 记录事件
    log_event("update", ability_name)

    return jsonify({"message": "Ability '{}' updated successfully".format(ability_name)}), 200

@app.route('/query', methods=['GET'])
def query_ability():
    ability_name = request.args.get('name')
    
    if not ability_name or ability_name not in abilities:
        return jsonify({"error": "Ability not found"}), 404

    # 记录事件
    log_event("query", ability_name)

    return jsonify({
        "message": "Ability '{}' queried successfully".format(ability_name),
        "ability": abilities[ability_name]
    }), 200

@app.route('/delete', methods=['DELETE'])
def delete_ability():
    ability_name = request.args.get('name')
    
    if not ability_name or ability_name not in abilities:
        return jsonify({"error": "Ability not found"}), 404

    # 删除指定的能力
    del abilities[ability_name]
    # 记录事件
    log_event("delete", ability_name)

    return jsonify({"message": "Ability '{}' deleted successfully".format(ability_name)}), 200

@app.route('/events', methods=['GET'])
def get_events():
    """获取所有事件记录"""
    return jsonify(event), 200

if __name__ == '__main__':
    print("Start register")
    app.run(debug=True, host='0.0.0.0', port=5000)

