from flask import Flask, request
import requests
import threading
import time

app = Flask(__name__)

class Ability:
    def __init__(self, name, attribute, property_name, http_url="127.0.0.1:8079"):
        self.name = name
        self.attribute = attribute
        self.property_name = property_name
        self.http_url = http_url
        self.headers = {'Content-Type': 'application/json'}
        self.status = 1 if "isonline" in property_name else 0
        self.regist()

    def regist(self):
        data = {
            "name": self.name,
            "http_url": self.http_url,
            "attributes": {
                self.attribute: {
                    "properties": {self.property_name: self.status}
                }
            }
        }
        res = requests.post(f"http://127.0.0.1:5000/register", headers=self.headers, json=data)

    def get(self):
        return self.status

    def change(self, status):
        self.status = status
        self.update_status()
        return self.status

    def update_status(self):
        data = {
            "name": self.name,
            "http_url": self.http_url,
            "attributes": {
                self.attribute: {
                    "properties": {self.property_name: self.status}
                }
            }
        }
        res = requests.put(f"http://127.0.0.1:5000/update", headers=self.headers, json=data)

def create_abilities(n):
    abilities = {}
    alerts = {}
    for i in range(1, n+1):
        ability_name = f"ability{i}_sensor"
        alert_name = f"ability{i}_executor"
        ability = Ability(ability_name, f"ability{i}", "isonline")
        alert = Ability(alert_name, f"alert_ability{i}", "activate")
        abilities[ability_name] = ability
        alerts[alert_name] = alert
        # 将创建的能力注册到全局变量中
        globals()[f"local_{ability_name}"] = ability
        globals()[f"local_alertforability{i}"] = alert
    return abilities, alerts

@app.route('/ability/get_status/<ability>', methods=['GET'])
def get_ability_status(ability):
    local_ability = globals().get(f"local_{ability}")
    if local_ability:
        return {"status": local_ability.get()}
    return {"error": "Ability not found"}, 404

@app.route('/ability/change_status/<ability>', methods=['POST'])
def change_ability_status(ability):
    local_ability = globals().get(f"local_{ability}")
    if local_ability:
        status = request.json["status"]
        if local_ability.change(status) == status:
            return "ok"
    return {"error": "Ability not found"}, 404

def toggle_ability_status(ability_name):
    while True:
        local_ability = globals().get(f"local_{ability_name}")
        if local_ability:
            new_status = 0 
            local_ability.change(new_status)
            data = {"status": new_status}
            try:
                response = requests.post(f"http://127.0.0.1:8079/ability/change_status/{ability_name}", json=data)
                print(f"Response: {response.status_code} - {response.text}")
            except requests.exceptions.RequestException as e:
                print(f"Request failed: {e}")
        time.sleep(3)

if __name__ == "__main__":
    local_camera = Ability("sensor", "camera", "isonline")
    local_alertforcamera = Ability("executor", "alert", "activate")
    local_plc = Ability("plc_sensor", "plc_1", "isonline")
    local_alertforplc = Ability("plc_executor", "alert_plc", "activate")

    # 批量创建 10 个能力和报警器能力
    abilities, alerts = create_abilities(20)

    # 启动线程来周期性地改变能力状态
    threading.Thread(target=toggle_ability_status, args=("camera",), daemon=True).start()
    threading.Thread(target=toggle_ability_status, args=("plc",), daemon=True).start()

    # 启动新创建的能力的线程
    for ability_name in abilities.keys():
        threading.Thread(target=toggle_ability_status, args=(ability_name,), daemon=True).start()

    app.run(host='0.0.0.0', port=8079)
